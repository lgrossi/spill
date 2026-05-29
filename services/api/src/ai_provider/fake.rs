//! Deterministic AI provider used by tests and any caller that
//! wants a stub completion without standing up a real HTTP gateway.
//!
//! Two flavours:
//!   - `Success("text")` — every call returns the same text.
//!   - `Failure("msg")`  — every call returns [`AiError::Upstream`]
//!     with status 500 and the given message as the body.
//!
//! Kept intentionally trivial: no scripted multi-call responses, no
//! per-prompt routing. Tests that need richer behaviour can
//! construct their own [`AiProvider::Fake`] variant inline.

use reqwest::StatusCode;

use super::AiError;

#[derive(Clone)]
pub enum FakeProvider {
    Success(String),
    Failure(String),
}

impl FakeProvider {
    pub fn responding_with(text: impl Into<String>) -> Self {
        Self::Success(text.into())
    }

    pub fn failing_with(message: impl Into<String>) -> Self {
        Self::Failure(message.into())
    }

    pub async fn complete(&self, _prompt: &str) -> Result<String, AiError> {
        match self {
            Self::Success(text) => Ok(text.clone()),
            Self::Failure(message) => Err(AiError::Upstream {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: message.clone(),
            }),
        }
    }
}
