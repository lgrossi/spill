//! Retro summary AI job.
//!
//! Auto-triggered when a retro completes (see [`RetroWorkflow::complete_retro`])
//! and re-runnable via the existing AI-job retry endpoint. Persists into
//! the shared `ai_artifacts` table with `kind = "summary"`; the board
//! payload already serialises `ai_artifacts`, so the wrap-up tile reads
//! the state from there without any new endpoint.

use std::fmt::Write as _;
use std::sync::Arc;

use retro_db::{RetroBoard, RetroRepository};
use uuid::Uuid;

use crate::ai_provider::AiProvider;
use crate::events::{BoardEvent, BoardEventHub};

pub const KIND: &str = "summary";

/// Run the summary artifact end-to-end: mark `running`, build the
/// prompt, call the provider, persist the result, and emit a board
/// event so connected clients refresh. Designed to be spawned from a
/// background task — any failure is recorded on the artifact, never
/// surfaced to the caller.
pub async fn run(
    repository: RetroRepository,
    event_hub: BoardEventHub,
    provider: Arc<AiProvider>,
    artifact_id: Uuid,
    retro_id: Uuid,
) {
    if let Err(error) = repository.mark_ai_running(artifact_id).await {
        tracing::warn!(%artifact_id, %error, "failed to mark summary artifact running");
        return;
    }

    let board = match repository.fetch_board_readonly(retro_id).await {
        Ok(Some(board)) => board,
        Ok(None) => {
            mark_failed(&repository, artifact_id, "retro vanished before summary ran").await;
            publish_change(&event_hub, retro_id);
            return;
        }
        Err(error) => {
            mark_failed(
                &repository,
                artifact_id,
                &format!("failed to load board: {error}"),
            )
            .await;
            publish_change(&event_hub, retro_id);
            return;
        }
    };

    let prompt = build_prompt(&board);
    match provider.complete(&prompt).await {
        Ok(summary) => {
            let output = serde_json::json!({
                "review_required": false,
                "summary": summary,
            });
            if let Err(error) = repository.complete_ai_artifact(artifact_id, output).await {
                tracing::warn!(%artifact_id, %error, "failed to mark summary artifact succeeded");
            }
        }
        Err(error) => {
            // The provider error already carries enough detail for the
            // operator log; the user-facing message is intentionally
            // generic so we don't leak transport/upstream specifics.
            tracing::warn!(%artifact_id, %error, "ai provider call failed");
            mark_failed(&repository, artifact_id, "AI provider call failed").await;
        }
    }

    publish_change(&event_hub, retro_id);
}

async fn mark_failed(repository: &RetroRepository, artifact_id: Uuid, message: &str) {
    if let Err(error) = repository.fail_ai_artifact(artifact_id, message).await {
        tracing::warn!(%artifact_id, %error, "failed to mark summary artifact failed");
    }
}

fn publish_change(event_hub: &BoardEventHub, retro_id: Uuid) {
    event_hub.publish(BoardEvent::CardChanged { retro_id });
}

/// Builds the prompt sent to the AI provider. Lives next to the runner
/// (not in the provider) because the prompt is a Spill domain concern,
/// not a provider concern.
pub fn build_prompt(board: &RetroBoard) -> String {
    let mut prompt = String::new();
    let _ = writeln!(
        prompt,
        "Summarise this team retrospective in 3 to 4 sentences for an executive reader. \
         Highlight the recurring theme, the loudest concern (most-voted card), and the \
         single most concrete next step. Avoid platitudes."
    );
    let _ = writeln!(prompt, "\nRetro title: {}", board.retro.title);
    let _ = writeln!(prompt, "Phase: {}", board.retro.phase);

    for column in &board.columns {
        let visible: Vec<&retro_db::CardRecord> = column
            .cards
            .iter()
            .filter(|card| !card.hidden && card.parent_card_id.is_none())
            .collect();
        if visible.is_empty() {
            continue;
        }
        let _ = writeln!(prompt, "\n## {} ({} cards)", column.title, visible.len());
        for card in visible {
            let text = card
                .body_text
                .as_deref()
                .or(card.gif_alt_text.as_deref())
                .unwrap_or("(media card)");
            let _ = writeln!(prompt, "- ({} votes) {}", card.vote_count, text);
        }
    }

    if !board.actions.is_empty() {
        let _ = writeln!(prompt, "\n## Committed actions ({})", board.actions.len());
        for action in &board.actions {
            let _ = writeln!(prompt, "- [{}] {}", action.status, action.title);
        }
    }

    prompt
}
