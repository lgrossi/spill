use serde_json::Value;
use uuid::Uuid;

use crate::*;

impl RetroRepository {
    pub async fn create_ai_artifact(
        &self,
        retro_id: Uuid,
        kind: &str,
        input: Value,
    ) -> Result<AiArtifactRecord, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "INSERT INTO ai_artifacts (retro_id, kind, status, input)
             VALUES ($1, $2, 'pending', $3)
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(retro_id)
        .bind(kind)
        .bind(Json(input))
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn create_meeting_note(
        &self,
        input: CreateMeetingNoteInput,
    ) -> Result<MeetingNoteRecord, sqlx::Error> {
        let participant_id = self
            .ensure_participant(
                input.retro_id,
                &input.author_subject,
                &input.author_display_name,
            )
            .await?;
        let row = sqlx::query_as::<_, MeetingNoteRecord>(
            "INSERT INTO meeting_notes (retro_id, author_participant_id, title, body_text)
             VALUES ($1, $2, $3, $4)
             RETURNING id, retro_id, author_participant_id, title, body_text",
        )
        .bind(input.retro_id)
        .bind(participant_id)
        .bind(input.title.trim())
        .bind(input.body_text.trim())
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn ai_input_with_note_context(
        &self,
        retro_id: Uuid,
        kind: &str,
    ) -> Result<Value, sqlx::Error> {
        let notes = self.fetch_meeting_notes(retro_id).await?;
        Ok(serde_json::json!({
            "provider": "fake",
            "kind": kind,
            "meeting_notes_included": matches!(kind, "summary" | "mood") && !notes.is_empty(),
            "meeting_notes": if matches!(kind, "summary" | "mood") { notes } else { Vec::new() },
        }))
    }

    pub async fn mark_ai_running(
        &self,
        artifact_id: Uuid,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'running', error_message = NULL, updated_at = NOW()
             WHERE id = $1
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn retry_ai_artifact(
        &self,
        retro_id: Uuid,
        artifact_id: Uuid,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'running', error_message = NULL, retry_count = retry_count + 1, updated_at = NOW()
             WHERE id = $1 AND retro_id = $2
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn complete_ai_artifact(
        &self,
        artifact_id: Uuid,
        output: Value,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'succeeded', output = $2, error_message = NULL, updated_at = NOW()
             WHERE id = $1
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .bind(Json(output))
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn fail_ai_artifact(
        &self,
        artifact_id: Uuid,
        error_message: &str,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'failed', error_message = $2, updated_at = NOW()
             WHERE id = $1
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .bind(error_message)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn create_delivery(
        &self,
        retro_id: Uuid,
        kind: &str,
        output: Value,
        fail: bool,
    ) -> Result<DeliveryRecord, sqlx::Error> {
        let status = if fail { "failed" } else { "succeeded" };
        let error_message = if fail {
            Some("fake delivery failure")
        } else {
            None
        };
        let row = sqlx::query_as::<_, DeliveryRow>(
            "INSERT INTO deliveries (retro_id, kind, status, output, error_message)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, retro_id, kind, status, output, error_message, retry_count",
        )
        .bind(retro_id)
        .bind(kind)
        .bind(status)
        .bind(Json(output))
        .bind(error_message)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn retry_delivery(
        &self,
        retro_id: Uuid,
        delivery_id: Uuid,
    ) -> Result<Option<DeliveryRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, DeliveryRow>(
            "UPDATE deliveries
             SET status = 'succeeded', error_message = NULL, retry_count = retry_count + 1, updated_at = NOW()
             WHERE id = $1 AND retro_id = $2
             RETURNING id, retro_id, kind, status, output, error_message, retry_count",
        )
        .bind(delivery_id)
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn export_summary_payload(&self, retro_id: Uuid) -> Result<Value, sqlx::Error> {
        let Some(board) = self.fetch_board(retro_id).await? else {
            return Err(sqlx::Error::RowNotFound);
        };
        let confirmed_actions = board
            .actions
            .iter()
            .filter(|action| action.status != "rejected")
            .map(|action| {
                serde_json::json!({
                    "title": action.title,
                    "details": action.details,
                    "status": action.status,
                    "external_action_link": "https://example.invalid/spillio/action-placeholder"
                })
            })
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "title": board.retro.title,
            "phase": board.retro.phase,
            "copy_markdown": format!("# {}\n\nActions: {}", board.retro.title, confirmed_actions.len()),
            "confirmed_actions": confirmed_actions,
            "meeting_notes_count": board.meeting_notes.len(),
            "ai_summary": board.ai_artifacts.iter().find(|artifact| artifact.kind == "summary").and_then(|artifact| artifact.output.clone()),
        }))
    }
}
