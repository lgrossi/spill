use std::sync::Arc;

use retro_core::{AiArtifactKind, DeliveryKind, DomainError};
use retro_db::{CreateMeetingNoteInput, RetroRepository};
use uuid::Uuid;

use crate::{
    ai_provider::AiProvider,
    ai_summary,
    contracts::{CreateDeliveryRequest, CreateMeetingNoteRequest, StartAiJobRequest},
    error::ApiError,
    events::{BoardEvent, BoardEventHub},
    identity::CurrentUser,
    workflow::{authorize_retro_participant, require_non_empty},
};

#[derive(Clone)]
pub struct JobWorkflow {
    repository: RetroRepository,
    event_hub: BoardEventHub,
    /// Optional, opt-in: when set, `summary`-kind AI jobs run through
    /// the real provider in the background. Without it, every kind
    /// (including `summary`) keeps the synchronous fake-provider
    /// behaviour the existing tests assume.
    ai_provider: Option<Arc<AiProvider>>,
}

impl JobWorkflow {
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

    pub async fn start_ai_job(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: StartAiJobRequest,
    ) -> Result<retro_db::AiArtifactRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        validate_ai_kind(&request.kind)?;
        if request.kind == "tagging" {
            require_retro_host(&self.repository, retro_id, &user.email).await?;
        }
        let artifact = self
            .repository
            .create_ai_artifact(
                retro_id,
                &request.kind,
                self.ai_input_with_requested_failure(retro_id, &request.kind, request.fail)
                    .await?,
            )
            .await
            .map_err(|error| ApiError::internal(format!("failed to create AI job: {error}")))?;
        let artifact = self
            .dispatch_ai_job(artifact, retro_id, request.fail)
            .await?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(artifact)
    }

    pub async fn retry_ai_job(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        artifact_id: Uuid,
    ) -> Result<retro_db::AiArtifactRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let artifact = self
            .repository
            .retry_ai_artifact(retro_id, artifact_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to retry AI job: {error}")))?
            .ok_or_else(|| ApiError::not_found("AI artifact not found"))?;
        let artifact = self.dispatch_ai_job(artifact, retro_id, false).await?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(artifact)
    }

    pub async fn create_meeting_note(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: CreateMeetingNoteRequest,
    ) -> Result<retro_db::MeetingNoteRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let note = self
            .repository
            .create_meeting_note(CreateMeetingNoteInput {
                retro_id,
                author_subject: user.subject,
                author_display_name: user.display_name,
                title: optional_non_empty(request.title)
                    .unwrap_or_else(|| "Meeting notes".to_owned()),
                body_text: require_non_empty("body_text", request.body_text)?,
            })
            .await
            .map_err(|error| {
                ApiError::internal(format!("failed to create meeting note: {error}"))
            })?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(note)
    }

    pub async fn create_delivery(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        request: CreateDeliveryRequest,
    ) -> Result<retro_db::DeliveryRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        validate_delivery_kind(&request.kind)?;
        let output = match request.kind.as_str() {
            "summary_export" => self
                .repository
                .export_summary_payload(retro_id)
                .await
                .map_err(|error| {
                    ApiError::internal(format!("failed to export summary: {error}"))
                })?,
            "external_action_link" => serde_json::json!({
                "placeholder_url": "https://example.invalid/spillio/action-placeholder",
                "message": "External action delivery integration placeholder"
            }),
            _ => unreachable!(),
        };
        let delivery = self
            .repository
            .create_delivery(retro_id, &request.kind, output, request.fail)
            .await
            .map_err(|error| ApiError::internal(format!("failed to create delivery: {error}")))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(delivery)
    }

    pub async fn retry_delivery(
        &self,
        user: CurrentUser,
        retro_id: Uuid,
        delivery_id: Uuid,
    ) -> Result<retro_db::DeliveryRecord, ApiError> {
        authorize_retro_participant(&self.repository, &user, retro_id).await?;
        let delivery = self
            .repository
            .retry_delivery(retro_id, delivery_id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to retry delivery: {error}")))?
            .ok_or_else(|| ApiError::not_found("delivery not found"))?;
        self.event_hub.publish(BoardEvent::CardChanged { retro_id });
        Ok(delivery)
    }

    async fn ai_input_with_requested_failure(
        &self,
        retro_id: Uuid,
        kind: &str,
        fail: bool,
    ) -> Result<serde_json::Value, ApiError> {
        let mut input = self
            .repository
            .ai_input_with_note_context(retro_id, kind)
            .await
            .map_err(|error| ApiError::internal(format!("failed to build AI input: {error}")))?;
        input["requested_failure"] = serde_json::json!(fail);
        Ok(input)
    }

    /// Routes a freshly-created (or freshly-retried) artifact to the
    /// right runner. Today only `summary` has a real provider; every
    /// other kind continues to use the synchronous fake provider.
    /// When no provider is configured, even `summary` falls back to
    /// fake — keeps the test suite working without env vars.
    async fn dispatch_ai_job(
        &self,
        artifact: retro_db::AiArtifactRecord,
        retro_id: Uuid,
        fail: bool,
    ) -> Result<retro_db::AiArtifactRecord, ApiError> {
        let is_summary = artifact.kind == ai_summary::KIND;
        if let (true, false, Some(provider)) = (is_summary, fail, self.ai_provider.clone()) {
            let Some(board) = self
                .repository
                .fetch_board_readonly(retro_id)
                .await
                .map_err(|error| {
                    ApiError::internal(format!("failed to load board for AI job: {error}"))
                })?
            else {
                return Err(ApiError::not_found("retro not found"));
            };
            if board.retro.phase != "completed" {
                return self
                    .repository
                    .fail_ai_artifact(
                        artifact.id,
                        "Summary generation is available after retro completion",
                    )
                    .await
                    .map_err(|error| {
                        ApiError::internal(format!("failed to mark AI job failed: {error}"))
                    })?
                    .ok_or_else(|| ApiError::not_found("AI artifact not found"));
            }
            let artifact = self
                .repository
                .mark_ai_running(artifact.id)
                .await
                .map_err(|error| {
                    ApiError::internal(format!("failed to mark AI job running: {error}"))
                })?
                .ok_or_else(|| ApiError::not_found("AI artifact not found"))?;
            let repository = self.repository.clone();
            let event_hub = self.event_hub.clone();
            let artifact_id = artifact.id;
            tokio::spawn(async move {
                ai_summary::run(repository, event_hub, provider, artifact_id, retro_id).await;
            });
            return Ok(artifact);
        }
        self.run_fake_ai_job(artifact, fail).await
    }

    async fn run_fake_ai_job(
        &self,
        artifact: retro_db::AiArtifactRecord,
        fail: bool,
    ) -> Result<retro_db::AiArtifactRecord, ApiError> {
        let artifact = self
            .repository
            .mark_ai_running(artifact.id)
            .await
            .map_err(|error| ApiError::internal(format!("failed to mark AI job running: {error}")))?
            .ok_or_else(|| ApiError::not_found("AI artifact not found"))?;

        if fail {
            return self
                .repository
                .fail_ai_artifact(artifact.id, "fake AI provider failure")
                .await
                .map_err(|error| {
                    ApiError::internal(format!("failed to mark AI job failed: {error}"))
                })?
                .ok_or_else(|| ApiError::not_found("AI artifact not found"));
        }

        let output = fake_ai_output(&artifact.kind, &artifact.input);
        self.repository
            .complete_ai_artifact(artifact.id, output)
            .await
            .map_err(|error| ApiError::internal(format!("failed to complete AI job: {error}")))?
            .ok_or_else(|| ApiError::not_found("AI artifact not found"))
    }
}

async fn require_retro_host(
    repository: &RetroRepository,
    retro_id: Uuid,
    email: &str,
) -> Result<(), ApiError> {
    let retro = repository
        .fetch_retro(retro_id)
        .await
        .map_err(|error| ApiError::internal(format!("failed to fetch retro: {error}")))?
        .ok_or_else(|| ApiError::not_found("retro not found"))?;
    let is_host = repository
        .is_board_host(retro_id, email)
        .await
        .map_err(|error| ApiError::internal(format!("failed to check host access: {error}")))?;
    let is_creator =
        !retro.creator_email.is_empty() && retro.creator_email == email.trim().to_lowercase();
    if !is_host && !is_creator {
        return Err(ApiError::forbidden("only hosts can perform this action"));
    }
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

fn fake_ai_output(kind: &str, input: &serde_json::Value) -> serde_json::Value {
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
            "clusters": fake_tagging_clusters(input)
        }),
        _ => serde_json::json!({"review_required": true}),
    }
}

fn fake_tagging_clusters(input: &serde_json::Value) -> Vec<serde_json::Value> {
    input["tagging"]["clusters"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|cluster| {
            let id = cluster["id"].as_str()?;
            let category = cluster["category"]
                .as_str()
                .or_else(|| cluster["title"].as_str())
                .unwrap_or("follow-up");
            Some(serde_json::json!({
                "cluster_id": id,
                "tags": [category.trim().to_lowercase()]
            }))
        })
        .collect()
}

fn optional_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn domain_error(error: DomainError) -> ApiError {
    match error {
        DomainError::InvalidDomainValue { domain, value } => {
            ApiError::bad_request(format!("invalid {domain}: {value}"))
        }
        other => ApiError::bad_request(format!("domain validation failed: {other:?}")),
    }
}
