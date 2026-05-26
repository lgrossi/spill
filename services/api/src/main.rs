use std::net::SocketAddr;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{
        FromRef, Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use clap::{Parser, Subcommand};
use contracts::{
    AcceptDeckItemRequest, CastVoteRequest, ClusterCardsRequest, CreateDeliveryRequest,
    CreateDraftCardRequest, CreateMeetingNoteRequest, CreateRetroRequest, HealthResponse,
    IngestItemRequest, MoveDraftCardRequest, SessionResponse, StartAiJobRequest,
    UpdateActionRequest, UpdateDraftCardRequest,
};
use error::ApiError;
use events::{BoardEvent, BoardEventHub};
use identity::{AccessModel, CurrentUser, LinkAccessPolicy};
#[cfg(test)]
use identity::{HEADER_USER_NAME, HEADER_USER_SUBJECT};
use retro_db::{RetroOverview, RetroRepository};
use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

mod contracts;
mod error;
mod events;
mod identity;
mod jobs;
mod media;
mod workflow;

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

#[derive(Clone, Default)]
struct AppState {
    access_policy: LinkAccessPolicy,
    repository: Option<RetroRepository>,
    event_hub: BoardEventHub,
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
        .route("/gifs/search", get(media::search_gifs))
        .route("/retros", get(list_retros).post(create_retro))
        .route("/retros/{retro_id}", get(open_retro))
        .route("/retros/{retro_id}/events", get(board_events))
        .route("/retros/{retro_id}/cards", post(create_draft_card))
        .route(
            "/retros/{retro_id}/cards/{card_id}",
            patch(update_draft_card).delete(delete_draft_card),
        )
        .route(
            "/retros/{retro_id}/cards/{card_id}/move",
            patch(move_draft_card),
        )
        .route(
            "/retros/{retro_id}/cards/{card_id}/cluster",
            patch(cluster_cards),
        )
        .route(
            "/retros/{retro_id}/cards/{card_id}/cluster-member",
            delete(remove_cluster_member),
        )
        .route(
            "/retros/{retro_id}/ready",
            post(mark_ready).delete(unmark_ready),
        )
        .route("/retros/{retro_id}/reveal", post(reveal_board))
        .route("/retros/{retro_id}/voting/start", post(start_voting))
        .route("/retros/{retro_id}/votes", post(cast_vote))
        .route("/retros/{retro_id}/votes/{card_id}", delete(remove_vote))
        .route("/retros/{retro_id}/cluster", post(cluster_board))
        .route(
            "/retros/{retro_id}/actions/start",
            post(start_action_discussion),
        )
        .route(
            "/retros/{retro_id}/actions/{action_id}",
            patch(update_action),
        )
        .route(
            "/retros/{retro_id}/actions/{action_id}/confirm",
            post(confirm_action),
        )
        .route(
            "/retros/{retro_id}/actions/{action_id}/done",
            post(complete_action),
        )
        .route(
            "/retros/{retro_id}/actions/{action_id}/reject",
            post(reject_action),
        )
        .route(
            "/retros/{retro_id}/actions/{action_id}/propose",
            post(propose_action),
        )
        .route("/retros/{retro_id}/complete", post(complete_retro))
        .route("/retros/{retro_id}/ingest", post(ingest_item))
        .route(
            "/retros/{retro_id}/deck/{item_id}/accept",
            post(accept_deck_item),
        )
        .route("/retros/{retro_id}/ai-jobs", post(start_ai_job))
        .route(
            "/retros/{retro_id}/ai-jobs/{artifact_id}/retry",
            post(retry_ai_job),
        )
        .route(
            "/retros/{retro_id}/meeting-notes",
            post(create_meeting_note),
        )
        .route("/retros/{retro_id}/deliveries", post(create_delivery))
        .route(
            "/retros/{retro_id}/deliveries/{delivery_id}/retry",
            post(retry_delivery),
        )
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
    headers: HeaderMap,
) -> Result<Json<RetroOverview>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .list_retros(&user.subject)
        .await
        .map(Json)
        .map_err(|error| ApiError::internal(format!("failed to list retros: {error}")))
}

async fn create_retro(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Json(request): Json<CreateRetroRequest>,
) -> Result<(StatusCode, Json<retro_db::RetroBoard>), ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let (status, board) = retro_workflow(repository, event_hub)?
        .create_retro(user, request)
        .await?;

    Ok((status, Json(board)))
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
    let user = CurrentUser::from_headers(&headers)?;
    let (status, card) = retro_workflow(repository, event_hub)?
        .create_draft_card(user, retro_id, request)
        .await?;
    Ok((status, Json(card)))
}

async fn ingest_item(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<IngestItemRequest>,
) -> Result<(StatusCode, Json<retro_db::IngestedItemRecord>), ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let (status, item) = retro_workflow(repository, event_hub)?
        .ingest_item(user, retro_id, request)
        .await?;
    Ok((status, Json(item)))
}

async fn accept_deck_item(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, item_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AcceptDeckItemRequest>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let card = retro_workflow(repository, event_hub)?
        .accept_deck_item(user, retro_id, item_id, request)
        .await?;
    Ok(Json(card))
}

async fn start_ai_job(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<StartAiJobRequest>,
) -> Result<Json<retro_db::AiArtifactRecord>, ApiError> {
    let artifact = job_workflow(repository, event_hub)?
        .start_ai_job(retro_id, request)
        .await?;
    Ok(Json(artifact))
}

async fn retry_ai_job(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::AiArtifactRecord>, ApiError> {
    let artifact = job_workflow(repository, event_hub)?
        .retry_ai_job(retro_id, artifact_id)
        .await?;
    Ok(Json(artifact))
}

async fn create_meeting_note(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CreateMeetingNoteRequest>,
) -> Result<(StatusCode, Json<retro_db::MeetingNoteRecord>), ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let note = job_workflow(repository, event_hub)?
        .create_meeting_note(user, retro_id, request)
        .await?;
    Ok((StatusCode::CREATED, Json(note)))
}

async fn create_delivery(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CreateDeliveryRequest>,
) -> Result<Json<retro_db::DeliveryRecord>, ApiError> {
    let delivery = job_workflow(repository, event_hub)?
        .create_delivery(retro_id, request)
        .await?;
    Ok(Json(delivery))
}

async fn retry_delivery(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, delivery_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::DeliveryRecord>, ApiError> {
    let delivery = job_workflow(repository, event_hub)?
        .retry_delivery(retro_id, delivery_id)
        .await?;
    Ok(Json(delivery))
}

async fn update_draft_card(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateDraftCardRequest>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .update_draft_card(user, retro_id, card_id, request)
        .await
        .map(Json)
}

async fn move_draft_card(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<MoveDraftCardRequest>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .move_draft_card(user, retro_id, card_id, request)
        .await
        .map(Json)
}

async fn cluster_cards(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ClusterCardsRequest>,
) -> Result<Json<retro_db::ClusterRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let cluster = retro_workflow(repository, event_hub)?
        .cluster_cards(user, retro_id, card_id, request)
        .await?;
    Ok(Json(cluster))
}

async fn delete_draft_card(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .delete_draft_card(user, retro_id, card_id)
        .await
}

async fn remove_cluster_member(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    retro_workflow(repository, event_hub)?
        .remove_cluster_member(retro_id, card_id)
        .await
        .map(Json)
}

async fn mark_ready(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .mark_ready(user, retro_id)
        .await
        .map(Json)
}

async fn unmark_ready(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .unmark_ready(user, retro_id)
        .await
        .map(Json)
}

async fn reveal_board(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .reveal_board(user, retro_id)
        .await
        .map(Json)
}

async fn start_voting(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .start_voting(user, retro_id)
        .await
        .map(Json)
}

async fn cast_vote(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CastVoteRequest>,
) -> Result<Json<retro_db::VotingInfo>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let info = retro_workflow(repository, event_hub)?
        .cast_vote(user, retro_id, request)
        .await?;
    Ok(Json(info))
}

async fn remove_vote(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::VotingInfo>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let info = retro_workflow(repository, event_hub)?
        .remove_vote(user, retro_id, card_id)
        .await?;
    Ok(Json(info))
}

async fn cluster_board(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .cluster_board(user, retro_id)
        .await
        .map(Json)
}

async fn start_action_discussion(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .start_action_discussion(user, retro_id)
        .await
        .map(Json)
}

async fn update_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateActionRequest>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let action = retro_workflow(repository, event_hub)?
        .update_action(retro_id, action_id, request)
        .await?;
    Ok(Json(action))
}

async fn confirm_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    set_action_status(repository, event_hub, retro_id, action_id, "confirmed").await
}

async fn complete_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    set_action_status(repository, event_hub, retro_id, action_id, "done").await
}

async fn reject_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    set_action_status(repository, event_hub, retro_id, action_id, "rejected").await
}

async fn propose_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    set_action_status(repository, event_hub, retro_id, action_id, "proposed").await
}

async fn complete_retro(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .complete_retro(user, retro_id)
        .await
        .map(Json)
}

async fn set_action_status(
    repository: Option<RetroRepository>,
    event_hub: BoardEventHub,
    retro_id: Uuid,
    action_id: Uuid,
    status: &'static str,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let action = retro_workflow(repository, event_hub)?
        .set_action_status(retro_id, action_id, status)
        .await?;
    Ok(Json(action))
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

async fn api_not_found() -> ApiError {
    ApiError::not_found("API route not found")
}

fn health_response() -> HealthResponse {
    HealthResponse {
        status: "ok",
        service: "spillio-api",
    }
}

fn configured_repository(repository: Option<RetroRepository>) -> Result<RetroRepository, ApiError> {
    repository.ok_or_else(|| ApiError::internal("retro repository is not configured"))
}

fn retro_workflow(
    repository: Option<RetroRepository>,
    event_hub: BoardEventHub,
) -> Result<workflow::RetroWorkflow, ApiError> {
    Ok(workflow::RetroWorkflow::new(
        configured_repository(repository)?,
        event_hub,
    ))
}

fn job_workflow(
    repository: Option<RetroRepository>,
    event_hub: BoardEventHub,
) -> Result<jobs::JobWorkflow, ApiError> {
    Ok(jobs::JobWorkflow::new(
        configured_repository(repository)?,
        event_hub,
    ))
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
mod tests;
