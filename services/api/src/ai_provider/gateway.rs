//! HTTP gateway provider with audience-bound ID-token auth.
//!
//! Talks to an LLM gateway over HTTP: `POST {prompt_text: "…"}` →
//! `{response: "…"}`. Before each request the provider mints an
//! audience-bound Google ID token via the GCE metadata server and
//! sends it as `Authorization: Bearer …` — matching the existing
//! IAP auth pattern in `apps/web/app/lib/directory.ts`.
//!
//! The audience is the scheme + host of `SPILLIO_AI_GATEWAY_URL`, so
//! configuration stays a single env var. Local environments without a
//! reachable metadata server (laptops, CI, anything off GCP) return
//! [`AiError::Identity`], which the artifact runner records as a
//! normal failure — no special error path.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;

use super::AiError;

/// Endpoint that returns an audience-bound ID token for the runtime
/// service account. Same URL the directory client uses via
/// `gcp-metadata`; we call it directly here to avoid pulling in a
/// full Google client library for one HTTP GET.
const METADATA_IDENTITY_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/identity";

/// Budget for a gateway call. Every caller runs in a background job
/// (summary, auto-clustering, next-title), and the upstream model
/// (gaas / Gemini) routinely takes well over a handful of seconds, so
/// this is generous rather than request-path tight.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct GatewayProvider {
    /// Fully-formed endpoint URL — what we POST to.
    endpoint: String,
    /// Scheme + host of `endpoint`. Pre-computed at construction
    /// because every call needs it as the ID-token audience.
    audience: String,
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

        let audience = match audience_from(&endpoint) {
            Some(audience) => audience,
            None => {
                tracing::warn!(
                    endpoint = %endpoint,
                    "SPILLIO_AI_GATEWAY_URL is not a valid absolute URL; AI provider disabled"
                );
                return None;
            }
        };

        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("reqwest client build cannot fail with default config");

        Some(Self {
            endpoint,
            audience,
            http,
        })
    }

    pub async fn complete(&self, prompt: &str) -> Result<String, AiError> {
        let token = self.fetch_id_token().await?;
        let resp = self
            .http
            .post(&self.endpoint)
            .bearer_auth(token)
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

    async fn fetch_id_token(&self) -> Result<String, AiError> {
        let resp = self
            .http
            .get(METADATA_IDENTITY_URL)
            // Required by the metadata server; without it the server
            // refuses the request as a defence against SSRF.
            .header("Metadata-Flavor", "Google")
            .query(&[("audience", self.audience.as_str())])
            .send()
            .await
            .map_err(|e| AiError::Identity(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(AiError::Identity(format!(
                "metadata server returned {status}"
            )));
        }
        resp.text()
            .await
            .map_err(|e| AiError::Identity(e.to_string()))
    }
}

/// Derive the audience claim from the endpoint URL: scheme + host (+
/// non-default port). Anything past the host is irrelevant to the
/// token verifier on the other end.
fn audience_from(endpoint: &str) -> Option<String> {
    let parsed = Url::parse(endpoint).ok()?;
    let host = parsed.host_str()?;
    let mut audience = format!("{}://{}", parsed.scheme(), host);
    if let Some(port) = parsed.port() {
        audience.push(':');
        audience.push_str(&port.to_string());
    }
    Some(audience)
}

#[cfg(test)]
mod tests {
    use super::audience_from;

    #[test]
    fn audience_strips_path_query_and_fragment() {
        assert_eq!(
            audience_from("https://example.run.app/some/path?x=1#frag").as_deref(),
            Some("https://example.run.app"),
        );
    }

    #[test]
    fn audience_preserves_explicit_port() {
        assert_eq!(
            audience_from("https://example.run.app:8443/foo").as_deref(),
            Some("https://example.run.app:8443"),
        );
    }

    #[test]
    fn audience_drops_default_port() {
        assert_eq!(
            audience_from("https://example.run.app:443/foo").as_deref(),
            Some("https://example.run.app"),
        );
    }

    #[test]
    fn audience_rejects_relative_url() {
        assert!(audience_from("/just/a/path").is_none());
    }
}
