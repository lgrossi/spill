use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetroPhase {
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
            (Self::Writing, Self::Discussion)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardState {
    Draft,
    Revealed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnOrder {
    Chronological,
    ReverseChronological,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retro(
    pub Uuid,
    pub NonEmptyText,
    pub RetroPhase,
    pub RetroSettings,
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroSettings {
    pub vote_limit: VoteLimit,
    pub action_discussion_limit: ActionDiscussionLimit,
}

impl Default for RetroSettings {
    fn default() -> Self {
        Self {
            vote_limit: VoteLimit::default(),
            action_discussion_limit: ActionDiscussionLimit::default(),
        }
    }
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
pub struct Participant(pub Uuid, pub NonEmptyText, pub ParticipantRole);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRole {
    Host,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroColumn(pub Uuid, pub NonEmptyText, pub ColumnOrder);

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionItem(pub Uuid, pub NonEmptyText, pub ActionStatus);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Proposed,
    Confirmed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestedItem(pub Uuid, pub Uuid, pub IngestedItemPlacement, pub CardBody);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestedItemPlacement {
    UserDeck,
    RetroDraft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiArtifact(pub Uuid, pub AiArtifactKind, pub JobStatus);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonEmptyText(String);

impl NonEmptyText {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or(DomainError::EmptyText)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainError {
    EmptyCardBody,
    EmptyText,
    EmptyVote,
    InvalidActionDiscussionLimit(u16),
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
        let phase = RetroPhase::Writing
            .transition_to(RetroPhase::Discussion)
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
