use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{
        FromRef, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use clap::{Parser, Subcommand};
use retro_db::{CreateRetroInput, DraftCardInput, RetroOverview, RetroRepository, RetroTemplate};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, sync::broadcast};
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
    event_hub: BoardEventHub,
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

impl FromRef<AppState> for BoardEventHub {
    fn from_ref(state: &AppState) -> Self {
        state.event_hub.clone()
    }
}

#[derive(Clone, Default)]
struct BoardEventHub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<BoardEvent>>>>,
}

impl BoardEventHub {
    fn subscribe(&self, retro_id: Uuid) -> broadcast::Receiver<BoardEvent> {
        let sender = self.sender(retro_id);
        let receiver = sender.subscribe();
        let _ = sender.send(BoardEvent::BoardSnapshot { retro_id });
        receiver
    }

    fn publish(&self, event: BoardEvent) {
        let sender = self.sender(event.retro_id());
        let _ = sender.send(event);
    }

    fn sender(&self, retro_id: Uuid) -> broadcast::Sender<BoardEvent> {
        let mut channels = self
            .channels
            .lock()
            .expect("board event hub mutex poisoned");
        channels
            .entry(retro_id)
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(128);
                sender
            })
            .clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BoardEvent {
    BoardSnapshot { retro_id: Uuid },
    CardChanged { retro_id: Uuid },
    ReadyChanged { retro_id: Uuid },
    PhaseChanged { retro_id: Uuid },
}

impl BoardEvent {
    fn retro_id(&self) -> Uuid {
        match self {
            Self::BoardSnapshot { retro_id }
            | Self::CardChanged { retro_id }
            | Self::ReadyChanged { retro_id }
            | Self::PhaseChanged { retro_id } => *retro_id,
        }
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

#[derive(Deserialize)]
struct CreateDraftCardRequest {
    column_id: Uuid,
    body_text: Option<String>,
    gif_url: Option<String>,
    gif_alt_text: Option<String>,
}

#[derive(Deserialize)]
struct UpdateDraftCardRequest {
    body_text: Option<String>,
    gif_url: Option<String>,
    gif_alt_text: Option<String>,
}

#[derive(Deserialize)]
struct GifSearchQuery {
    q: Option<String>,
}

#[derive(Serialize)]
struct GifSearchResponse {
    results: Vec<GifResult>,
    degraded: bool,
}

#[derive(Serialize)]
struct GifResult {
    id: String,
    url: String,
    preview_url: String,
    alt_text: String,
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
        event_hub: BoardEventHub::default(),
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
        .route("/gifs/search", get(search_gifs))
        .route("/retros", get(list_retros).post(create_retro))
        .route("/retros/{retro_id}", get(open_retro))
        .route("/retros/{retro_id}/events", get(board_events))
        .route("/retros/{retro_id}/cards", post(create_draft_card))
        .route(
            "/retros/{retro_id}/cards/{card_id}",
            patch(update_draft_card).delete(delete_draft_card),
        )
        .route("/retros/{retro_id}/ready", post(mark_ready))
        .route("/retros/{retro_id}/reveal", post(reveal_board))
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
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn create_draft_card(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CreateDraftCardRequest>,
) -> Result<(StatusCode, Json<retro_db::CardRecord>), ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let card_body = card_body_payload(request.body_text, request.gif_url, request.gif_alt_text)?;
    let card = repository
        .create_draft_card(DraftCardInput {
            retro_id,
            column_id: request.column_id,
            author_subject: user.subject,
            author_display_name: user.display_name,
            body_text: card_body.body_text,
            gif_url: card_body.gif_url,
            gif_alt_text: card_body.gif_alt_text,
        })
        .await
        .map_err(|error| ApiError::internal(format!("failed to create draft card: {error}")))?;

    event_hub.publish(BoardEvent::CardChanged { retro_id });

    Ok((StatusCode::CREATED, Json(card)))
}

async fn update_draft_card(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateDraftCardRequest>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let card_body = card_body_payload(request.body_text, request.gif_url, request.gif_alt_text)?;
    repository
        .update_draft_card(
            card_id,
            &user.subject,
            card_body.body_text.as_deref(),
            card_body.gif_url.as_deref(),
            card_body.gif_alt_text.as_deref(),
        )
        .await
        .map_err(|error| ApiError::internal(format!("failed to update draft card: {error}")))?
        .map(|card| {
            event_hub.publish(BoardEvent::CardChanged { retro_id });
            Json(card)
        })
        .ok_or_else(|| ApiError::not_found("draft card not found"))
}

async fn delete_draft_card(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    if repository
        .delete_draft_card(card_id, &user.subject)
        .await
        .map_err(|error| ApiError::internal(format!("failed to delete draft card: {error}")))?
    {
        event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("draft card not found"))
    }
}

async fn mark_ready(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .mark_ready(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to mark ready: {error}")))?;
    event_hub.publish(BoardEvent::ReadyChanged { retro_id });
    repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn reveal_board(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .reveal_board(retro_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to reveal board: {error}")))?;
    event_hub.publish(BoardEvent::PhaseChanged { retro_id });
    repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn board_events(
    State(event_hub): State<BoardEventHub>,
    Path(retro_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| board_event_socket(socket, event_hub, retro_id))
}

async fn board_event_socket(mut socket: WebSocket, event_hub: BoardEventHub, retro_id: Uuid) {
    let mut receiver = event_hub.subscribe(retro_id);

    loop {
        let event = match receiver.recv().await {
            Ok(event) => event,
            Err(broadcast::error::RecvError::Lagged(_)) => BoardEvent::BoardSnapshot { retro_id },
            Err(broadcast::error::RecvError::Closed) => break,
        };
        let Ok(payload) = serde_json::to_string(&event) else {
            continue;
        };
        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

async fn search_gifs(Query(query): Query<GifSearchQuery>) -> Json<GifSearchResponse> {
    let provider = FakeGifProvider;
    match provider
        .search(query.q.as_deref().unwrap_or_default())
        .await
    {
        Ok(results) => Json(GifSearchResponse {
            results,
            degraded: false,
        }),
        Err(()) => Json(GifSearchResponse {
            results: Vec::new(),
            degraded: true,
        }),
    }
}

struct CardBodyPayload {
    body_text: Option<String>,
    gif_url: Option<String>,
    gif_alt_text: Option<String>,
}

fn card_body_payload(
    body_text: Option<String>,
    gif_url: Option<String>,
    gif_alt_text: Option<String>,
) -> Result<CardBodyPayload, ApiError> {
    let body_text = optional_non_empty(body_text);
    let gif_url = optional_non_empty(gif_url);
    let gif_alt_text = optional_non_empty(gif_alt_text);

    if body_text.is_none() && gif_url.is_none() {
        return Err(ApiError::bad_request("card requires text or gif"));
    }

    Ok(CardBodyPayload {
        body_text,
        gif_url,
        gif_alt_text,
    })
}

fn optional_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

struct FakeGifProvider;

impl FakeGifProvider {
    async fn search(&self, query: &str) -> Result<Vec<GifResult>, ()> {
        let query = query.trim();
        if query.eq_ignore_ascii_case("fail") {
            return Err(());
        }
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let slug = query
            .chars()
            .filter_map(|character| {
                if character.is_ascii_alphanumeric() {
                    Some(character.to_ascii_lowercase())
                } else if character.is_whitespace() || character == '-' || character == '_' {
                    Some('-')
                } else {
                    None
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_owned();

        Ok((1..=4)
            .map(|index| GifResult {
                id: format!("{slug}-{index}"),
                url: format!("https://media.spillitout.local/{slug}-{index}.gif"),
                preview_url: format!("https://media.spillitout.local/{slug}-{index}.webp"),
                alt_text: format!("{query} GIF {index}"),
            })
            .collect())
    }
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
    async fn board_event_hub_sends_snapshot_then_mutation_events() {
        let hub = BoardEventHub::default();
        let retro_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();

        let mut first = hub.subscribe(retro_id);
        assert_eq!(
            first.recv().await.unwrap(),
            BoardEvent::BoardSnapshot { retro_id }
        );

        hub.publish(BoardEvent::CardChanged { retro_id });
        assert_eq!(
            first.recv().await.unwrap(),
            BoardEvent::CardChanged { retro_id }
        );

        let mut reconnected = hub.subscribe(retro_id);
        assert_eq!(
            reconnected.recv().await.unwrap(),
            BoardEvent::BoardSnapshot { retro_id }
        );
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

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn writing_endpoints_hide_other_drafts_until_reveal(pool: sqlx::PgPool) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header(HEADER_USER_NAME, "Ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Writing API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let retro_id = created["retro"]["id"].as_str().unwrap();
        let column_id = created["columns"][0]["id"].as_str().unwrap();

        for (subject, body) in [("ava", "Ava draft"), ("lee", "Lee private draft")] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/retros/{retro_id}/cards"))
                        .header(HEADER_USER_SUBJECT, subject)
                        .header("content-type", "application/json")
                        .body(Body::from(format!(
                            r#"{{"column_id":"{column_id}","body_text":"{body}"}}"#
                        )))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
        }

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/retros/{retro_id}"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let ava_board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        assert_eq!(
            ava_board["columns"][0]["cards"][0]["body_text"],
            "Ava draft"
        );
        assert_eq!(
            ava_board["columns"][0]["cards"][1]["body_text"],
            Value::Null
        );
        assert_eq!(ava_board["columns"][0]["cards"][1]["hidden"], true);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ready"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/reveal"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let revealed: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(revealed["retro"]["phase"], "discussion");
        assert_eq!(
            revealed["columns"][0]["cards"][1]["body_text"],
            "Lee private draft"
        );
    }

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn gif_endpoints_search_attach_and_degrade_gracefully(pool: sqlx::PgPool) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/gifs/search?q=high%20five")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let search: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(search["degraded"], false);
        assert_eq!(search["results"].as_array().unwrap().len(), 4);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/gifs/search?q=fail")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let failed_search: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(failed_search["degraded"], true);
        assert_eq!(failed_search["results"].as_array().unwrap().len(), 0);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header(HEADER_USER_NAME, "Ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"GIF API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let retro_id = created["retro"]["id"].as_str().unwrap();
        let column_id = created["columns"][0]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/cards"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"column_id":"{column_id}","gif_url":"https://media.spillitout.local/high-five-1.gif","gif_alt_text":"high five"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let gif_card: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(
            gif_card["gif_url"],
            "https://media.spillitout.local/high-five-1.gif"
        );
    }
}
