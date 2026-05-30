use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::identity::{AccessModel, CurrentUser};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

#[derive(Serialize)]
pub struct SessionResponse {
    pub user: CurrentUser,
    pub access_model: AccessModel,
}

#[derive(Deserialize)]
pub struct CreateRetroRequest {
    pub title: String,
    pub scheduled_at: Option<String>,
    pub template: String,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub column_colors: Vec<String>,
    #[serde(default = "default_vote_limit")]
    pub vote_limit: i32,
    #[serde(default = "default_action_discussion_limit")]
    pub action_discussion_limit: i32,
    #[serde(default)]
    pub invitees: Vec<InviteeRequest>,
    pub clustering_mode: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateDraftCardRequest {
    pub column_id: Uuid,
    pub body_text: Option<String>,
    pub gif_url: Option<String>,
    pub gif_alt_text: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateDraftCardRequest {
    pub body_text: Option<String>,
    pub gif_url: Option<String>,
    pub gif_alt_text: Option<String>,
    pub cluster_details: Option<String>,
}

#[derive(Deserialize)]
pub struct MoveDraftCardRequest {
    pub column_id: Uuid,
    pub before_card_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ClusterCardsRequest {
    pub target_card_id: Uuid,
}

#[derive(Deserialize)]
pub struct CastVoteRequest {
    pub card_id: Uuid,
    #[serde(default = "default_vote_count")]
    pub count: i32,
}

#[derive(Deserialize)]
pub struct UpdateActionRequest {
    pub title: String,
    pub details: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRetroMetadataRequest {
    pub title: String,
    pub scheduled_at: Option<String>,
    pub cover_gif_url: Option<String>,
    pub cover_gif_alt_text: Option<String>,
}

#[derive(Deserialize)]
pub struct CloneRetroRequest {
    pub title: Option<String>,
    pub scheduled_at: Option<String>,
    #[serde(default)]
    pub suggest_title: bool,
}

#[derive(Deserialize)]
pub struct IngestItemRequest {
    pub source: String,
    pub placement: String,
    pub target_column_id: Option<Uuid>,
    pub suggested_text: Option<String>,
    pub gif_url: Option<String>,
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub source_metadata: serde_json::Value,
    #[serde(default)]
    pub raw_payload: serde_json::Value,
}

#[derive(Deserialize)]
pub struct AcceptDeckItemRequest {
    pub column_id: Uuid,
}

#[derive(Deserialize)]
pub struct StartAiJobRequest {
    pub kind: String,
    #[serde(default)]
    pub fail: bool,
}

#[derive(Deserialize)]
pub struct ApplyTaggingRequest {
    pub artifact_id: Uuid,
}

#[derive(Deserialize)]
pub struct CreateMeetingNoteRequest {
    pub title: Option<String>,
    pub body_text: String,
}

#[derive(Deserialize)]
pub struct CreateDeliveryRequest {
    pub kind: String,
    #[serde(default)]
    pub fail: bool,
}

#[derive(Deserialize)]
pub struct GifSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub page: usize,
    pub kind: Option<String>,
}

#[derive(Serialize)]
pub struct GifSearchResponse {
    pub results: Vec<GifResult>,
    pub degraded: bool,
}

#[derive(Serialize)]
pub struct GifResult {
    pub id: String,
    pub url: String,
    pub preview_url: String,
    pub alt_text: String,
    pub media_type: String,
    pub kind: String,
}

fn default_vote_count() -> i32 {
    1
}

fn default_vote_limit() -> i32 {
    3
}

fn default_action_discussion_limit() -> i32 {
    3
}

fn default_member_role() -> String {
    "member".to_owned()
}

#[derive(Deserialize)]
pub struct AddGrantRequest {
    pub email: String,
    #[serde(default = "default_member_role")]
    pub role: String,
}

#[derive(Deserialize)]
pub struct RemoveGrantRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct InviteeRequest {
    pub email: String,
    #[serde(default = "default_member_role")]
    pub role: String,
}

#[derive(Deserialize, Default)]
pub struct RevealBoardRequest {
    #[serde(default)]
    pub force: bool,
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn frontend_contract_mirrors_core_rust_contract_names() {
        let frontend_contracts = fs::read_to_string("../../apps/web/app/lib/contracts.ts")
            .expect("frontend contract file");
        for contract_name in [
            "RetroPhase",
            "RetroSummary",
            "RetroOverview",
            "RetroColumn",
            "RetroCard",
            "GifResult",
            "GifSearchResponse",
            "RetroBoard",
            "RetroActionItem",
            "IngestedItem",
            "AiArtifact",
            "MeetingNote",
            "Delivery",
            "CreateRetroPayload",
        ] {
            assert!(
                frontend_contracts.contains(&format!("export type {contract_name}")),
                "frontend contract is missing {contract_name}"
            );
        }
    }

    #[test]
    fn frontend_contract_mirrors_core_enum_values_and_payload_fields() {
        let frontend_contracts = fs::read_to_string("../../apps/web/app/lib/contracts.ts")
            .expect("frontend contract file");

        assert_contract_contains(
            &frontend_contracts,
            "RetroPhase",
            &[
                "\"writing\"",
                "\"discussion\"",
                "\"voting\"",
                "\"action_discussion\"",
                "\"completed\"",
            ],
        );
        assert_contract_contains(
            &frontend_contracts,
            "GifResult",
            &[
                "id: string",
                "url: string",
                "preview_url: string",
                "alt_text: string",
                "media_type: \"image\" | \"video\"",
                "kind: \"all\" | \"gif\" | \"sticker\" | \"clip\"",
            ],
        );
        assert_contract_contains(
            &frontend_contracts,
            "RetroBoard",
            &[
                "retro:",
                "scheduled_at: string | null",
                "completed_at: string | null",
                "clustering_status:",
                "columns: RetroColumn[]",
                "ready:",
                "voting:",
                "clusters:",
                "actions: RetroActionItem[]",
                "deck: IngestedItem[]",
                "ai_artifacts: AiArtifact[]",
                "meeting_notes: MeetingNote[]",
                "deliveries: Delivery[]",
            ],
        );
        assert_contract_contains(
            &frontend_contracts,
            "RetroSummary",
            &[
                "scheduled_at: string | null",
                "completed_at: string | null",
                "cover_gif_url: string | null",
                "team_mood: string | null",
            ],
        );
        assert_contract_contains(
            &frontend_contracts,
            "RetroOverview",
            &["retros: RetroSummary[]"],
        );
        assert_contract_contains(
            &frontend_contracts,
            "CreateRetroPayload",
            &["scheduled_at?: string | null"],
        );
    }

    fn assert_contract_contains(source: &str, contract_name: &str, expected: &[&str]) {
        for value in expected {
            assert!(
                source.contains(value),
                "frontend {contract_name} contract is missing {value}"
            );
        }
    }
}
