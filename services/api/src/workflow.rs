use std::sync::Arc;

use axum::http::StatusCode;
use retro_core::{CardBody, DomainError, IngestedItemPlacement, IngestionSource};
use retro_db::{
    AcceptDeckItemInput, AutoClusterGroupInput, CastVoteInput, ClusterCardsInput, ClusterError,
    CreateRetroInput, DraftCardInput, IngestItemInput, RetroRepository, RetroTemplate,
    UpdateActionInput, UpdateRetroDetailsInput, VotingError,
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
        for invitee in &invitees {
            let email = invitee.email.trim().to_lowercase();
            if email.is_empty() || !email.contains('@') || email == creator_email_lc {
                continue;
            }
            validate_invitee_role(Some(&invitee.role))?;
        }
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
                cover_gif_url: optional_non_empty(request.cover_gif_url),
                cover_gif_alt_text: optional_non_empty(request.cover_gif_alt_text),
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
        let requested_clustering_mode = clustering_mode(request.clustering_mode);
        if requested_clustering_mode == "auto_on_vote_start" {
            self.repository
                .set_clustering_mode(retro_id, &requested_clustering_mode)
                .await
                .map_err(|error| {
                    ApiError::internal(format!("failed to set clustering mode: {error}"))
                })?;
        }
        for invitee in invitees {
            let email = invitee.email.trim().to_lowercase();
            if email.is_empty() || !email.contains('@') || email == creator_email_lc {
                continue;
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
        let cover_gif_url = optional_non_empty(request.cover_gif_url);
        let cover_gif_alt_text = optional_non_empty(request.cover_gif_alt_text);
        let vote_limit = match request.vote_limit {
            Some(value) => Some(require_non_negative("vote_limit", value)?),
            None => None,
        };
        let action_discussion_limit = match request.action_discussion_limit {
            Some(value) => Some(require_non_negative("action_discussion_limit", value)?),
            None => None,
        };
        // Moving top-voted cards into actions requires an actions column. Reject
        // enabling it on a board that has none so the action-discussion phase
        // cannot break later.
        if matches!(action_discussion_limit, Some(value) if value > 0) {
            let columns = self
                .repository
                .fetch_columns(retro_id)
                .await
                .map_err(|error| ApiError::internal(format!("failed to fetch columns: {error}")))?;
            let has_action_column = columns
                .iter()
                .any(|column| column.title.to_lowercase().contains("action"));
            if !has_action_column {
                return Err(ApiError::bad_request(
                    "this board has no actions column, so top voted cards cannot move to actions",
                ));
            }
        }
        let clustering_mode = request
            .clustering_mode
            .map(|value| clustering_mode(Some(value)));
        if title.is_none()
            && group_name.is_none()
            && cover_gif_url.is_none()
            && !request.remove_cover_gif
            && vote_limit.is_none()
            && action_discussion_limit.is_none()
            && clustering_mode.is_none()
        {
            return Err(ApiError::bad_request(
                "title, group_name, cover_gif_url, vote_limit, action_discussion_limit, or clustering_mode is required",
            ));
        }

        self.repository
            .update_retro_details(UpdateRetroDetailsInput {
                retro_id,
                title,
                group_name,
                cover_gif_url,
                cover_gif_alt_text,
                remove_cover_gif: request.remove_cover_gif,
                vote_limit,
                action_discussion_limit,
                clustering_mode,
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
        self.trigger_auto_clustering_compute(retro_id).await;
        self.fetch_board_for_user(retro_id, &user).await
    }

    pub async fn start_voting(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let retro = self
            .repository
            .fetch_retro(retro_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
            .ok_or_else(|| ApiError::not_found("retro not found"))?;
        if retro.phase == "voting" {
            self.trigger_auto_clustering_apply(retro_id).await;
            return self.fetch_board_for_user(retro_id, &user).await;
        }
        self.repository
            .start_voting(retro_id)
            .await
            .map_err(voting_error)?;
        self.trigger_auto_clustering_apply(retro_id).await;
        self.event_hub
            .publish(BoardEvent::PhaseChanged { retro_id });
        self.fetch_board_for_user(retro_id, &user).await
    }

    /// Host applies a ready clustering proposal to the board. Idempotent: a second
    /// apply (or an already-applied board) is a no-op.
    pub async fn apply_clustering(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        ensure_retro_host(&self.repository, retro_id, &user.email).await?;
        let retro = self
            .repository
            .fetch_retro(retro_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
            .ok_or_else(|| ApiError::not_found("retro not found"))?;
        match retro.clustering_status.as_str() {
            "applied" => {}
            "ready" => {
                // A stale apply (e.g. an old discussion-era button) must not
                // reorganize cards after wrap-up generated actions from them.
                if !matches!(retro.phase.as_str(), "discussion" | "voting") {
                    return Err(ApiError::bad_request(
                        "clustering can no longer be applied after action discussion has started",
                    ));
                }
                if let Some(groups) = self
                    .repository
                    .fetch_clustering_proposal(retro_id)
                    .await
                    .map_err(|error| {
                        ApiError::internal(format!("failed to load clustering proposal: {error}"))
                    })?
                {
                    self.repository
                        .apply_auto_cluster_groups(retro_id, groups)
                        .await
                        .map_err(cluster_error)?;
                }
                self.event_hub
                    .publish(BoardEvent::ClusteringChanged { retro_id });
            }
            _ => {
                return Err(ApiError::bad_request("no clustering proposal is ready to apply"));
            }
        }
        self.fetch_board_for_user(retro_id, &user).await
    }

    /// Host retries clustering after a failure (or recomputes): claims a fresh
    /// compute slot and runs it in the background.
    pub async fn retry_clustering(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        ensure_retro_host(&self.repository, retro_id, &user.email).await?;
        self.trigger_auto_clustering_compute(retro_id).await;
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
        require_completion_host(&self.repository, &user, retro_id).await?;
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
        self.spawn_next_title_if_configured(retro_id).await;
        self.event_hub
            .publish(BoardEvent::PhaseChanged { retro_id });
        // Auto-trigger the summary AI artifact. We deliberately fire and
        // forget: the artifact lifecycle (`pending` → `running` →
        // `succeeded`/`failed`) is persisted, so any failure becomes
        // visible to the wrap-up page via the existing board fetch /
        // WebSocket update — the completion request itself is never
        // blocked or rejected by an AI failure.
        self.spawn_summary_if_configured(retro_id).await;
        let mut board = self.fetch_board_for_user(retro_id, &user).await?;
        if self.ai_provider.is_some() {
            if let Some(next_retro) = board.next_retro.as_mut() {
                next_retro.title = "Generating title...".to_owned();
            }
        }
        Ok(board)
    }

    async fn spawn_next_title_if_configured(&self, retro_id: Uuid) {
        let Some(provider) = self.ai_provider.clone() else {
            return;
        };
        let repository = self.repository.clone();
        let event_hub = self.event_hub.clone();
        tokio::spawn(async move {
            let fallback = repository
                .fetch_retro(retro_id)
                .await
                .ok()
                .flatten()
                .map(|retro| next_title(&retro.title))
                .unwrap_or_else(|| "Next retro".to_owned());
            let recent_titles = repository
                .fetch_recent_series_titles(retro_id, 6)
                .await
                .unwrap_or_default();
            let title = suggest_next_title(provider, &fallback, &recent_titles)
                .await
                .unwrap_or_else(|| fallback.clone());
            let updated = repository
                .finish_next_retro_title(retro_id, &title, Some(&fallback))
                .await
                .ok()
                .flatten();
            event_hub.publish(BoardEvent::CardChanged { retro_id });
            if let Some(next_retro) = updated {
                event_hub.publish(BoardEvent::CardChanged {
                    retro_id: next_retro.id,
                });
            }
        });
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

    /// Discussion-phase: compute a clustering proposal in the background without
    /// mutating the board. Claims a single compute slot; a new run replaces a
    /// prior proposal.
    async fn trigger_auto_clustering_compute(&self, retro_id: Uuid) {
        let Some(provider) = self.ai_provider.clone() else {
            return;
        };
        match self.repository.claim_clustering_compute(retro_id).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!(%retro_id, %error, "failed to claim clustering compute");
                return;
            }
        }
        let repository = self.repository.clone();
        let event_hub = self.event_hub.clone();
        tokio::spawn(async move {
            run_clustering_compute(repository, event_hub, provider, retro_id).await;
        });
        self.event_hub
            .publish(BoardEvent::ClusteringChanged { retro_id });
    }

    /// Voting transition: apply the ready proposal in the background, or compute
    /// and apply as a fallback when none is ready. Non-auto retros are a no-op.
    async fn trigger_auto_clustering_apply(&self, retro_id: Uuid) {
        let provider = self.ai_provider.clone();
        let repository = self.repository.clone();
        let event_hub = self.event_hub.clone();
        tokio::spawn(async move {
            run_auto_clustering_apply(repository, event_hub, provider, retro_id).await;
        });
        self.event_hub
            .publish(BoardEvent::ClusteringChanged { retro_id });
    }

    async fn fetch_board_for_user(
        &self,
        retro_id: Uuid,
        user: &CurrentUser,
    ) -> Result<retro_db::RetroBoard, ApiError> {
        self.repository
            .fetch_board_for_user_with_email(
                retro_id,
                &user.subject,
                &user.display_name,
                &user.email,
            )
            .await
            .map_err(|error| ApiError::internal(format!("failed to open retro: {error}")))?
            .ok_or_else(|| ApiError::not_found("retro not found"))
    }
}

async fn run_clustering_compute(
    repository: RetroRepository,
    event_hub: BoardEventHub,
    provider: Arc<AiProvider>,
    retro_id: Uuid,
) {
    match compute_clustering_proposal(&repository, provider, retro_id).await {
        Ok(groups) => {
            if let Err(error) = repository.store_clustering_proposal(retro_id, &groups).await {
                tracing::warn!(%retro_id, %error, "failed to store clustering proposal");
                let _ = repository.mark_clustering_failed(retro_id).await;
            } else if let Err(error) = apply_ready_during_voting(&repository, retro_id).await {
                // Voting may already have started (or a voting-phase retry ran):
                // the start-voting auto-apply could have stopped waiting, so the
                // compute that just finished must apply itself.
                tracing::warn!(%retro_id, ?error, "failed to auto-apply clustering after compute");
            }
        }
        Err(error) => {
            tracing::warn!(%retro_id, ?error, "clustering compute failed");
            if let Err(mark_error) = repository.mark_clustering_failed(retro_id).await {
                tracing::warn!(%retro_id, %mark_error, "failed to mark clustering failed");
            }
        }
    }
    event_hub.publish(BoardEvent::ClusteringChanged { retro_id });
}

/// Apply a freshly-stored proposal when the retro is in voting. In discussion the
/// host applies explicitly; once voting starts apply is automatic, so a slow
/// compute (or a voting-phase retry) still lands on the board. It deliberately
/// does not apply from `action_discussion` onward: by then actions have been
/// generated from the current cards, and a late reorganization would desync
/// votes/actions. Idempotent: a no-op once already applied.
async fn apply_ready_during_voting(
    repository: &RetroRepository,
    retro_id: Uuid,
) -> Result<(), ApiError> {
    let retro = repository
        .fetch_retro(retro_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
        .ok_or_else(|| ApiError::not_found("retro not found"))?;
    if retro.clustering_mode != "auto_on_vote_start" {
        return Ok(());
    }
    if retro.phase != "voting" {
        return Ok(());
    }
    if let Some(groups) = repository
        .fetch_clustering_proposal(retro_id)
        .await
        .map_err(|error| {
            ApiError::internal(format!("failed to load clustering proposal: {error}"))
        })?
    {
        repository
            .apply_auto_cluster_groups(retro_id, groups)
            .await
            .map_err(cluster_error)?;
    }
    Ok(())
}

async fn compute_clustering_proposal(
    repository: &RetroRepository,
    provider: Arc<AiProvider>,
    retro_id: Uuid,
) -> Result<Vec<AutoClusterGroupInput>, ApiError> {
    let board = repository
        .fetch_board_readonly(retro_id)
        .await
        .map_err(|error| {
            ApiError::internal(format!("failed to load board for organization: {error}"))
        })?
        .ok_or_else(|| ApiError::not_found("retro not found"))?;
    let existing_tags = repository
        .existing_cluster_tag_context(retro_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to load tag context: {error}")))?;
    let prompt = build_auto_cluster_prompt(&board, &existing_tags);
    let response = provider
        .complete(&prompt)
        .await
        .map_err(|error| ApiError::internal(format!("organization AI failed: {error}")))?;
    auto_cluster_groups_from_response(&response).map_err(|error| ApiError::internal(error.to_owned()))
}

async fn run_auto_clustering_apply(
    repository: RetroRepository,
    event_hub: BoardEventHub,
    provider: Option<Arc<AiProvider>>,
    retro_id: Uuid,
) {
    if let Err(error) = ensure_clustering_applied(&repository, provider, retro_id).await {
        tracing::warn!(%retro_id, ?error, "auto clustering apply failed");
        if let Err(mark_error) = repository.mark_clustering_failed(retro_id).await {
            tracing::warn!(%retro_id, %mark_error, "failed to mark clustering failed");
        }
    }
    event_hub.publish(BoardEvent::ClusteringChanged { retro_id });
}

/// Drive the proposal to `applied`: apply a ready proposal, wait out an in-flight
/// compute, or compute+store as a fallback (which the next iteration applies).
async fn ensure_clustering_applied(
    repository: &RetroRepository,
    provider: Option<Arc<AiProvider>>,
    retro_id: Uuid,
) -> Result<(), ApiError> {
    for _ in 0..140 {
        let retro = repository
            .fetch_retro(retro_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
            .ok_or_else(|| ApiError::not_found("retro not found"))?;
        if retro.clustering_mode != "auto_on_vote_start" {
            return Ok(());
        }
        // Stop if the host has already wrapped up: applying after actions are
        // generated from the current cards would desync votes/actions.
        if retro.phase != "voting" {
            return Ok(());
        }
        match retro.clustering_status.as_str() {
            "applied" => return Ok(()),
            "ready" => {
                if let Some(groups) = repository
                    .fetch_clustering_proposal(retro_id)
                    .await
                    .map_err(|error| {
                        ApiError::internal(format!("failed to load clustering proposal: {error}"))
                    })?
                {
                    repository
                        .apply_auto_cluster_groups(retro_id, groups)
                        .await
                        .map_err(cluster_error)?;
                }
                return Ok(());
            }
            "computing" => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            _ => {
                let Some(provider) = provider.clone() else {
                    return Ok(());
                };
                if repository.claim_clustering_compute(retro_id).await.map_err(|error| {
                    ApiError::internal(format!("failed to claim clustering compute: {error}"))
                })? {
                    let groups = compute_clustering_proposal(repository, provider, retro_id).await?;
                    repository
                        .store_clustering_proposal(retro_id, &groups)
                        .await
                        .map_err(|error| {
                            ApiError::internal(format!("failed to store clustering proposal: {error}"))
                        })?;
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }
    Ok(())
}

pub(crate) async fn authorize_retro_participant(
    repository: &RetroRepository,
    user: &CurrentUser,
    retro_id: Uuid,
) -> Result<(), ApiError> {
    if !repository
        .is_board_member(retro_id, &user.email)
        .await
        .map_err(|error| ApiError::internal(format!("failed to check board access: {error}")))?
    {
        return Err(ApiError::forbidden("not a member of this board"));
    }
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

async fn require_completion_host(
    repository: &RetroRepository,
    user: &CurrentUser,
    retro_id: Uuid,
) -> Result<(), ApiError> {
    let allowed = repository
        .is_retro_host(retro_id, &user.subject, &user.email)
        .await
        .map_err(|error| ApiError::internal(format!("failed to check host access: {error}")))?;
    if allowed {
        Ok(())
    } else {
        Err(ApiError::forbidden("only a host can finish this retro"))
    }
}

async fn suggest_next_title(
    provider: Arc<AiProvider>,
    fallback: &str,
    recent_titles: &[String],
) -> Option<String> {
    let prompt = build_next_title_prompt(fallback, recent_titles);
    let suggested = provider.complete(&prompt).await.ok()?;
    let title = suggested
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_owned();
    if title.is_empty() || title.len() > 80 {
        None
    } else {
        Some(title)
    }
}

fn build_next_title_prompt(fallback: &str, recent_titles: &[String]) -> String {
    let mut prompt = format!(
        "You are naming the next session in a recurring retrospective series.\n\
         Return only one plain title and nothing else.\n\
\n\
         Use the previous sequence titles to infer the naming pattern, tone, and stable team/project wording.\n\
         If the sequence has no clear pattern, minimally improve the fallback instead of being creative.\n\
\n\
         Rules:\n\
         - 2 to 5 words.\n\
         - Must be recognizable as the next retro in the same series.\n\
         - Keep stable team/project wording from the fallback when present.\n\
         - Do not add dates, numbering, emojis, quotes, markdown, or punctuation-heavy copy.\n\
         - Do not invent facts, team names, incidents, or themes that are not in the fallback.\n\
         - Avoid generic hype words like awesome, amazing, legendary, or epic.\n\
         - If unsure, minimally improve the fallback instead of being creative.\n\
\n\
         Fallback title: {}\n",
        fallback.trim().replace('\"', "'")
    );
    if !recent_titles.is_empty() {
        prompt.push_str("\nPrevious sequence titles, oldest to newest:\n");
        for title in recent_titles {
            prompt.push_str("- ");
            prompt.push_str(&title.trim().replace('\"', "'"));
            prompt.push('\n');
        }
    }
    prompt
}

fn next_title(source_title: &str) -> String {
    let trimmed = source_title.trim();
    if let Some(rest) = trimmed.strip_prefix("Next: ") {
        format!("Next: {rest}")
    } else {
        format!("Next: {trimmed}")
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

fn clustering_mode(value: Option<String>) -> String {
    match value.as_deref().map(str::trim) {
        Some("auto_on_vote_start") => "auto_on_vote_start".to_owned(),
        _ => "disabled".to_owned(),
    }
}

fn build_auto_cluster_prompt(board: &retro_db::RetroBoard, existing_tags: &[String]) -> String {
    let mut prompt = String::from(
        "You are organizing retrospective cards after voting has started.\n\
         Your job is to group cards by the concrete topic, problem, idea, or action described in their text.\n\
\n\
         Return exactly one valid JSON object and nothing else.\n\
         Required schema: {\"groups\":[{\"title\":\"string\",\"summary\":\"string\",\"card_ids\":[\"uuid\"],\"category\":\"string|null\",\"tags\":[\"string\"]}]}\n\
\n\
         Eligible input:\n\
         - Only use the cards listed under Eligible cards.\n\
         - Mood/check-in columns such as \"How are you feeling?\" are excluded and must not be inferred.\n\
         - Treat body text and GIF/media alt descriptions as text that describes participant intent.\n\
         - Ignore media type itself; a GIF card is not related to another GIF card unless the text says the same thing.\n\
\n\
         Grouping rules:\n\
         - Group cards only when their text describes the same specific theme, issue, win, risk, or next step.\n\
         - Do not group by column name, broad sentiment, card format, author, vote count, or the presence of media.\n\
         - Column is context only; it is never a grouping reason and must not become the group title.\n\
         - Never put cards from different columns in the same group; every card in a group must come from the same column.\n\
         - If there is no real shared textual theme, keep the card as a single-card group.\n\
         - Include every eligible card id exactly once across groups.\n\
         - A card id can belong to only one group; never reuse the same card id in multiple groups.\n\
\n\
         Title and metadata rules:\n\
         - title: concise human-readable theme from the grouped card content, 2 to 5 words.\n\
         - title must not be copied from a column name and must not be just a category/tag.\n\
         - summary: one short sentence explaining why the cards belong together.\n\
         - category: broad area or null.\n\
         - tags: lowercase, short, deduped; prefer existing tags when they fit and do not invent near-duplicates.\n",
    );
    prompt.push_str("\nRetro title: ");
    prompt.push_str(&board.retro.title.replace('"', "'"));
    prompt.push('\n');
    if !existing_tags.is_empty() {
        prompt.push_str("Existing cluster tags to prefer: ");
        prompt.push_str(&existing_tags.join(", "));
        prompt.push('\n');
    }
    prompt.push_str("\nEligible cards:\n");
    for column in &board.columns {
        if is_mood_column(&column.title, &column.column_key) {
            continue;
        }
        for card in &column.cards {
            if card.hidden || card.parent_card_id.is_some() || card.cluster_id.is_some() {
                continue;
            }
            let text = [card.body_text.as_deref(), card.gif_alt_text.as_deref()]
                .into_iter()
                .flatten()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" [media] ");
            prompt.push_str(&format!(
                "- id={} column_context=\"{}\" text=\"{}\"\n",
                card.id,
                column.title,
                text.replace('"', "'")
            ));
        }
    }
    prompt
}

fn is_mood_column(title: &str, column_key: &str) -> bool {
    let title = title.trim().to_lowercase();
    let key = column_key.trim().to_lowercase();
    key.contains("feeling")
        || key.contains("mood")
        || title.contains("how are you feeling")
        || title.contains("how do you feel")
        || title.contains("feeling")
        || title.contains("mood")
}

fn auto_cluster_groups_from_response(
    response: &str,
) -> Result<Vec<AutoClusterGroupInput>, &'static str> {
    let value = serde_json::from_str::<serde_json::Value>(response)
        .or_else(|_| serde_json::from_str::<serde_json::Value>(json_object_body(response)))
        .map_err(|_| "AI provider returned invalid organization JSON")?;
    let groups = value
        .get("groups")
        .and_then(serde_json::Value::as_array)
        .ok_or("AI provider returned organization JSON without groups")?;
    let groups = groups
        .iter()
        .filter_map(|group| {
            let title = group.get("title")?.as_str()?.trim().to_owned();
            let card_ids = group
                .get("card_ids")?
                .as_array()?
                .iter()
                .filter_map(|id| id.as_str()?.parse().ok())
                .collect::<Vec<_>>();
            if title.is_empty() || card_ids.is_empty() {
                return None;
            }
            let details = group
                .get("summary")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            let category = group
                .get("category")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_lowercase());
            let tags = group
                .get("tags")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect();
            Some(AutoClusterGroupInput {
                title,
                details,
                category,
                tags,
                card_ids,
            })
        })
        .collect::<Vec<_>>();
    if groups.is_empty() {
        return Err("AI provider returned no usable organization groups");
    }
    Ok(groups)
}

fn json_object_body(response: &str) -> &str {
    let Some(start) = response.find('{') else {
        return response;
    };
    let Some(end) = response.rfind('}') else {
        return response;
    };
    if start <= end {
        &response[start..=end]
    } else {
        response
    }
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

fn validate_invitee_role(role: Option<&str>) -> Result<String, ApiError> {
    match role.unwrap_or("member") {
        "host" => Ok("host".to_owned()),
        "member" => Ok("member".to_owned()),
        other => Err(ApiError::bad_request(format!(
            "role must be \"host\" or \"member\", got: {other}"
        ))),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_cluster_prompt_excludes_mood_columns_and_guides_titles() {
        let mood_card_id = uuid::Uuid::from_u128(1);
        let work_card_id = uuid::Uuid::from_u128(2);
        let board = retro_db::RetroBoard {
            retro: retro_db::RetroRecord {
                id: uuid::Uuid::from_u128(10),
                title: "Sprint 42".to_owned(),
                phase: "voting".to_owned(),
                vote_limit: 3,
                action_discussion_limit: 3,
                creator_email: "host@example.com".to_owned(),
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: "2026-06-02".to_owned(),
                happened_at: None,
                clustering_mode: "auto_on_vote_start".to_owned(),
                clustering_status: "running".to_owned(),
            },
            series: None,
            next_retro: None,
            participants: Vec::new(),
            columns: vec![
                retro_column(
                    uuid::Uuid::from_u128(11),
                    "0_how_are_you_feeling",
                    "How are you feeling?",
                    mood_card_id,
                    "Tired but okay",
                ),
                retro_column(
                    uuid::Uuid::from_u128(12),
                    "1_went_well",
                    "Went well",
                    work_card_id,
                    "Deploys got faster",
                ),
            ],
            ready: Default::default(),
            voting: Default::default(),
            clusters: Vec::new(),
            actions: Vec::new(),
            deck: Vec::new(),
            ai_artifacts: Vec::new(),
            meeting_notes: Vec::new(),
            deliveries: Vec::new(),
        };

        let prompt = build_auto_cluster_prompt(&board, &[]);

        assert!(!prompt.contains(&mood_card_id.to_string()));
        assert!(prompt.contains(&work_card_id.to_string()));
        assert!(prompt.contains("group cards by the concrete topic, problem, idea, or action"));
        assert!(prompt.contains("Treat body text and GIF/media alt descriptions as text"));
        assert!(prompt.contains("Include every eligible card id exactly once"));
        assert!(prompt.contains("never reuse the same card id in multiple groups"));
        assert!(prompt.contains("Column is context only"));
        assert!(prompt.contains("every card in a group must come from the same column"));
        assert!(prompt.contains("must not be copied from a column name"));
    }

    #[test]
    fn auto_cluster_parser_extracts_groups() {
        let output = auto_cluster_groups_from_response(
            r#"{"groups":[{"title":"Deploy pain","summary":"Release friction","card_ids":["11111111-1111-1111-1111-111111111111"],"category":"Delivery","tags":["Release","release"," deploy "]}]}"#,
        )
        .unwrap();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0].title, "Deploy pain");
        assert_eq!(output[0].details.as_deref(), Some("Release friction"));
        assert_eq!(output[0].category.as_deref(), Some("delivery"));
        assert_eq!(output[0].tags, vec!["Release", "release", " deploy "]);
        assert_eq!(output[0].card_ids.len(), 1);
    }

    #[test]
    fn next_title_prompt_sets_clear_title_rules() {
        let prompt = build_next_title_prompt(
            "Next: Platform Retro",
            &[
                "Platform Retro".to_owned(),
                "Platform Retro: May".to_owned(),
            ],
        );

        assert!(prompt.contains("Return only one plain title and nothing else"));
        assert!(prompt.contains("Use the previous sequence titles to infer the naming pattern"));
        assert!(prompt.contains("2 to 5 words"));
        assert!(prompt.contains("Do not add dates, numbering, emojis"));
        assert!(prompt.contains("Do not invent facts"));
        assert!(prompt.contains("Fallback title: Next: Platform Retro"));
        assert!(prompt.contains("Previous sequence titles, oldest to newest"));
        assert!(prompt.contains("Platform Retro: May"));
    }

    fn retro_column(
        id: uuid::Uuid,
        column_key: &str,
        title: &str,
        card_id: uuid::Uuid,
        body_text: &str,
    ) -> retro_db::RetroColumnRecord {
        retro_db::RetroColumnRecord {
            id,
            retro_id: uuid::Uuid::from_u128(10),
            column_key: column_key.to_owned(),
            title: title.to_owned(),
            position: 0,
            accent_color: None,
            cards: vec![retro_db::CardRecord {
                id: card_id,
                retro_id: uuid::Uuid::from_u128(10),
                column_id: id,
                author_participant_id: uuid::Uuid::from_u128(20),
                body_text: Some(body_text.to_owned()),
                gif_url: None,
                gif_alt_text: None,
                state: "revealed".to_owned(),
                position: 0,
                cluster_id: None,
                parent_card_id: None,
                cluster_details: None,
                cluster_title: None,
                cluster_category: None,
                vote_count: 0,
                current_user_vote_count: 0,
                hidden: false,
                cluster_members: Vec::new(),
            }],
        }
    }
}
