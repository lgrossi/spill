use std::net::SocketAddr;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{FromRef, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use clap::{Parser, Subcommand};
use retro_db::{CreateRetroInput, RetroOverview, RetroRepository, RetroTemplate};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

const HEADER_USER_SUBJECT: &str = "x-spillio-user-subject";
const HEADER_USER_NAME: &str = "x-spillio-user-name";

#[derive(Parser, Debug)]
#[command(name = "spillio-api")]
#[command(about = "SpillItOut API service")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve {
        #[arg(long, env = "SPILLIO_API_ADDR", default_value = "127.0.0.1:4000")]
        addr: SocketAddr,
        #[arg(
            long,
            env = "DATABASE_URL",
            default_value = "postgres://spillio:spillio@localhost:5432/spillio"
        )]
        database_url: String,
    },
    Migrate {
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,
    },
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Clone, Default)]
struct AppState {
    access_policy: LinkAccessPolicy,
    repository: Option<RetroRepository>,
}

#[derive(Clone, Default)]
struct LinkAccessPolicy;

impl LinkAccessPolicy {
    fn can_edit_retro_link(&self, retro_id: &str) -> bool {
        !retro_id.trim().is_empty()
    }
}

impl FromRef<AppState> for LinkAccessPolicy {
    fn from_ref(state: &AppState) -> Self {
        state.access_policy.clone()
    }
}

impl FromRef<AppState> for Option<RetroRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repository.clone()
    }
}

#[derive(Serialize)]
struct SessionResponse {
    user: CurrentUser,
    access_model: AccessModel,
}

#[derive(Serialize)]
struct CurrentUser {
    subject: String,
    display_name: String,
}

impl CurrentUser {
    fn from_headers(headers: &HeaderMap) -> Result<Self, ApiError> {
        let subject = required_header(headers, HEADER_USER_SUBJECT)?;
        let display_name =
            optional_header(headers, HEADER_USER_NAME).unwrap_or_else(|| subject.clone());

        Ok(Self {
            subject,
            display_name,
        })
    }
}

#[derive(Serialize)]
struct AccessModel {
    kind: &'static str,
    can_edit_with_link: bool,
}

#[derive(Deserialize)]
struct CreateRetroRequest {
    title: String,
    template: String,
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default = "default_vote_limit")]
    vote_limit: i32,
    #[serde(default = "default_action_discussion_limit")]
    action_discussion_limit: i32,
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request",
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code,
                message: self.message,
            },
        };

        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
fn app() -> Router {
    app_with_state(AppState::default())
}

fn app_with_repository(repository: RetroRepository) -> Router {
    app_with_state(AppState {
        access_policy: LinkAccessPolicy,
        repository: Some(repository),
    })
}

fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api", api_router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

fn api_router() -> Router<AppState> {
    Router::new()
        .route("/session", get(session))
        .route("/retros", get(list_retros).post(create_retro))
        .route("/retros/{retro_id}", get(open_retro))
        .fallback(api_not_found)
}

async fn health() -> Json<HealthResponse> {
    Json(health_response())
}

async fn session(headers: HeaderMap) -> Result<Json<SessionResponse>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    Ok(Json(SessionResponse {
        user,
        access_model: AccessModel {
            kind: "link",
            can_edit_with_link: LinkAccessPolicy.can_edit_retro_link("retro-link"),
        },
    }))
}

async fn list_retros(
    State(repository): State<Option<RetroRepository>>,
) -> Result<Json<RetroOverview>, ApiError> {
    let repository = configured_repository(repository)?;
    repository
        .list_retros()
        .await
        .map(Json)
        .map_err(|error| ApiError::internal(format!("failed to list retros: {error}")))
}

async fn create_retro(
    State(repository): State<Option<RetroRepository>>,
    headers: HeaderMap,
    Json(request): Json<CreateRetroRequest>,
) -> Result<(StatusCode, Json<retro_db::RetroBoard>), ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let template = request.to_template()?;

    let board = repository
        .create_retro(CreateRetroInput {
            title: require_non_empty("title", request.title)?,
            creator_subject: user.subject,
            creator_display_name: user.display_name,
            template,
            vote_limit: require_positive("vote_limit", request.vote_limit)?,
            action_discussion_limit: require_positive(
                "action_discussion_limit",
                request.action_discussion_limit,
            )?,
        })
        .await
        .map_err(|error| ApiError::internal(format!("failed to create retro: {error}")))?;

    Ok((StatusCode::CREATED, Json(board)))
}

async fn open_retro(
    State(repository): State<Option<RetroRepository>>,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    repository
        .fetch_board(retro_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn api_not_found() -> ApiError {
    ApiError::not_found("API route not found")
}

fn health_response() -> HealthResponse {
    HealthResponse {
        status: "ok",
        service: "spillio-api",
    }
}

impl CreateRetroRequest {
    fn to_template(&self) -> Result<RetroTemplate, ApiError> {
        match self.template.as_str() {
            "standard" => Ok(RetroTemplate::Standard),
            "custom" => {
                let columns = self
                    .columns
                    .iter()
                    .map(|column| column.trim())
                    .filter(|column| !column.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();

                if columns.is_empty() {
                    Err(ApiError::bad_request(
                        "custom retros require at least one column",
                    ))
                } else {
                    Ok(RetroTemplate::Custom { columns })
                }
            }
            _ => Err(ApiError::bad_request("template must be standard or custom")),
        }
    }
}

fn configured_repository(repository: Option<RetroRepository>) -> Result<RetroRepository, ApiError> {
    repository.ok_or_else(|| ApiError::internal("retro repository is not configured"))
}

fn require_non_empty(field: &'static str, value: String) -> Result<String, ApiError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err(ApiError::bad_request(format!("{field} cannot be empty")))
    } else {
        Ok(value)
    }
}

fn require_positive(field: &'static str, value: i32) -> Result<i32, ApiError> {
    if value > 0 {
        Ok(value)
    } else {
        Err(ApiError::bad_request(format!("{field} must be positive")))
    }
}

fn default_vote_limit() -> i32 {
    3
}

fn default_action_discussion_limit() -> i32 {
    3
}

fn required_header(headers: &HeaderMap, name: &'static str) -> Result<String, ApiError> {
    optional_header(headers, name)
        .ok_or_else(|| ApiError::unauthorized(format!("missing required header {name}")))
}

fn optional_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

async fn run_server(addr: SocketAddr, database_url: &str) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind API listener on {addr}"))?;

    tracing::info!(%addr, "spillio API listening");

    axum::serve(listener, app_with_repository(RetroRepository::new(pool)))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("API server failed")
}

async fn run_migrations(database_url: &str) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    tracing::info!("database migrations applied");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spillio_api=info,tower_http=info".into()),
        )
        .init();

    match Cli::parse().command {
        Command::Serve { addr, database_url } => run_server(addr, &database_url).await,
        Command::Migrate { database_url } => run_migrations(&database_url).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use serde_json::Value;
    use tower::ServiceExt;

    #[test]
    fn health_response_identifies_the_api() {
        let response = health_response();

        assert_eq!(response.status, "ok");
        assert_eq!(response.service, "spillio-api");
    }

    #[tokio::test]
    async fn session_endpoint_returns_identity_from_platform_headers() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/session")
                    .header(HEADER_USER_SUBJECT, "user-123")
                    .header(HEADER_USER_NAME, "Ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        assert_eq!(body["user"]["subject"], "user-123");
        assert_eq!(body["user"]["display_name"], "Ava");
        assert_eq!(body["access_model"]["kind"], "link");
        assert_eq!(body["access_model"]["can_edit_with_link"], true);
    }

    #[tokio::test]
    async fn session_endpoint_returns_structured_error_without_identity() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        assert_eq!(body["error"]["code"], "unauthorized");
    }

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn retro_endpoints_create_list_and_open_standard_board(pool: sqlx::PgPool) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "user-123")
                    .header(HEADER_USER_NAME, "Ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Sprint 43","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(created["retro"]["phase"], "writing");
        assert_eq!(
            created["columns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|column| column["title"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["Mood", "Went well", "Went wrong", "Actions"]
        );

        let retro_id = created["retro"]["id"].as_str().unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/retros/{retro_id}"))
                    .header(HEADER_USER_SUBJECT, "user-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "user-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let overview: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(overview["active"].as_array().unwrap().len(), 1);
        assert_eq!(overview["completed"].as_array().unwrap().len(), 0);
    }
}
