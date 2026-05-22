use std::net::SocketAddr;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::FromRef,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

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
        let display_name = optional_header(headers, HEADER_USER_NAME).unwrap_or_else(|| subject.clone());

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

fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api", api_router())
        .with_state(AppState::default())
        .layer(TraceLayer::new_for_http())
}

fn api_router() -> Router<AppState> {
    Router::new()
        .route("/session", get(session))
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

async fn api_not_found() -> ApiError {
    ApiError::not_found("API route not found")
}

fn health_response() -> HealthResponse {
    HealthResponse {
        status: "ok",
        service: "spillio-api",
    }
}

fn required_header(headers: &HeaderMap, name: &'static str) -> Result<String, ApiError> {
    optional_header(headers, name).ok_or_else(|| ApiError::unauthorized(format!("missing required header {name}")))
}

fn optional_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

async fn run_server(addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind API listener on {addr}"))?;

    tracing::info!(%addr, "spillio API listening");

    axum::serve(listener, app())
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
        Command::Serve { addr } => run_server(addr).await,
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
        let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();

        assert_eq!(body["user"]["subject"], "user-123");
        assert_eq!(body["user"]["display_name"], "Ava");
        assert_eq!(body["access_model"]["kind"], "link");
        assert_eq!(body["access_model"]["can_edit_with_link"], true);
    }

    #[tokio::test]
    async fn session_endpoint_returns_structured_error_without_identity() {
        let response = app()
            .oneshot(Request::builder().uri("/api/session").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();

        assert_eq!(body["error"]["code"], "unauthorized");
    }
}
