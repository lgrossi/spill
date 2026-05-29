//! HTTP gateway provider.
//!
//! Talks to an LLM gateway over plain HTTP: `POST {prompt_text: "…"}` →
//! `{response: "…"}`. The endpoint URL is injected wholesale via
//! `SPILLIO_AI_GATEWAY_URL`, so this client doesn't know or care what
//! sits behind it.
//!
//! Authentication is intentionally out of scope. When an authenticated
//! upstream is needed, add a second provider variant in [`super`]
//! rather than smuggling auth into this one — keeps each variant's
//! responsibilities small.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::AiError;

/// Default budget. Use-case calls beyond a handful of seconds belong
/// in a background job, not the request path.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone)]
pub struct GatewayProvider {
    /// Fully-formed endpoint URL — what we POST to.
    endpoint: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct CompleteRequest<'a> {
    prompt_text: &'a str,
}

#[derive(Deserialize)]
struct CompleteResponse {
    response: String,
}

impl GatewayProvider {
    /// Build from `SPILLIO_AI_GATEWAY_URL`. Returns `None` when the env
    /// var is unset — local-dev path; the route returns 503.
    pub fn from_env() -> Option<Self> {
        let endpoint = std::env::var("SPILLIO_AI_GATEWAY_URL")
            .ok()
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty())?;

        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("reqwest client build cannot fail with default config");

        Some(Self { endpoint, http })
    }

    pub async fn complete(&self, prompt: &str) -> Result<String, AiError> {
        let resp = self
            .http
            .post(&self.endpoint)
            .json(&CompleteRequest { prompt_text: prompt })
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Upstream { status, body });
        }
        let parsed = resp.json::<CompleteResponse>().await?;
        Ok(parsed.response)
    }
}
