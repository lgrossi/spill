//! Retro summary AI job.
//!
//! Auto-triggered when a retro completes (see [`RetroWorkflow::complete_retro`])
//! and re-runnable via the existing AI-job retry endpoint. Persists into
//! the shared `ai_artifacts` table with `kind = "summary"`; the board
//! payload already serialises `ai_artifacts`, so the wrap-up tile reads
//! the state from there without any new endpoint.

use std::fmt::Write as _;
use std::sync::Arc;

use retro_db::{
    ActionItemRecord, CardRecord, ClusterMemberRecord, MeetingNoteRecord, RetroBoard,
    RetroRepository,
};
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
            mark_failed(
                &repository,
                artifact_id,
                "retro vanished before summary ran",
            )
            .await;
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
        Ok(response) => match output_from_provider_response(&response) {
            Ok(output) => {
                if let Err(error) = repository.complete_ai_artifact(artifact_id, output).await {
                    tracing::warn!(%artifact_id, %error, "failed to mark summary artifact succeeded");
                }
            }
            Err(message) => {
                tracing::warn!(%artifact_id, "ai provider returned invalid summary JSON");
                mark_failed(&repository, artifact_id, message).await;
            }
        },
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
        "Return exactly one valid JSON object and nothing else.\n\
         Required schema:\n\
         {{\n\
           \"team_mood\": \"quietly-proud\" | \"smooth-sailing\" | \"good-sparks\" | \"productive-chaos\" | \"foggy\" | \"spicy\" | \"stuck-in-mud\" | \"needs-a-map\",\n\
           \"summary\": \"string\"\n\
         }}\n\
         Rules:\n\
         - Use double quotes.\n\
         - Do not wrap the JSON in markdown or a code fence.\n\
         - Do not include extra keys.\n\
         - team_mood must be exactly one allowed value.\n\
         - summary must be 2 short sentences, 35 to 55 words total.\n\
         Write summary in simple, human language. Give it a small proverb-like turn: calm, \
         memorable, and a little playful, but never grand, mystical, or fake-wise. Use at most \
         one light image. Do not mention the mood label directly or reuse its exact words.\n\
         Use only the structured evidence below; do not invent facts, owners, or actions. Start with the \
         overall pattern, include the strongest concern using vote counts and repeated themes as signal, \
         and end with the most concrete next step from committed actions. If multiple committed actions \
         clearly reinforce the same next step, combine them. If there is no committed action, infer one \
         cautiously from the evidence.\n\
         Treat media/GIF descriptions as participant intent, not literal events. Do not use bullets, \
         headings, labels, markdown, colon-heavy analysis, generic platitudes, ornate metaphors, \
         or empty poetic fluff."
    );
    let _ = writeln!(prompt, "\nRetro title: {}", board.retro.title);
    let _ = writeln!(prompt, "Phase: {}", board.retro.phase);

    for column in &board.columns {
        let visible: Vec<&CardRecord> = column
            .cards
            .iter()
            .filter(|card| !card.hidden && card.parent_card_id.is_none())
            .collect();
        if visible.is_empty() {
            continue;
        }
        let visible_count = visible
            .iter()
            .map(|card| prompt_card_count(card))
            .sum::<usize>();
        let _ = writeln!(prompt, "\n## {} ({} cards)", column.title, visible_count);
        for card in visible {
            let text = card_text(card.body_text.as_deref(), card.gif_alt_text.as_deref());
            let visible_members: Vec<&ClusterMemberRecord> = card
                .cluster_members
                .iter()
                .filter(|member| !member.hidden)
                .collect();
            if visible_members.is_empty() {
                let _ = writeln!(prompt, "- ({} votes) {}", card.vote_count, text);
                continue;
            }

            let _ = writeln!(
                prompt,
                "- Cluster: {} ({} votes on group)",
                text, card.vote_count
            );
            for member in visible_members {
                let member_text =
                    card_text(member.body_text.as_deref(), member.gif_alt_text.as_deref());
                let _ = writeln!(prompt, "  - ({} votes) {}", member.vote_count, member_text);
            }
        }
    }

    let committed_actions: Vec<&ActionItemRecord> = board
        .actions
        .iter()
        .filter(|action| action.status != "rejected")
        .collect();
    if !committed_actions.is_empty() {
        let _ = writeln!(
            prompt,
            "\n## Committed actions ({})",
            committed_actions.len()
        );
        for action in committed_actions {
            let _ = writeln!(prompt, "- [{}] {}", action.status, action.title);
        }
    }

    let visible_notes: Vec<&MeetingNoteRecord> = board
        .meeting_notes
        .iter()
        .filter(|note| !note.body_text.trim().is_empty())
        .collect();
    if !visible_notes.is_empty() {
        let _ = writeln!(prompt, "\n## Meeting notes ({})", visible_notes.len());
        for note in visible_notes {
            let _ = writeln!(prompt, "- {}: {}", note.title, note.body_text);
        }
    }

    prompt
}

fn prompt_card_count(card: &CardRecord) -> usize {
    let visible_members = card
        .cluster_members
        .iter()
        .filter(|member| !member.hidden)
        .count();
    if visible_members == 0 {
        1
    } else {
        visible_members
    }
}

fn card_text(body_text: Option<&str>, gif_alt_text: Option<&str>) -> String {
    let body_text = body_text.map(str::trim).filter(|text| !text.is_empty());
    let media_text = gif_alt_text.map(str::trim).filter(|text| !text.is_empty());
    match (body_text, media_text) {
        (Some(body), Some(media)) => format!("{body} [media: {media}]"),
        (Some(body), None) => body.to_owned(),
        (None, Some(media)) => format!("[media: {media}]"),
        (None, None) => "[media card without text metadata]".to_owned(),
    }
}

fn output_from_provider_response(response: &str) -> Result<serde_json::Value, &'static str> {
    let parsed = parse_provider_json(response);
    let summary = match &parsed {
        Some(value) => value
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or("AI provider returned JSON without a summary")?,
        None if looks_like_json_response(response) => {
            return Err("AI provider returned invalid JSON");
        }
        None => response,
    };
    let team_mood = match &parsed {
        Some(value) => {
            let mood = value
                .get("team_mood")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or("AI provider returned JSON without a team mood")?;
            if !is_allowed_team_mood(mood) {
                return Err("AI provider returned unknown team mood");
            }
            Some(mood)
        }
        None => None,
    };

    let mut output = serde_json::json!({
        "review_required": false,
        "summary": summary,
    });
    if let Some(team_mood) = team_mood {
        output["team_mood"] = serde_json::json!(team_mood);
    }
    Ok(output)
}

fn looks_like_json_response(response: &str) -> bool {
    let trimmed = response.trim();
    trimmed.starts_with('{')
        || trimmed.starts_with("```")
        || (trimmed.contains('{') && trimmed.contains('}'))
}

fn parse_provider_json(response: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(response)
        .or_else(|_| serde_json::from_str::<serde_json::Value>(fenced_json_body(response)))
        .or_else(|_| serde_json::from_str::<serde_json::Value>(json_object_body(response)))
        .ok()
}

fn fenced_json_body(response: &str) -> &str {
    let trimmed = response.trim();
    let Some(after_fence) = trimmed.strip_prefix("```") else {
        return response;
    };
    let after_language = after_fence
        .strip_prefix("json")
        .or_else(|| after_fence.strip_prefix("JSON"))
        .unwrap_or(after_fence);
    after_language
        .strip_suffix("```")
        .unwrap_or(after_language)
        .trim()
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

fn is_allowed_team_mood(value: &str) -> bool {
    matches!(
        value,
        "quietly-proud"
            | "smooth-sailing"
            | "good-sparks"
            | "productive-chaos"
            | "foggy"
            | "spicy"
            | "stuck-in-mud"
            | "needs-a-map"
    )
}

#[cfg(test)]
mod tests {
    use retro_db::{
        ActionItemRecord, CardRecord, ClusterMemberRecord, MeetingNoteRecord, RetroBoard,
        RetroColumnRecord, RetroRecord, VotingInfo,
    };

    use super::*;

    #[test]
    fn build_prompt_requests_structured_mood_and_picturesque_summary() {
        let prompt = build_prompt(&RetroBoard {
            retro: retro(uuid(1)),
            series: None,
            next_retro: None,
            participants: Vec::new(),
            columns: Vec::new(),
            ready: Default::default(),
            voting: VotingInfo {
                vote_limit: 3,
                votes_used: 0,
                votes_remaining: 3,
            },
            clusters: Vec::new(),
            actions: Vec::new(),
            deck: Vec::new(),
            ai_artifacts: Vec::new(),
            meeting_notes: Vec::new(),
            deliveries: Vec::new(),
        });

        assert!(prompt.contains("Return exactly one valid JSON object and nothing else"));
        assert!(prompt.contains("\"team_mood\""));
        assert!(prompt.contains("\"quietly-proud\" | \"smooth-sailing\" | \"good-sparks\""));
        assert!(prompt.contains("summary must be 2 short sentences, 35 to 55 words total"));
        assert!(prompt.contains("small proverb-like turn"));
        assert!(prompt.contains("Use at most one light image"));
        assert!(prompt.contains("Do not mention the mood label directly or reuse its exact words"));
        assert!(prompt.contains("colon-heavy analysis"));
        assert!(prompt.contains("ornate metaphors"));
        assert!(prompt.contains("empty poetic fluff"));
    }

    #[test]
    fn output_from_provider_response_extracts_json_summary_and_mood() {
        let output = output_from_provider_response(
            r#"{"team_mood":"needs-a-map","summary":"The team found the path, but the signposts need work."}"#,
        )
        .unwrap();

        assert_eq!(output["review_required"], false);
        assert_eq!(output["team_mood"], "needs-a-map");
        assert_eq!(
            output["summary"],
            "The team found the path, but the signposts need work."
        );
    }

    #[test]
    fn output_from_provider_response_keeps_plain_text_compatible() {
        let output = output_from_provider_response("Plain summary from legacy provider").unwrap();

        assert_eq!(output["review_required"], false);
        assert_eq!(output["summary"], "Plain summary from legacy provider");
        assert!(output.get("team_mood").is_none());
    }

    #[test]
    fn output_from_provider_response_rejects_unknown_mood() {
        let error = output_from_provider_response(
            r#"{"team_mood":"dumpster-fire","summary":"The team had signal without the label."}"#,
        )
        .expect_err("unknown structured mood should fail");

        assert_eq!(error, "AI provider returned unknown team mood");
    }

    #[test]
    fn output_from_provider_response_extracts_fenced_json() {
        let output = output_from_provider_response(
            "```json\n{\"team_mood\":\"spicy\",\"summary\":\"The retro had heat, but the kitchen stayed open.\"}\n```",
        )
        .unwrap();

        assert_eq!(output["team_mood"], "spicy");
        assert_eq!(
            output["summary"],
            "The retro had heat, but the kitchen stayed open."
        );
    }

    #[test]
    fn output_from_provider_response_extracts_surrounded_json_object() {
        let output = output_from_provider_response(
            "Here you go: {\"team_mood\":\"foggy\",\"summary\":\"The signal is there, but the path needs clearing.\"}",
        )
        .unwrap();

        assert_eq!(output["team_mood"], "foggy");
        assert_eq!(
            output["summary"],
            "The signal is there, but the path needs clearing."
        );
    }

    #[test]
    fn output_from_provider_response_rejects_json_without_summary() {
        let error = output_from_provider_response(r#"{"team_mood":"spicy","summary":""}"#)
            .expect_err("empty structured summary should fail");

        assert_eq!(error, "AI provider returned JSON without a summary");
    }

    #[test]
    fn output_from_provider_response_rejects_json_without_mood() {
        let error = output_from_provider_response(r#"{"summary":"The team had signal."}"#)
            .expect_err("missing structured mood should fail");

        assert_eq!(error, "AI provider returned JSON without a team mood");
    }

    #[test]
    fn output_from_provider_response_rejects_malformed_json_like_text() {
        let error = output_from_provider_response(
            r#"```json
{"team_mood":"spicy","summary":"The retro had heat"
```"#,
        )
        .expect_err("malformed fenced JSON should fail");

        assert_eq!(error, "AI provider returned invalid JSON");
    }

    #[test]
    fn build_prompt_includes_visible_cluster_members() {
        let retro_id = uuid(1);
        let column_id = uuid(2);
        let author_id = uuid(3);
        let mut group_card = card(
            retro_id,
            column_id,
            author_id,
            "Grouped: platform friction",
            1,
        );
        group_card.cluster_id = Some(uuid(4));
        group_card.cluster_members = vec![
            cluster_member(6, author_id, "Deploys are slow", 4, false),
            cluster_member(7, author_id, "Staging flakes during QA", 2, false),
            cluster_member(8, author_id, "Hidden draft", 9, true),
        ];

        let board = RetroBoard {
            retro: RetroRecord {
                id: retro_id,
                title: "Sprint 42".to_owned(),
                phase: "completed".to_owned(),
                vote_limit: 3,
                action_discussion_limit: 3,
                creator_email: "host@example.com".to_owned(),
                planned_for: "2026-05-15".to_owned(),
                happened_at: Some("2026-05-15T12:00:00Z".to_owned()),
            },
            series: None,
            next_retro: None,
            participants: Vec::new(),
            columns: vec![RetroColumnRecord {
                id: column_id,
                retro_id,
                column_key: "pain".to_owned(),
                title: "Pain".to_owned(),
                position: 0,
                order_direction: "desc".to_owned(),
                accent_color: None,
                cards: vec![group_card],
            }],
            ready: Default::default(),
            voting: VotingInfo {
                vote_limit: 3,
                votes_used: 0,
                votes_remaining: 3,
            },
            clusters: Vec::new(),
            actions: Vec::new(),
            deck: Vec::new(),
            ai_artifacts: Vec::new(),
            meeting_notes: Vec::new(),
            deliveries: Vec::new(),
        };

        let prompt = build_prompt(&board);

        assert!(prompt.contains("## Pain (2 cards)"), "{prompt}");
        assert!(
            prompt.contains("- Cluster: Grouped: platform friction (1 votes on group)"),
            "{prompt}"
        );
        assert!(
            prompt.contains("  - (4 votes) Deploys are slow"),
            "{prompt}"
        );
        assert!(
            prompt.contains("  - (2 votes) Staging flakes during QA"),
            "{prompt}"
        );
        assert!(!prompt.contains("Hidden draft"), "{prompt}");
    }

    #[test]
    fn build_prompt_includes_notes_and_excludes_rejected_actions() {
        let retro_id = uuid(1);
        let column_id = uuid(2);
        let author_id = uuid(3);
        let board = RetroBoard {
            retro: retro(retro_id),
            series: None,
            next_retro: None,
            participants: Vec::new(),
            columns: vec![RetroColumnRecord {
                id: column_id,
                retro_id,
                column_key: "wins".to_owned(),
                title: "Wins".to_owned(),
                position: 0,
                order_direction: "desc".to_owned(),
                accent_color: None,
                cards: vec![card(retro_id, column_id, author_id, "Launch went well", 2)],
            }],
            ready: Default::default(),
            voting: VotingInfo {
                vote_limit: 3,
                votes_used: 0,
                votes_remaining: 3,
            },
            clusters: Vec::new(),
            actions: vec![
                action(9, retro_id, "Follow up with support", "confirmed"),
                action(10, retro_id, "Rewrite everything", "rejected"),
            ],
            deck: Vec::new(),
            ai_artifacts: Vec::new(),
            meeting_notes: vec![meeting_note(
                retro_id,
                author_id,
                "Facilitator",
                "Support handoff stayed unclear.",
            )],
            deliveries: Vec::new(),
        };

        let prompt = build_prompt(&board);

        assert!(prompt.contains("## Committed actions (1)"), "{prompt}");
        assert!(
            prompt.contains("- [confirmed] Follow up with support"),
            "{prompt}"
        );
        assert!(!prompt.contains("Rewrite everything"), "{prompt}");
        assert!(prompt.contains("## Meeting notes (1)"), "{prompt}");
        assert!(
            prompt.contains("- Facilitator: Support handoff stayed unclear."),
            "{prompt}"
        );
    }

    #[test]
    fn build_prompt_formats_media_alt_text_as_evidence() {
        let retro_id = uuid(1);
        let column_id = uuid(2);
        let author_id = uuid(3);
        let mut media_only = card(retro_id, column_id, author_id, "", 3);
        media_only.gif_alt_text = Some("confused Travolta looking around".to_owned());
        let mut text_with_media = card(retro_id, column_id, author_id, "Release felt risky", 1);
        text_with_media.gif_alt_text = Some("fire alarm gif".to_owned());
        let board = RetroBoard {
            retro: retro(retro_id),
            series: None,
            next_retro: None,
            participants: Vec::new(),
            columns: vec![RetroColumnRecord {
                id: column_id,
                retro_id,
                column_key: "risks".to_owned(),
                title: "Risks".to_owned(),
                position: 0,
                order_direction: "desc".to_owned(),
                accent_color: None,
                cards: vec![media_only, text_with_media],
            }],
            ready: Default::default(),
            voting: VotingInfo {
                vote_limit: 3,
                votes_used: 0,
                votes_remaining: 3,
            },
            clusters: Vec::new(),
            actions: Vec::new(),
            deck: Vec::new(),
            ai_artifacts: Vec::new(),
            meeting_notes: Vec::new(),
            deliveries: Vec::new(),
        };

        let prompt = build_prompt(&board);

        assert!(
            prompt.contains("- (3 votes) [media: confused Travolta looking around]"),
            "{prompt}"
        );
        assert!(
            prompt.contains("- (1 votes) Release felt risky [media: fire alarm gif]"),
            "{prompt}"
        );
    }

    fn retro(id: Uuid) -> RetroRecord {
        RetroRecord {
            id,
            title: "Sprint 42".to_owned(),
            phase: "completed".to_owned(),
            vote_limit: 3,
            action_discussion_limit: 3,
            creator_email: "host@example.com".to_owned(),
            planned_for: "2026-05-15".to_owned(),
            happened_at: Some("2026-05-15T12:00:00Z".to_owned()),
        }
    }

    fn card(
        retro_id: Uuid,
        column_id: Uuid,
        author_participant_id: Uuid,
        body_text: &str,
        vote_count: i64,
    ) -> CardRecord {
        CardRecord {
            id: uuid(5),
            retro_id,
            column_id,
            author_participant_id,
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
            vote_count,
            current_user_vote_count: 0,
            hidden: false,
            cluster_members: Vec::new(),
        }
    }

    fn cluster_member(
        id: u128,
        author_participant_id: Uuid,
        body_text: &str,
        vote_count: i64,
        hidden: bool,
    ) -> ClusterMemberRecord {
        ClusterMemberRecord {
            id: uuid(id),
            author_participant_id,
            body_text: Some(body_text.to_owned()),
            gif_url: None,
            gif_alt_text: None,
            vote_count,
            hidden,
        }
    }

    fn action(id: u128, retro_id: Uuid, title: &str, status: &str) -> ActionItemRecord {
        ActionItemRecord {
            id: uuid(id),
            retro_id,
            source_card_id: None,
            source_cluster_id: None,
            title: title.to_owned(),
            details: None,
            status: status.to_owned(),
            position: 0,
            tags: Vec::new(),
        }
    }

    fn meeting_note(
        retro_id: Uuid,
        author_participant_id: Uuid,
        title: &str,
        body_text: &str,
    ) -> MeetingNoteRecord {
        MeetingNoteRecord {
            id: uuid(11),
            retro_id,
            author_participant_id,
            title: title.to_owned(),
            body_text: body_text.to_owned(),
        }
    }

    fn uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }
}
