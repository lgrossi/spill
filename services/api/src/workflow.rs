use std::sync::Arc;

use axum::http::StatusCode;
use retro_core::{CardBody, DomainError, IngestedItemPlacement, IngestionSource};
use retro_db::{
    AcceptDeckItemInput, CastVoteInput, ClusterCardsInput, ClusterError, CreateRetroInput,
    DraftCardInput, IngestItemInput, RetroRepository, RetroTemplate, UpdateActionInput,
    UpdateRetroDetailsInput, VotingError,
};
use uuid::Uuid;

use crate::{
    ai_provider::AiProvider,
    ai_summary,
    contracts::{
        AcceptDeckItemRequest, CastVoteRequest, ClusterCardsRequest, CreateDraftCardRequest,
        CreateRetroRequest, IngestItemRequest, MoveDraftCardRequest, RescheduleRetroRequest,
        UpdateActionRequest, UpdateDraftCardRequest, UpdateRetroDetailsRequest,
    },
    error::ApiError,
    events::{BoardEvent, BoardEventHub},
    identity::CurrentUser,
};

#[derive(Clone)]
pub struct RetroWorkflow {
    repository: RetroRepository,
    event_hub: BoardEventHub,
    /// Optional, opt-in: only `complete_retro` consults this to
    /// auto-trigger the summary AI artifact. None means no auto-trigger;
    /// every other workflow method is unaffected.
    ai_provider: Option<Arc<AiProvider>>,
}

impl RetroWorkflow {
    pub fn new(repository: RetroRepository, event_hub: BoardEventHub) -> Self {
        Self {
            repository,
            event_hub,
            ai_provider: None,
        }
    }

    pub fn with_ai_provider(mut self, ai_provider: Option<Arc<AiProvider>>) -> Self {
        self.ai_provider = ai_provider;
        self
    }

    pub async fn create_retro(
        &self,
        user: CurrentUser,
        request: CreateRetroRequest,
    ) -> Result<(StatusCode, retro_db::RetroBoard), ApiError> {
        let invitees = request.invitees;
        let creator_email_lc = user.email.to_lowercase();
        let planned_for = match optional_non_empty(request.planned_for) {
            Some(value) => Some(require_date_only("planned_for", value)?),
            None => None,
        };
        let board = self
            .repository
            .create_retro(CreateRetroInput {
                title: require_non_empty("title", request.title)?,
                creator_subject: user.subject,
                creator_email: user.email,
                creator_display_name: user.display_name,
                group_name: optional_non_empty(request.group_name),
                planned_for,
                template: retro_template(&request.template, request.columns)?,
                vote_limit: require_non_negative("vote_limit", request.vote_limit)?,
                action_discussion_limit: require_non_negative(
                    "action_discussion_limit",
                    request.action_discussion_limit,
                )?,
                column_colors: request.column_colors,
            })
            .await
            .map_err(|error| ApiError::internal(format!("failed to create retro: {error}")))?;

        let retro_id = board.retro.id;
        for invitee in invitees {
            let email = invitee.email.trim().to_lowercase();
            if email.is_empty() || !email.contains('@') || email == creator_email_lc {
                continue;
            }
            if invitee.role != "host" && invitee.role != "member" {
                return Err(ApiError::bad_request(format!(
                    "role must be \"host\" or \"member\", got: {}",
                    invitee.role
                )));
            }
            self.repository
                .add_board_grant(retro_id, &email, &invitee.role)
                .await
                .map_err(|e| ApiError::internal(format!("failed to add invitee grant: {e}")))?;
        }

        Ok((StatusCode::CREATED, board))
    }

    pub async fn create_draft_card(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: CreateDraftCardRequest,
    ) -> Result<(StatusCode, retro_db::CardRecord), ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let retro = self
            .repository
            .fetch_retro(retro_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
            .ok_or_else(|| ApiError::not_found("retro not found"))?;
        if retro.phase == "scheduled" || retro.phase == "completed" {
            return Err(ApiError::bad_request(
                "cards can only be created after the retro has started",
            ));
        }
        let card_body =
            card_body_payload(request.body_text, request.gif_url, request.gif_alt_text)?;
        let card = self
            .repository
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

        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok((StatusCode::CREATED, card))
    }

    pub async fn reschedule_retro(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: RescheduleRetroRequest,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let planned_for = require_date_only(
            "planned_for",
            require_non_empty(
                "planned_for",
                request
                    .planned_for
                    .ok_or_else(|| ApiError::bad_request("planned_for is required"))?,
            )?,
        )?;
        if let Some(retro) = self
            .repository
            .reschedule_scheduled_retro(retro_id, &planned_for)
            .await
            .map_err(|error| ApiError::internal(format!("failed to reschedule retro: {error}")))?
        {
            self.event_hub
                .publish(BoardEvent::PhaseChanged { retro_id: retro.id });
            return self.fetch_board_for_user(retro_id, &user).await;
        }

        let retro = self
            .repository
            .fetch_retro(retro_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
            .ok_or_else(|| ApiError::not_found("retro not found"))?;
        Err(ApiError::bad_request(format!(
            "planned_for can only be rescheduled while scheduled, current phase is {}",
            retro.phase
        )))
    }

    pub async fn update_retro_details(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: UpdateRetroDetailsRequest,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        ensure_retro_host(&self.repository, retro_id, &user.email).await?;
        let title = match request.title {
            Some(value) => Some(require_non_empty("title", value)?),
            None => None,
        };
        let group_name = match request.group_name {
            Some(value) => Some(require_non_empty("group_name", value)?),
            None => None,
        };
        if title.is_none() && group_name.is_none() {
            return Err(ApiError::bad_request("title or group_name is required"));
        }

        self.repository
            .update_retro_details(UpdateRetroDetailsInput {
                retro_id,
                title,
                group_name,
            })
            .await
            .map_err(|error| {
                ApiError::internal(format!("failed to update retro details: {error}"))
            })?
            .ok_or_else(|| ApiError::not_found("retro not found"))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn ingest_item(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: IngestItemRequest,
    ) -> Result<(StatusCode, retro_db::IngestedItemRecord), ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        validate_source(&request.source)?;
        validate_placement(&request)?;
        let item = self
            .repository
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
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok((StatusCode::CREATED, item))
    }

    pub async fn accept_deck_item(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        item_id: Uuid,
        request: AcceptDeckItemRequest,
    ) -> Result<retro_db::CardRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let card = self
            .repository
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
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(card)
    }

    pub async fn update_draft_card(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        card_id: Uuid,
        request: UpdateDraftCardRequest,
    ) -> Result<retro_db::CardRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let card_body =
            card_body_payload(request.body_text, request.gif_url, request.gif_alt_text)?;
        let card = self
            .repository
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
            .ok_or_else(|| ApiError::not_found("draft card not found"))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(card)
    }

    pub async fn move_draft_card(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        card_id: Uuid,
        request: MoveDraftCardRequest,
    ) -> Result<retro_db::CardRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let card = self
            .repository
            .move_draft_card(
                retro_id,
                card_id,
                request.column_id,
                request.before_card_id,
                &user.subject,
            )
            .await
            .map_err(|error| ApiError::internal(format!("failed to move draft card: {error}")))?
            .ok_or_else(|| ApiError::not_found("draft card not found"))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(card)
    }

    pub async fn cluster_cards(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        card_id: Uuid,
        request: ClusterCardsRequest,
    ) -> Result<retro_db::ClusterRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let cluster = self
            .repository
            .cluster_cards(ClusterCardsInput {
                retro_id,
                card_id,
                target_card_id: request.target_card_id,
                subject: user.subject,
                display_name: user.display_name,
            })
            .await
            .map_err(cluster_error)?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(cluster)
    }

    pub async fn delete_draft_card(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        card_id: Uuid,
    ) -> Result<StatusCode, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        if self
            .repository
            .delete_draft_card(card_id, &user.subject)
            .await
            .map_err(|error| ApiError::internal(format!("failed to delete draft card: {error}")))?
        {
            self.event_hub.publish(BoardEvent::CardChanged { retro_id });
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(ApiError::not_found("draft card not found"))
        }
    }

    pub async fn remove_cluster_member(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        card_id: Uuid,
    ) -> Result<retro_db::CardRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let card = self
            .repository
            .remove_cluster_member(retro_id, card_id)
            .await
            .map_err(|error| {
                ApiError::internal(format!("failed to remove cluster member: {error}"))
            })?
            .ok_or_else(|| ApiError::not_found("cluster member not found"))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(card)
    }

    pub async fn mark_ready(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        self.repository
            .mark_ready(retro_id, &user.subject, &user.display_name)
            .await
            .map_err(|error| ApiError::internal(format!("failed to mark ready: {error}")))?;
        self.event_hub
            .publish(BoardEvent::ReadyChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn unmark_ready(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        self.repository
            .unmark_ready(retro_id, &user.subject)
            .await
            .map_err(|error| ApiError::internal(format!("failed to unmark ready: {error}")))?;
        self.event_hub
            .publish(BoardEvent::ReadyChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn reveal_board(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        force: bool,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        if !force {
            let board = self.fetch_board_for_user(retro_id, &user).await?;
            if board.retro.phase == "writing"
                && board.ready.ready_count < board.ready.participant_count
            {
                return Err(ApiError::bad_request(
                    "everyone must be ready before reveal",
                ));
            }
        }
        self.repository
            .reveal_board(retro_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to reveal board: {error}")))?;
        self.event_hub
            .publish(BoardEvent::PhaseChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn start_voting(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        self.repository
            .start_voting(retro_id)
            .await
            .map_err(voting_error)?;
        self.event_hub
            .publish(BoardEvent::PhaseChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn start_scheduled_retro(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let started = self
            .repository
            .start_scheduled_retro(retro_id)
            .await
            .map_err(|error| {
                ApiError::internal(format!("failed to start scheduled retro: {error}"))
            })?;
        if started.is_some() {
            self.event_hub
                .publish(BoardEvent::PhaseChanged { retro_id });
        } else {
            let retro = self
                .repository
                .fetch_retro(retro_id)
                .await
                .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
                .ok_or_else(|| ApiError::not_found("retro not found"))?;
            if retro.phase != "writing" {
                return Err(ApiError::bad_request(format!(
                    "scheduled retro cannot be started from {}",
                    retro.phase
                )));
            }
        }
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn cast_vote(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: CastVoteRequest,
    ) -> Result<retro_db::VotingInfo, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let info = self
            .repository
            .cast_vote(CastVoteInput {
                retro_id,
                card_id: request.card_id,
                subject: user.subject,
                display_name: user.display_name,
                count: request.count,
            })
            .await
            .map_err(voting_error)?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(info)
    }

    pub async fn remove_vote(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        card_id: Uuid,
    ) -> Result<retro_db::VotingInfo, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let info = self
            .repository
            .remove_vote(retro_id, card_id, &user.subject, &user.display_name)
            .await
            .map_err(voting_error)?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(info)
    }

    pub async fn cluster_board(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        self.repository
            .cluster_board(retro_id)
            .await
            .map_err(cluster_error)?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn start_action_discussion(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        self.repository
            .start_action_discussion(retro_id)
            .await
            .map_err(action_error)?;
        self.event_hub
            .publish(BoardEvent::PhaseChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn update_action(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        action_id: Uuid,
        request: UpdateActionRequest,
    ) -> Result<retro_db::ActionItemRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let action = self
            .repository
            .update_action(UpdateActionInput {
                retro_id,
                action_id,
                title: require_non_empty("title", request.title)?,
                details: request.details,
            })
            .await
            .map_err(|error| ApiError::internal(format!("failed to update action: {error}")))?
            .ok_or_else(|| ApiError::not_found("action not found"))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(action)
    }

    pub async fn set_action_status(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        action_id: Uuid,
        status: &'static str,
    ) -> Result<retro_db::ActionItemRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let action = self
            .repository
            .set_action_status(retro_id, action_id, status)
            .await
            .map_err(|error| {
                ApiError::internal(format!("failed to update action status: {error}"))
            })?
            .ok_or_else(|| ApiError::not_found("action not found"))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(action)
    }

    pub async fn complete_retro(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        self.repository
            .complete_retro(retro_id)
            .await
            .map_err(action_error)?
            .ok_or_else(|| {
                ApiError::bad_request("retro must be in action discussion to complete")
            })?;
        self.repository
            .ensure_next_retro(retro_id, &user.subject, &user.email, &user.display_name)
            .await
            .map_err(|error| ApiError::internal(format!("failed to plan next retro: {error}")))?;
        self.event_hub
            .publish(BoardEvent::PhaseChanged { retro_id });
        // Auto-trigger the summary AI artifact. We deliberately fire and
        // forget: the artifact lifecycle (`pending` → `running` →
        // `succeeded`/`failed`) is persisted, so any failure becomes
        // visible to the wrap-up page via the existing board fetch /
        // WebSocket update — the completion request itself is never
        // blocked or rejected by an AI failure.
        self.spawn_summary_if_configured(retro_id).await;
        self.fetch_board_for_user(retro_id, &user).await
    }

    async fn spawn_summary_if_configured(&self, retro_id: Uuid) {
        let Some(provider) = self.ai_provider.clone() else {
            return;
        };
        let artifact = match self
            .repository
            .create_ai_artifact(
                retro_id,
                ai_summary::KIND,
                serde_json::json!({ "trigger": "complete_retro" }),
            )
            .await
        {
            Ok(artifact) => artifact,
            Err(error) => {
                tracing::warn!(%retro_id, %error, "failed to create summary artifact");
                return;
            }
        };
        let repository = self.repository.clone();
        let event_hub = self.event_hub.clone();
        let artifact_id = artifact.id;
        tokio::spawn(async move {
            ai_summary::run(repository, event_hub, provider, artifact_id, retro_id).await;
        });
    }

    async fn fetch_board_for_user(
        &self,
        retro_id: Uuid,
        user: &CurrentUser,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        self.repository
            .fetch_board_for_user_with_email(retro_id, &user.subject, &user.display_name, &user.email)
            .await
            .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
            .ok_or_else(|| ApiError::not_found("retro not found"))
    }
}

pub(crate) async fn authorize_retro_participant(
    repository: &RetroRepository,
    user: &CurrentUser,
    retro_id: Uuid,
) -> Result<(), ApiError> {
    if repository
        .authorize_retro_participant(retro_id, &user.subject, &user.display_name)
        .await
        .map_err(|error| {
            ApiError::internal(format!("failed to authorize retro participant: {error}"))
        })?
    {
        Ok(())
    } else {
        Err(ApiError::not_found("retro not found"))
    }
}

async fn ensure_retro_host(
    repository: &RetroRepository,
    retro_id: Uuid,
    email: &str,
) -> Result<(), ApiError> {
    if email.is_empty() {
        return Err(ApiError::unauthorized("email required"));
    }
    let email_lc = email.trim().to_lowercase();
    let retro = repository
        .fetch_retro(retro_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
        .ok_or_else(|| ApiError::not_found("retro not found"))?;
    if !retro.creator_email.is_empty() && retro.creator_email == email_lc {
        return Ok(());
    }
    let grants = repository
        .list_board_grants(retro_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to check host: {error}")))?;
    if grants
        .iter()
        .any(|grant| grant.principal_email.eq_ignore_ascii_case(email) && grant.role == "host")
    {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "only the board host can manage this retro",
        ))
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

fn optional_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn require_non_empty(field: &'static str, value: String) -> Result<String, ApiError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err(ApiError::bad_request(format!("{field} cannot be empty")))
    } else {
        Ok(value)
    }
}

fn require_date_only(field: &'static str, value: String) -> Result<String, ApiError> {
    let bytes = value.as_bytes();
    let valid_shape = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
    if !valid_shape {
        return Err(ApiError::bad_request(format!(
            "{field} must use YYYY-MM-DD"
        )));
    }

    let year = value[0..4].parse::<i32>().unwrap_or(0);
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..10].parse::<u32>().unwrap_or(0);
    let leap_year = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > max_day {
        return Err(ApiError::bad_request(format!(
            "{field} is not a valid date"
        )));
    }
    Ok(value)
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

fn retro_template(template: &str, columns: Vec<String>) -> Result<RetroTemplate, ApiError> {
    match template {
        "standard" => Ok(RetroTemplate::Standard),
        "custom" => {
            let columns = columns
                .into_iter()
                .map(|column| column.trim().to_owned())
                .filter(|column| !column.is_empty())
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

fn action_error(error: retro_db::ActionError) -> ApiError {
    match error {
        retro_db::ActionError::Sqlx(error) => {
            ApiError::internal(format!("action discussion failed: {error}"))
        }
        retro_db::ActionError::Invalid(message) => ApiError::bad_request(message),
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
