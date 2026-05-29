//! AI provider abstraction.
//!
//! Spill talks to "an AI provider" through a single small surface
//! (`AiProvider::complete`) so use-case code stays vendor-agnostic.
//! Today we ship one shape — a gateway provider that POSTs the prompt
//! to an external HTTP endpoint (see [`gateway`]). The enum exists so
//! future providers (direct OpenAI, direct Anthropic, a stub for tests)
//! drop in without touching call sites.
//!
//! All providers are configured by `SPILLIO_AI_*` env vars; if nothing
//! is configured, [`AiProvider::from_env`] returns `None` and AI-flavoured
//! routes degrade to `503 Service Unavailable`. The rest of the API
//! keeps working.

mod gateway;
mod fake;

pub use gateway::GatewayProvider;
pub use fake::FakeProvider;

use std::env;

/// Concrete AI provider. Each variant owns a self-contained client.
#[derive(Clone)]
pub enum AiProvider {
    /// HTTP gateway in front of an upstream model.
    Gateway(GatewayProvider),
    /// Deterministic stub used by tests and local development.
    Fake(FakeProvider),
}

/// Errors any provider can surface. Stays narrow on purpose: callers
/// translate to HTTP without leaking provider internals.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("ai provider: transport error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("ai provider: upstream returned {status}: {body}")]
    Upstream {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl AiProvider {
    /// Build a provider from `SPILLIO_AI_*` env vars.
    ///
    /// Resolution order:
    /// 1. `SPILLIO_AI_PROVIDER=gateway` (or unset, with `SPILLIO_AI_GATEWAY_URL` set)
    ///    → [`GatewayProvider`] from `SPILLIO_AI_GATEWAY_URL` (a fully-formed endpoint URL).
    /// 2. Anything else → `None`. Callers should return 503.
    pub fn from_env() -> Option<Self> {
        let configured = env::var("SPILLIO_AI_PROVIDER")
            .ok()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty());

        match configured.as_deref() {
            Some("gateway") | None => GatewayProvider::from_env().map(Self::Gateway),
            Some(_) => None,
        }
    }

    /// Generate a free-text completion for the given prompt.
    pub async fn complete(&self, prompt: &str) -> Result<String, AiError> {
        match self {
            Self::Gateway(provider) => provider.complete(prompt).await,
            Self::Fake(provider) => provider.complete(prompt).await,
        }
    }
}
