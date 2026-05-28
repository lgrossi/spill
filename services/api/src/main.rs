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
    AcceptDeckItemRequest, AddGrantRequest, CastVoteRequest, ClusterCardsRequest,
    CreateDeliveryRequest, CreateDraftCardRequest, CreateMeetingNoteRequest, CreateRetroRequest,
    HealthResponse, IngestItemRequest, MoveDraftCardRequest, RemoveGrantRequest, SessionResponse,
    StartAiJobRequest, UpdateActionRequest, UpdateDraftCardRequest,
};
use contracts::RevealBoardRequest;
use error::ApiError;
use events::{BoardEvent, BoardEventHub};
use identity::{AccessModel, CurrentUser, LinkAccessPolicy};
#[cfg(test)]
use identity::{HEADER_USER_NAME, HEADER_USER_SUBJECT};
#[cfg(test)]
use identity::HEADER_USER_EMAIL;
use retro_db::{BoardGrant, RetroOverview, RetroRepository};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{net::TcpListener, sync::broadcast};
use tracing_subscriber::prelude::*;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use uuid::Uuid;

mod contracts;
mod error;
mod events;
mod identity;
mod jobs;
mod media;
mod telemetry;
mod workflow;

#[derive(Parser, Debug)]
#[command(name = "spillio-api")]
#[command(about = "Spill. API service")]
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
    Migrate {},
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
        // DefaultMakeSpan creates spans at DEBUG by default; INFO keeps them
        // visible through the EnvFilter and lets tracing-opentelemetry export them.
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO)))
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
        .route("/retros/{retro_id}/grants", get(list_grants).post(add_grant))
        .route("/retros/{retro_id}/grants/remove", post(remove_grant))
        .route(
            "/retros/{retro_id}/participants/{subject}",
            delete(remove_participant_from_session),
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
    if !user.email.is_empty() {
        let allowed = repository
            .is_board_member(retro_id, &user.email)
            .await
            .map_err(|error| ApiError::internal(format!("failed to check board access: {error}")))?;
        if !allowed {
            return Err(ApiError::forbidden("not a member of this board"));
        }
    }
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
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<StartAiJobRequest>,
) -> Result<Json<retro_db::AiArtifactRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let artifact = job_workflow(repository, event_hub)?
        .start_ai_job(user, retro_id, request)
        .await?;
    Ok(Json(artifact))
}

async fn retry_ai_job(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::AiArtifactRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let artifact = job_workflow(repository, event_hub)?
        .retry_ai_job(user, retro_id, artifact_id)
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
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CreateDeliveryRequest>,
) -> Result<Json<retro_db::DeliveryRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let delivery = job_workflow(repository, event_hub)?
        .create_delivery(user, retro_id, request)
        .await?;
    Ok(Json(delivery))
}

async fn retry_delivery(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, delivery_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::DeliveryRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let delivery = job_workflow(repository, event_hub)?
        .retry_delivery(user, retro_id, delivery_id)
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
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    retro_workflow(repository, event_hub)?
        .remove_cluster_member(user, retro_id, card_id)
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
    body: Option<Json<RevealBoardRequest>>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let force = body.map(|b| b.force).unwrap_or(false);
    if force {
        let repo = configured_repository(repository.clone())?;
        require_host(&repo, retro_id, &user.email).await?;
    }
    retro_workflow(repository, event_hub)?
        .reveal_board(user, retro_id, force)
        .await
        .map(Json)
}

async fn remove_participant_from_session(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, subject)): Path<(Uuid, String)>,
) -> Result<StatusCode, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let is_host = check_is_host(&repository, retro_id, &user.email).await?;
    if is_host {
        if user.subject == subject {
            return Err(ApiError::bad_request(
                "cannot remove yourself from the session",
            ));
        }
    } else if user.subject != subject {
        return Err(ApiError::forbidden(
            "only the host or the participant themselves can leave the session",
        ));
    }
    let removed = repository
        .remove_participant(retro_id, &subject)
        .await
        .map_err(|e| ApiError::internal(format!("failed to remove participant: {e}")))?;
    if !removed {
        return Err(ApiError::not_found("participant not found"));
    }
    event_hub.publish(BoardEvent::ReadyChanged { retro_id });
    Ok(StatusCode::NO_CONTENT)
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
    headers: HeaderMap,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateActionRequest>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    let action = retro_workflow(repository, event_hub)?
        .update_action(user, retro_id, action_id, request)
        .await?;
    Ok(Json(action))
}

async fn confirm_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    set_action_status(repository, event_hub, user, retro_id, action_id, "confirmed").await
}

async fn complete_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    set_action_status(repository, event_hub, user, retro_id, action_id, "done").await
}

async fn reject_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    set_action_status(repository, event_hub, user, retro_id, action_id, "rejected").await
}

async fn propose_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let user = CurrentUser::from_headers(&headers)?;
    set_action_status(repository, event_hub, user, retro_id, action_id, "proposed").await
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
    user: CurrentUser,
    retro_id: Uuid,
    action_id: Uuid,
    status: &'static str,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let action = retro_workflow(repository, event_hub)?
        .set_action_status(user, retro_id, action_id, status)
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

async fn list_grants(
    State(repository): State<Option<RetroRepository>>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<Vec<BoardGrant>>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let grants = repository
        .list_board_grants(retro_id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to list grants: {e}")))?;
    // Any board member may list grants — the frontend uses the response to
    // determine the current user's role and show/hide invite controls.
    let is_member = grants.iter().any(|g| {
        g.principal_email.eq_ignore_ascii_case(&user.email)
    });
    if !user.email.is_empty() && !is_member {
        return Err(ApiError::forbidden("not a member of this board"));
    }
    Ok(Json(grants))
}

async fn add_grant(
    State(repository): State<Option<RetroRepository>>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<AddGrantRequest>,
) -> Result<(StatusCode, Json<BoardGrant>), ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    require_host(&repository, retro_id, &user.email).await?;
    let email = request.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::bad_request("invalid email address"));
    }
        validate_role(&request.role)?;
    if email == user.email.trim().to_lowercase() {
        let grants = repository
            .list_board_grants(retro_id)
            .await
            .map_err(|e| ApiError::internal(format!("failed to fetch grants: {e}")))?;
        let grant = grants
            .into_iter()
            .find(|g| g.principal_email.eq_ignore_ascii_case(&email))
            .ok_or_else(|| ApiError::not_found("own grant not found"))?;
        return Ok((StatusCode::OK, Json(grant)));
    }
    let grant = repository
        .add_board_grant(retro_id, &email, &request.role)
        .await
        .map_err(|e| ApiError::internal(format!("failed to add grant: {e}")))?;
    Ok((StatusCode::CREATED, Json(grant)))
}

async fn remove_grant(
    State(repository): State<Option<RetroRepository>>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<RemoveGrantRequest>,
) -> Result<StatusCode, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    require_host(&repository, retro_id, &user.email).await?;
    let email = request.email.trim().to_lowercase();
    // Prevent host from revoking their own grant.
    if email.eq_ignore_ascii_case(&user.email) {
        return Err(ApiError::bad_request("cannot remove your own host grant"));
    }
    repository
        .remove_board_grant(retro_id, &email)
        .await
        .map_err(|e| ApiError::internal(format!("failed to remove grant: {e}")))?;
    // Feature 1: evict the participant row so they leave the live session immediately.
    // Errors here are best-effort — the grant is already gone.
    let _ = repository
        .remove_participant_by_email(retro_id, &email)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn require_host(
    repository: &RetroRepository,
    retro_id: Uuid,
    email: &str,
) -> Result<(), ApiError> {
    if email.is_empty() {
        return Err(ApiError::unauthorized("email required"));
    }
    let email_lc = email.to_lowercase();
    let retro = repository
        .fetch_retro(retro_id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to fetch retro: {e}")))?
        .ok_or_else(|| ApiError::not_found("retro not found"))?;
    let is_creator = !retro.creator_email.is_empty() && retro.creator_email == email_lc;
    let grants = repository
        .list_board_grants(retro_id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to check host: {e}")))?;
    let grant_role = grants
        .iter()
        .find(|g| g.principal_email.eq_ignore_ascii_case(email))
        .map(|g| g.role.as_str());
    let is_host = grant_role == Some("host");
    if !is_host && !is_creator {
        return Err(ApiError::forbidden("only the board host can manage grants"));
    }
    if is_creator && !is_host {
        repository
            .add_board_grant(retro_id, &email_lc, "host")
            .await
            .map_err(|e| ApiError::internal(format!("failed to repair host grant: {e}")))?;
    }
    Ok(())
}

async fn check_is_host(
    repository: &RetroRepository,
    retro_id: Uuid,
    email: &str,
) -> Result<bool, ApiError> {
    if email.is_empty() {
        return Ok(false);
    }
    let grants = repository
        .list_board_grants(retro_id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to check host: {e}")))?;
    let is_host = grants
        .iter()
        .any(|g| g.principal_email.eq_ignore_ascii_case(email) && g.role == "host");
    let retro = repository
        .fetch_retro(retro_id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to fetch retro: {e}")))?
        .ok_or_else(|| ApiError::not_found("retro not found"))?;
    let is_creator = !retro.creator_email.is_empty()
        && retro.creator_email == email.trim().to_lowercase();
    Ok(is_host || is_creator)
}

fn validate_role(role: &str) -> Result<(), ApiError> {
    if role == "host" || role == "member" {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "role must be \"host\" or \"member\", got: {role}"
        )))
    }
}

fn init_tracer() -> opentelemetry_sdk::trace::Tracer {
    let dd_agent = std::env::var("DD_AGENT_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let service = std::env::var("DD_SERVICE").unwrap_or_else(|_| "spillio-api".into());
    opentelemetry_datadog::new_pipeline()
        .with_service_name(service)
        .with_agent_endpoint(format!("http://{}:8126", dd_agent))
        .with_api_version(opentelemetry_datadog::ApiVersion::Version05)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("failed to initialise Datadog tracer")
}

async fn run_server(addr: SocketAddr) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(PgConnectOptions::new())
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

async fn run_migrations() -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(PgConnectOptions::new())
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
    let tracer = init_tracer();
    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(telemetry::DdMakeWriter(std::io::stdout)),
        )
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spillio_api=info,tower_http=info".into()),
        )
        .init();

    let result = match Cli::parse().command {
        Command::Serve { addr } => run_server(addr).await,
        Command::Migrate {} => run_migrations().await,
    };
    // Flush in-flight spans to the DD agent before the process exits.
    opentelemetry::global::shutdown_tracer_provider();
    result
}

#[cfg(test)]
mod tests;
