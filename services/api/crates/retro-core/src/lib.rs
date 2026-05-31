use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! domain_string_enum {
    ($name:ident, $domain:literal, { $($variant:ident => $value:literal),+ $(,)? }) => {
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)+
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = DomainError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    other => Err(DomainError::InvalidDomainValue {
                        domain: $domain,
                        value: other.to_owned(),
                    }),
                }
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetroPhase {
    Scheduled,
    Writing,
    Discussion,
    Voting,
    ActionDiscussion,
    Completed,
}

impl RetroPhase {
    pub fn transition_to(self, next: Self) -> Result<Self, DomainError> {
        matches!(
            (self, next),
            (Self::Scheduled, Self::Writing)
                | (Self::Writing, Self::Discussion)
                | (Self::Discussion, Self::Voting)
                | (Self::Voting, Self::ActionDiscussion)
                | (Self::ActionDiscussion, Self::Completed)
        )
        .then_some(next)
        .ok_or(DomainError::InvalidPhaseTransition {
            from: self,
            to: next,
        })
    }

    pub fn supports_ready(self) -> bool {
        matches!(self, Self::Writing | Self::Voting)
    }
}

domain_string_enum!(RetroPhase, "retro_phase", {
    Scheduled => "scheduled",
    Writing => "writing",
    Discussion => "discussion",
    Voting => "voting",
    ActionDiscussion => "action_discussion",
    Completed => "completed",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardState {
    Draft,
    Revealed,
}

domain_string_enum!(CardState, "card_state", {
    Draft => "draft",
    Revealed => "revealed",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnOrder {
    Chronological,
    ReverseChronological,
}

domain_string_enum!(ColumnOrder, "column_order", {
    Chronological => "chronological",
    ReverseChronological => "reverse_chronological",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RetroSettings {
    pub vote_limit: VoteLimit,
    pub action_discussion_limit: ActionDiscussionLimit,
}

macro_rules! positive_count {
    ($name:ident, $error:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(u16);
        impl $name {
            pub const DEFAULT: u16 = 3;
            pub fn new(value: u16) -> Result<Self, DomainError> {
                (value > 0)
                    .then_some(Self(value))
                    .ok_or(DomainError::$error(value))
            }
            pub fn get(self) -> u16 {
                self.0
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self(Self::DEFAULT)
            }
        }
    };
}

positive_count!(VoteLimit, InvalidVoteLimit);
positive_count!(ActionDiscussionLimit, InvalidActionDiscussionLimit);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: Uuid,
    pub column_id: Uuid,
    pub author_id: Uuid,
    pub body: CardBody,
    pub state: CardState,
}

impl Card {
    pub fn new_draft(id: Uuid, column_id: Uuid, author_id: Uuid, body: CardBody) -> Self {
        Self {
            id,
            column_id,
            author_id,
            body,
            state: CardState::Draft,
        }
    }
    pub fn reveal(&mut self) {
        self.state = CardState::Revealed;
    }
    pub fn is_vote_target(&self) -> bool {
        self.state == CardState::Revealed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardBody {
    pub text: Option<NonEmptyText>,
    pub gif: Option<GifAttachment>,
}

impl CardBody {
    pub fn new(
        text: Option<NonEmptyText>,
        gif: Option<GifAttachment>,
    ) -> Result<Self, DomainError> {
        (text.is_some() || gif.is_some())
            .then_some(Self { text, gif })
            .ok_or(DomainError::EmptyCardBody)
    }

    pub fn from_payload(
        text: Option<String>,
        gif_url: Option<String>,
        gif_alt_text: Option<String>,
    ) -> Result<Self, DomainError> {
        let text = text.map(NonEmptyText::new).transpose()?;
        let gif = gif_url
            .map(|url| {
                Ok(GifAttachment {
                    url: NonEmptyText::new(url)?,
                    alt_text: gif_alt_text.map(NonEmptyText::new).transpose()?,
                })
            })
            .transpose()?;

        Self::new(text, gif)
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_ref().map(NonEmptyText::as_str)
    }

    pub fn gif_url(&self) -> Option<&str> {
        self.gif.as_ref().map(|gif| gif.url.as_str())
    }

    pub fn gif_alt_text(&self) -> Option<&str> {
        self.gif
            .as_ref()
            .and_then(|gif| gif.alt_text.as_ref())
            .map(NonEmptyText::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GifAttachment {
    pub url: NonEmptyText,
    pub alt_text: Option<NonEmptyText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadyMark {
    pub participant_id: Uuid,
    pub phase: RetroPhase,
}

impl ReadyMark {
    pub fn new(participant_id: Uuid, phase: RetroPhase) -> Result<Self, DomainError> {
        phase
            .supports_ready()
            .then_some(Self {
                participant_id,
                phase,
            })
            .ok_or(DomainError::ReadyUnsupportedPhase(phase))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantVotes {
    pub participant_id: Uuid,
    limit: VoteLimit,
    used: u16,
}

impl ParticipantVotes {
    pub fn new(participant_id: Uuid, limit: VoteLimit) -> Self {
        Self {
            participant_id,
            limit,
            used: 0,
        }
    }
    pub fn cast(&mut self, count: u16) -> Result<(), DomainError> {
        if count == 0 {
            return Err(DomainError::EmptyVote);
        }
        let attempted = self.used.saturating_add(count);
        if attempted > self.limit.get() {
            return Err(DomainError::VoteLimitExceeded {
                limit: self.limit.get(),
                attempted,
            });
        }
        self.used = attempted;
        Ok(())
    }
    pub fn remaining(&self) -> u16 {
        self.limit.get() - self.used
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Proposed,
    Confirmed,
    Rejected,
    Done,
}

domain_string_enum!(ActionStatus, "action_status", {
    Proposed => "proposed",
    Confirmed => "confirmed",
    Rejected => "rejected",
    Done => "done",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestedItemPlacement {
    UserDeck,
    RetroDraft,
}

domain_string_enum!(IngestedItemPlacement, "ingested_item_placement", {
    UserDeck => "user_deck",
    RetroDraft => "retro_draft",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionSource {
    Pi,
    ClaudeCode,
    Upload,
    Other,
}

domain_string_enum!(IngestionSource, "ingestion_source", {
    Pi => "pi",
    ClaudeCode => "claude_code",
    Upload => "upload",
    Other => "other",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiArtifactKind {
    GifSuggestions,
    Clustering,
    ActionSuggestions,
    Summary,
    Mood,
    Tagging,
}

domain_string_enum!(AiArtifactKind, "ai_artifact_kind", {
    GifSuggestions => "gif_suggestions",
    Clustering => "clustering",
    ActionSuggestions => "action_suggestions",
    Summary => "summary",
    Mood => "mood",
    Tagging => "tagging",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

domain_string_enum!(JobStatus, "job_status", {
    Pending => "pending",
    Running => "running",
    Succeeded => "succeeded",
    Failed => "failed",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryKind {
    SummaryExport,
    ExternalActionLink,
}

domain_string_enum!(DeliveryKind, "delivery_kind", {
    SummaryExport => "summary_export",
    ExternalActionLink => "external_action_link",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonEmptyText(String);

impl NonEmptyText {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into().trim().to_owned();
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or(DomainError::EmptyText)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    EmptyCardBody,
    EmptyText,
    EmptyVote,
    InvalidActionDiscussionLimit(u16),
    InvalidDomainValue { domain: &'static str, value: String },
    InvalidPhaseTransition { from: RetroPhase, to: RetroPhase },
    InvalidVoteLimit(u16),
    ReadyUnsupportedPhase(RetroPhase),
    VoteLimitExceeded { limit: u16, attempted: u16 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_phase_transitions_that_skip_the_mvp_flow() {
        assert_eq!(
            RetroPhase::Writing.transition_to(RetroPhase::Voting),
            Err(DomainError::InvalidPhaseTransition {
                from: RetroPhase::Writing,
                to: RetroPhase::Voting
            })
        );
    }

    #[test]
    fn accepts_the_ordered_mvp_phase_flow() {
        let phase = RetroPhase::Scheduled
            .transition_to(RetroPhase::Writing)
            .and_then(|phase| phase.transition_to(RetroPhase::Discussion))
            .and_then(|phase| phase.transition_to(RetroPhase::Voting))
            .and_then(|phase| phase.transition_to(RetroPhase::ActionDiscussion))
            .and_then(|phase| phase.transition_to(RetroPhase::Completed))
            .expect("documented MVP phase flow should be valid");
        assert_eq!(phase, RetroPhase::Completed);
    }

    #[test]
    fn ready_marks_are_limited_to_writing_and_voting() {
        assert!(ReadyMark::new(Uuid::nil(), RetroPhase::Writing).is_ok());
        assert!(ReadyMark::new(Uuid::nil(), RetroPhase::Voting).is_ok());
        assert_eq!(
            ReadyMark::new(Uuid::nil(), RetroPhase::Discussion),
            Err(DomainError::ReadyUnsupportedPhase(RetroPhase::Discussion))
        );
    }

    #[test]
    fn card_body_must_have_text_or_gif() {
        assert_eq!(CardBody::new(None, None), Err(DomainError::EmptyCardBody));
        assert!(CardBody::new(Some(NonEmptyText::new("keep this").unwrap()), None).is_ok());
    }

    #[test]
    fn draft_cards_are_not_vote_targets_until_revealed() {
        let body = CardBody::new(Some(NonEmptyText::new("pain point").unwrap()), None).unwrap();
        let mut card = Card::new_draft(Uuid::nil(), Uuid::nil(), Uuid::nil(), body);
        assert!(!card.is_vote_target());
        card.reveal();
        assert!(card.is_vote_target());
    }

    #[test]
    fn vote_limit_rejects_zero_and_caps_cast_votes() {
        assert_eq!(VoteLimit::new(0), Err(DomainError::InvalidVoteLimit(0)));
        let mut votes = ParticipantVotes::new(Uuid::nil(), VoteLimit::new(3).unwrap());
        votes.cast(2).expect("first votes fit within the limit");
        assert_eq!(
            votes.cast(2),
            Err(DomainError::VoteLimitExceeded {
                limit: 3,
                attempted: 4
            })
        );
        assert_eq!(votes.remaining(), 1);
    }
}
