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
use retro_core::{
    AiArtifactKind, CardBody, DeliveryKind, DomainError, IngestedItemPlacement, IngestionSource,
};
use retro_db::{
    AcceptDeckItemInput, ActionError, CastVoteInput, ClusterCardsInput, ClusterError,
    CreateMeetingNoteInput, CreateRetroInput, DraftCardInput, IngestItemInput, RetroOverview,
    RetroRepository, RetroTemplate, UpdateActionInput, VotingError,
};
use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

mod contracts;
mod error;
mod events;
mod identity;
mod media;

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
            vote_limit: require_non_negative("vote_limit", request.vote_limit)?,
            action_discussion_limit: require_non_negative(
                "action_discussion_limit",
                request.action_discussion_limit,
            )?,
            column_colors: request.column_colors,
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

async fn ingest_item(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<IngestItemRequest>,
) -> Result<(StatusCode, Json<retro_db::IngestedItemRecord>), ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    validate_source(&request.source)?;
    validate_placement(&request)?;
    let item = repository
        .ingest_item(IngestItemInput {
            retro_id,
            subject: user.subject,
            display_name: user.display_name,
            source: request.source,
            placement: request.placement,
            target_column_id: request.target_column_id,
            suggested_text: optional_non_empty(request.suggested_text),
            gif_url: optional_non_empty(request.gif_url),
            idempotency_key: optional_non_empty(request.idempotency_key),
            raw_payload: request.raw_payload,
            source_metadata: request.source_metadata,
        })
        .await
        .map_err(|error| ApiError::internal(format!("failed to ingest item: {error}")))?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok((StatusCode::CREATED, Json(item)))
}

async fn accept_deck_item(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, item_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AcceptDeckItemRequest>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let card = repository
        .accept_deck_item(AcceptDeckItemInput {
            retro_id,
            item_id,
            column_id: request.column_id,
            subject: user.subject,
            display_name: user.display_name,
        })
        .await
        .map_err(|error| ApiError::internal(format!("failed to accept deck item: {error}")))?
        .ok_or_else(|| ApiError::not_found("deck item not found"))?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(card))
}

async fn start_ai_job(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<StartAiJobRequest>,
) -> Result<Json<retro_db::AiArtifactRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    validate_ai_kind(&request.kind)?;
    let artifact = repository
        .create_ai_artifact(
            retro_id,
            &request.kind,
            ai_input_with_requested_failure(&repository, retro_id, &request.kind, request.fail)
                .await?,
        )
        .await
        .map_err(|error| ApiError::internal(format!("failed to create AI job: {error}")))?;
    let artifact = run_fake_ai_job(&repository, artifact, request.fail).await?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(artifact))
}

async fn retry_ai_job(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::AiArtifactRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let artifact = repository
        .retry_ai_artifact(retro_id, artifact_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to retry AI job: {error}")))?
        .ok_or_else(|| ApiError::not_found("AI artifact not found"))?;
    let artifact = run_fake_ai_job(&repository, artifact, false).await?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(artifact))
}

async fn create_meeting_note(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CreateMeetingNoteRequest>,
) -> Result<(StatusCode, Json<retro_db::MeetingNoteRecord>), ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let note = repository
        .create_meeting_note(CreateMeetingNoteInput {
            retro_id,
            author_subject: user.subject,
            author_display_name: user.display_name,
            title: optional_non_empty(request.title).unwrap_or_else(|| "Meeting notes".to_owned()),
            body_text: require_non_empty("body_text", request.body_text)?,
        })
        .await
        .map_err(|error| ApiError::internal(format!("failed to create meeting note: {error}")))?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok((StatusCode::CREATED, Json(note)))
}

async fn create_delivery(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CreateDeliveryRequest>,
) -> Result<Json<retro_db::DeliveryRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    validate_delivery_kind(&request.kind)?;
    let output = match request.kind.as_str() {
        "summary_export" => repository
            .export_summary_payload(retro_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to export summary: {error}")))?,
        "external_action_link" => serde_json::json!({
            "placeholder_url": "https://example.invalid/spillio/action-placeholder",
            "message": "External action delivery integration placeholder"
        }),
        _ => unreachable!(),
    };
    let delivery = repository
        .create_delivery(retro_id, &request.kind, output, request.fail)
        .await
        .map_err(|error| ApiError::internal(format!("failed to create delivery: {error}")))?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(delivery))
}

async fn retry_delivery(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, delivery_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::DeliveryRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let delivery = repository
        .retry_delivery(retro_id, delivery_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to retry delivery: {error}")))?
        .ok_or_else(|| ApiError::not_found("delivery not found"))?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(delivery))
}

async fn run_fake_ai_job(
    repository: &RetroRepository,
    artifact: retro_db::AiArtifactRecord,
    fail: bool,
) -> Result<retro_db::AiArtifactRecord, ApiError> {
    let artifact = repository
        .mark_ai_running(artifact.id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to mark AI job running: {error}")))?
        .ok_or_else(|| ApiError::not_found("AI artifact not found"))?;

    if fail {
        return repository
            .fail_ai_artifact(artifact.id, "fake AI provider failure")
            .await
            .map_err(|error| ApiError::internal(format!("failed to mark AI job failed: {error}")))?
            .ok_or_else(|| ApiError::not_found("AI artifact not found"));
    }

    let output = fake_ai_output(&artifact.kind);
    repository
        .complete_ai_artifact(artifact.id, output)
        .await
        .map_err(|error| ApiError::internal(format!("failed to complete AI job: {error}")))?
        .ok_or_else(|| ApiError::not_found("AI artifact not found"))
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
            request.cluster_details.as_deref(),
        )
        .await
        .map_err(|error| ApiError::internal(format!("failed to update draft card: {error}")))?
        .map(|card| {
            event_hub.publish(BoardEvent::CardChanged { retro_id });
            Json(card)
        })
        .ok_or_else(|| ApiError::not_found("draft card not found"))
}

async fn move_draft_card(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<MoveDraftCardRequest>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .move_draft_card(
            retro_id,
            card_id,
            request.column_id,
            request.before_card_id,
            &user.subject,
        )
        .await
        .map_err(|error| ApiError::internal(format!("failed to move draft card: {error}")))?
        .map(|card| {
            event_hub.publish(BoardEvent::CardChanged { retro_id });
            Json(card)
        })
        .ok_or_else(|| ApiError::not_found("draft card not found"))
}

async fn cluster_cards(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ClusterCardsRequest>,
) -> Result<Json<retro_db::ClusterRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let cluster = repository
        .cluster_cards(ClusterCardsInput {
            retro_id,
            card_id,
            target_card_id: request.target_card_id,
            subject: user.subject,
            display_name: user.display_name,
        })
        .await
        .map_err(cluster_error)?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(cluster))
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

async fn remove_cluster_member(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::CardRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    repository
        .remove_cluster_member(retro_id, card_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to remove cluster member: {error}")))?
        .map(|card| {
            event_hub.publish(BoardEvent::CardChanged { retro_id });
            Json(card)
        })
        .ok_or_else(|| ApiError::not_found("cluster member not found"))
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

async fn unmark_ready(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .unmark_ready(retro_id, &user.subject)
        .await
        .map_err(|error| ApiError::internal(format!("failed to unmark ready: {error}")))?;
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
    let board = repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .ok_or_else(|| ApiError::not_found("retro not found"))?;
    if board.retro.phase == "writing" && board.ready.ready_count < board.ready.participant_count {
        return Err(ApiError::bad_request(
            "everyone must be ready before reveal",
        ));
    }
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

async fn start_voting(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .start_voting(retro_id)
        .await
        .map_err(voting_error)?;
    event_hub.publish(BoardEvent::PhaseChanged { retro_id });
    repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn cast_vote(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
    Json(request): Json<CastVoteRequest>,
) -> Result<Json<retro_db::VotingInfo>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let info = repository
        .cast_vote(CastVoteInput {
            retro_id,
            card_id: request.card_id,
            subject: user.subject,
            display_name: user.display_name,
            count: request.count,
        })
        .await
        .map_err(voting_error)?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(info))
}

async fn remove_vote(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path((retro_id, card_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<retro_db::VotingInfo>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    let info = repository
        .remove_vote(retro_id, card_id, &user.subject, &user.display_name)
        .await
        .map_err(voting_error)?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    Ok(Json(info))
}

async fn cluster_board(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .cluster_board(retro_id)
        .await
        .map_err(cluster_error)?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
    repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn start_action_discussion(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    headers: HeaderMap,
    Path(retro_id): Path<Uuid>,
) -> Result<Json<retro_db::RetroBoard>, ApiError> {
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .start_action_discussion(retro_id)
        .await
        .map_err(action_error)?;
    event_hub.publish(BoardEvent::PhaseChanged { retro_id });
    repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn update_action(
    State(repository): State<Option<RetroRepository>>,
    State(event_hub): State<BoardEventHub>,
    Path((retro_id, action_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateActionRequest>,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let action = repository
        .update_action(UpdateActionInput {
            retro_id,
            action_id,
            title: require_non_empty("title", request.title)?,
            details: request.details,
        })
        .await
        .map_err(|error| ApiError::internal(format!("failed to update action: {error}")))?
        .ok_or_else(|| ApiError::not_found("action not found"))?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
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
    let repository = configured_repository(repository)?;
    let user = CurrentUser::from_headers(&headers)?;
    repository
        .complete_retro(retro_id)
        .await
        .map_err(action_error)?
        .ok_or_else(|| ApiError::bad_request("retro must be in action discussion to complete"))?;
    event_hub.publish(BoardEvent::PhaseChanged { retro_id });
    repository
        .fetch_board_for_user(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("retro not found"))
}

async fn set_action_status(
    repository: Option<RetroRepository>,
    event_hub: BoardEventHub,
    retro_id: Uuid,
    action_id: Uuid,
    status: &'static str,
) -> Result<Json<retro_db::ActionItemRecord>, ApiError> {
    let repository = configured_repository(repository)?;
    let action = repository
        .set_action_status(retro_id, action_id, status)
        .await
        .map_err(|error| ApiError::internal(format!("failed to update action status: {error}")))?
        .ok_or_else(|| ApiError::not_found("action not found"))?;
    event_hub.publish(BoardEvent::CardChanged { retro_id });
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
    let body = CardBody::from_payload(body_text, gif_url, gif_alt_text).map_err(domain_error)?;

    Ok(CardBodyPayload {
        body_text: body.text().map(ToOwned::to_owned),
        gif_url: body.gif_url().map(ToOwned::to_owned),
        gif_alt_text: body.gif_alt_text().map(ToOwned::to_owned),
    })
}

fn validate_source(source: &str) -> Result<(), ApiError> {
    IngestionSource::try_from(source)
        .map(|_| ())
        .map_err(domain_error)
}

fn validate_placement(request: &IngestItemRequest) -> Result<(), ApiError> {
    match IngestedItemPlacement::try_from(request.placement.as_str()).map_err(domain_error)? {
        IngestedItemPlacement::UserDeck => Ok(()),
        IngestedItemPlacement::RetroDraft if request.target_column_id.is_some() => Ok(()),
        IngestedItemPlacement::RetroDraft => Err(ApiError::bad_request(
            "retro_draft placement requires target_column_id",
        )),
    }?;

    CardBody::from_payload(
        optional_non_empty(request.suggested_text.clone()),
        optional_non_empty(request.gif_url.clone()),
        None,
    )
    .map_err(domain_error)?;

    Ok(())
}

fn validate_ai_kind(kind: &str) -> Result<(), ApiError> {
    AiArtifactKind::try_from(kind)
        .map(|_| ())
        .map_err(domain_error)
}

fn validate_delivery_kind(kind: &str) -> Result<(), ApiError> {
    DeliveryKind::try_from(kind)
        .map(|_| ())
        .map_err(domain_error)
}

async fn ai_input_with_requested_failure(
    repository: &RetroRepository,
    retro_id: Uuid,
    kind: &str,
    fail: bool,
) -> Result<serde_json::Value, ApiError> {
    let mut input = repository
        .ai_input_with_note_context(retro_id, kind)
        .await
        .map_err(|error| ApiError::internal(format!("failed to build AI input: {error}")))?;
    input["requested_failure"] = serde_json::json!(fail);
    Ok(input)
}

fn fake_ai_output(kind: &str) -> serde_json::Value {
    match kind {
        "gif_suggestions" => serde_json::json!({
            "review_required": true,
            "suggestions": [{"query": "ship it", "reason": "celebrate a positive moment"}]
        }),
        "clustering" => serde_json::json!({
            "review_required": true,
            "clusters": [{"title": "Release flow", "tags": ["release", "flow"]}]
        }),
        "action_suggestions" => serde_json::json!({
            "review_required": true,
            "actions": [{"title": "Assign one owner for follow-up", "confidence": "fake"}]
        }),
        "summary" => serde_json::json!({
            "review_required": true,
            "summary": "Fake provider summary ready for human review."
        }),
        "mood" => serde_json::json!({
            "review_required": true,
            "mood": "mixed",
            "signals": ["optimistic", "blocked"]
        }),
        "tagging" => serde_json::json!({
            "review_required": true,
            "tags": ["process", "ownership", "follow-up"]
        }),
        _ => serde_json::json!({"review_required": true}),
    }
}

fn optional_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
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

fn require_non_negative(field: &'static str, value: i32) -> Result<i32, ApiError> {
    if value >= 0 {
        Ok(value)
    } else {
        Err(ApiError::bad_request(format!(
            "{field} must be zero or positive"
        )))
    }
}

fn voting_error(error: VotingError) -> ApiError {
    match error {
        VotingError::Sqlx(error) => ApiError::internal(format!("voting failed: {error}")),
        VotingError::Invalid(message) => ApiError::bad_request(message),
    }
}

fn cluster_error(error: ClusterError) -> ApiError {
    match error {
        ClusterError::Sqlx(error) => ApiError::internal(format!("clustering failed: {error}")),
        ClusterError::Invalid(message) => ApiError::bad_request(message),
    }
}

fn action_error(error: ActionError) -> ApiError {
    match error {
        ActionError::Sqlx(error) => {
            ApiError::internal(format!("action discussion failed: {error}"))
        }
        ActionError::Invalid(message) => ApiError::bad_request(message),
    }
}

fn domain_error(error: DomainError) -> ApiError {
    match error {
        DomainError::EmptyCardBody => ApiError::bad_request("card requires text or gif"),
        DomainError::EmptyText => ApiError::bad_request("text cannot be empty"),
        DomainError::InvalidDomainValue { domain, value } => {
            ApiError::bad_request(format!("invalid {domain}: {value}"))
        }
        other => ApiError::bad_request(format!("domain validation failed: {other:?}")),
    }
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
            ["How are you feeling?", "Went well", "To improve", "Actions"]
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
            .clone()
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
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ready"))
                    .header(HEADER_USER_SUBJECT, "lee")
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
        if search["degraded"].as_bool().unwrap() {
            assert_eq!(search["results"].as_array().unwrap().len(), 0);
        } else {
            assert_eq!(search["results"].as_array().unwrap().len(), 8);
            assert!(matches!(
                search["results"][0]["media_type"].as_str(),
                Some("image" | "video")
            ));
            assert!(
                search["results"][0]["url"]
                    .as_str()
                    .unwrap()
                    .starts_with("http")
            );
        }

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/gifs/search?q=high%20five&page=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let page_two: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        if page_two["degraded"].as_bool().unwrap() {
            assert_eq!(page_two["results"].as_array().unwrap().len(), 0);
        } else {
            assert_ne!(page_two["results"][0]["url"], search["results"][0]["url"]);
        }

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/gifs/search?q=confused&page=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let other_query: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        if other_query["degraded"].as_bool().unwrap() {
            assert_eq!(other_query["results"].as_array().unwrap().len(), 0);
        } else {
            assert_ne!(
                other_query["results"][0]["url"],
                search["results"][0]["url"]
            );
        }

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
                        r#"{{"column_id":"{column_id}","gif_url":"https://media.giphy.com/media/111ebonMs90YLu/giphy.gif","gif_alt_text":"high five"}}"#
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
            "https://media.giphy.com/media/111ebonMs90YLu/giphy.gif"
        );
    }

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn voting_endpoints_track_remaining_votes_and_limits(pool: sqlx::PgPool) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Voting API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
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
                        r#"{{"column_id":"{column_id}","body_text":"vote here"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        let card: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let card_id = card["id"].as_str().unwrap();

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

        for path in ["reveal", "voting/start"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/retros/{retro_id}/{path}"))
                        .header(HEADER_USER_SUBJECT, "ava")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/votes"))
                    .header(HEADER_USER_SUBJECT, "lee")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"card_id":"{card_id}","count":2}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let voting: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(voting["votes_remaining"], 1);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/votes"))
                    .header(HEADER_USER_SUBJECT, "lee")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"card_id":"{card_id}","count":2}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ready"))
                    .header(HEADER_USER_SUBJECT, "lee")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(board["ready"]["current_user_ready"], true);
        assert_eq!(board["voting"]["votes_remaining"], 1);
        assert_eq!(board["columns"][0]["cards"][0]["vote_count"], 2);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/actions/start"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let action_board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(action_board["retro"]["phase"], "action_discussion");
        let action_id = action_board["actions"][0]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/retros/{retro_id}/actions/{action_id}/confirm"
                    ))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/complete"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let completed_board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(completed_board["retro"]["phase"], "completed");
    }

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn ingestion_endpoints_support_deck_and_direct_draft_modes(pool: sqlx::PgPool) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Ingestion API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let retro_id = created["retro"]["id"].as_str().unwrap();
        let first_column_id = created["columns"][0]["id"].as_str().unwrap();
        let second_column_id = created["columns"][1]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ingest"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"source":"pi","placement":"user_deck","suggested_text":"Deck idea","idempotency_key":"event-1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let deck_item: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let deck_item_id = deck_item["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/deck/{deck_item_id}/accept"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"column_id":"{first_column_id}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ingest"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"source":"claude_code","placement":"retro_draft","target_column_id":"{second_column_id}","suggested_text":"Direct idea","idempotency_key":"event-2"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/retros/{retro_id}"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(board["deck"].as_array().unwrap().len(), 0);
        assert_eq!(board["columns"][0]["cards"][0]["body_text"], "Deck idea");
        assert_eq!(board["columns"][1]["cards"][0]["body_text"], "Direct idea");
    }

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn ai_job_endpoints_persist_reviewable_outputs_and_retry_failure(pool: sqlx::PgPool) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"AI API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let retro_id = created["retro"]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"kind":"summary"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let summary: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(summary["status"], "succeeded");
        assert_eq!(summary["output"]["review_required"], true);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"kind":"mood","fail":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let failed: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(failed["status"], "failed");
        let artifact_id = failed["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/retros/{retro_id}/ai-jobs/{artifact_id}/retry"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let retried: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(retried["status"], "succeeded");
        assert_eq!(retried["retry_count"], 1);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/retros/{retro_id}"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(board["ai_artifacts"].as_array().unwrap().len(), 2);
    }

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn meeting_notes_feed_summary_and_mood_ai_context_without_blocking_completion(
        pool: sqlx::PgPool,
    ) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Notes API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let retro_id = created["retro"]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"kind":"summary"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let no_notes_ai: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(no_notes_ai["input"]["meeting_notes_included"], false);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/meeting-notes"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Retro notes","body_text":"Release ownership was unclear."}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"kind":"mood"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let mood_ai: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(mood_ai["input"]["meeting_notes_included"], true);
        assert_eq!(
            mood_ai["input"]["meeting_notes"][0]["body_text"],
            "Release ownership was unclear."
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/retros/{retro_id}"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(board["meeting_notes"].as_array().unwrap().len(), 1);
    }

    #[sqlx::test(migrator = "retro_db::MIGRATOR")]
    async fn delivery_endpoints_export_summary_and_retry_failure(pool: sqlx::PgPool) {
        let app = app_with_repository(retro_db::RetroRepository::new(pool));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Delivery API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let retro_id = created["retro"]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/deliveries"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"kind":"summary_export","fail":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let failed: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(failed["status"], "failed");
        assert_eq!(failed["output"]["title"], "Delivery API retro");
        let delivery_id = failed["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/retros/{retro_id}/deliveries/{delivery_id}/retry"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let retried: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(retried["status"], "succeeded");
        assert_eq!(retried["retry_count"], 1);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/retros/{retro_id}"))
                    .header(HEADER_USER_SUBJECT, "ava")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let board: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(board["deliveries"].as_array().unwrap().len(), 1);
    }
}
