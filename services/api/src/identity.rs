use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::HeaderMap;
use axum::http::request::Parts;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ApiError;

pub(crate) const HEADER_USER_NAME: &str = "x-spillio-user-name";
/// Dev/local identity header: the email the request acts for. Trusted **only**
/// in local mode; in token mode identity comes from the signed token, never a
/// header. Fixed name so the web tier, CLI, and API cannot drift.
pub(crate) const HEADER_ON_BEHALF_OF: &str = "x-spillio-on-behalf-of";
/// Fixed marker subprotocol the browser offers alongside the token, so the
/// server can echo a stable value on the handshake while the token rides as a
/// second `Sec-WebSocket-Protocol` entry (browsers cannot set request headers
/// on a WebSocket handshake).
pub(crate) const WS_SUBPROTOCOL: &str = "spillio.ws.v1";

#[derive(Clone, Default)]
pub struct LinkAccessPolicy;

impl LinkAccessPolicy {
    pub fn can_edit_retro_link(&self, retro_id: &str) -> bool {
        !retro_id.trim().is_empty()
    }
}

#[derive(Serialize, Clone)]
pub struct CurrentUser {
    pub subject: String,
    pub email: String,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct AccessModel {
    pub kind: &'static str,
    pub can_edit_with_link: bool,
}

/// Stable participant subject derived from the email. Mirrors the web tier's
/// `subjectForEmail` so the derived subject matches `participants.external_subject`.
pub fn subject_for_email(email: &str) -> String {
    let normalized = email.trim().to_lowercase();
    let digest = Sha256::digest(normalized.as_bytes());
    format!("email:{digest:x}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthMode {
    /// Dev/local: trust the on-behalf-of header directly (no token).
    Local,
    /// Deployed: require a first-party token signed with the shared secret.
    Token,
}

/// Shared authentication configuration. Cloned cheaply (Arc).
///
/// The web tier (behind IAP) is the sole token minter: it vouches for the
/// IAP-authenticated user by signing a short-lived HS256 token the API verifies
/// with the shared secret. There is one mechanism, no OIDC/JWKS/service-account
/// dependency, and identity is bound into the token rather than a spoofable
/// header.
#[derive(Clone)]
pub struct AuthState(Arc<AuthInner>);

struct AuthInner {
    mode: AuthMode,
    secret: Option<String>,
    config_error: Option<String>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self(Arc::new(AuthInner {
            mode: AuthMode::Local,
            secret: None,
            config_error: None,
        }))
    }
}

impl AuthState {
    /// Build from environment. Vendor-neutral: the only secret is the shared
    /// token-signing key; the mode is `local` (dev) or `token` (deployed).
    pub fn from_env() -> Self {
        let secret = env_nonempty("SPILLIO_TOKEN_SECRET");
        let configured_mode = env_nonempty("SPILLIO_AUTH_MODE");
        let config_error = invalid_auth_mode(configured_mode.as_deref(), secret.is_some())
            .or_else(|| implicit_local_auth_error(configured_mode.as_deref(), secret.is_some()));
        let mode = resolve_mode_value(configured_mode.as_deref(), secret.is_some());
        Self(Arc::new(AuthInner {
            mode,
            secret,
            config_error,
        }))
    }

    /// True when running without token verification. Used by the startup guard.
    pub fn is_local(&self) -> bool {
        self.0.mode == AuthMode::Local
    }

    /// Returns a config error when token mode is missing the signing secret.
    /// Used by the startup guard to fail fast instead of failing every request.
    pub fn config_error(&self) -> Option<String> {
        if let Some(error) = &self.0.config_error {
            return Some(error.clone());
        }
        if self.0.mode == AuthMode::Token && self.0.secret.is_none() {
            Some("token mode requires SPILLIO_TOKEN_SECRET".to_string())
        } else {
            None
        }
    }

    async fn authenticate(&self, headers: &HeaderMap) -> Result<CurrentUser, ApiError> {
        match self.0.mode {
            AuthMode::Local => from_on_behalf_of(headers),
            AuthMode::Token => {
                let token = bearer_token(headers)
                    .ok_or_else(|| ApiError::unauthorized("missing bearer token"))?;
                let claims = self.verify_token(token)?;
                if claims.retro.is_some() {
                    return Err(ApiError::unauthorized(
                        "board-scoped ws token cannot authenticate REST requests",
                    ));
                }
                user_from_claims(claims)
            }
        }
    }

    /// Verify a first-party, short-lived token (HS256, signed with the shared
    /// secret). Used for both REST (Authorization header) and WS (subprotocol).
    pub fn verify_token(&self, token: &str) -> Result<TokenClaims, ApiError> {
        let secret = self
            .0
            .secret
            .as_deref()
            .ok_or_else(|| ApiError::internal("auth: SPILLIO_TOKEN_SECRET not configured"))?;
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_aud = false;
        let data = decode::<TokenClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map_err(|_| ApiError::unauthorized("invalid token"))?;
        Ok(data.claims)
    }

    /// Whether connections must present a token. Tokenless connections are only
    /// accepted in local/dev mode.
    pub fn ws_token_required(&self) -> bool {
        self.0.mode == AuthMode::Token
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthState::from_ref(state);
        auth.authenticate(&parts.headers).await
    }
}

/// Claims carried by a first-party token. `retro` is present only on
/// board-scoped (WS) tokens; REST/CLI tokens omit it.
#[derive(Deserialize)]
pub struct TokenClaims {
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub retro: Option<String>,
    #[allow(dead_code)]
    pub exp: usize,
}

fn resolve_mode_value(configured: Option<&str>, secret_present: bool) -> AuthMode {
    if secret_present {
        return AuthMode::Token;
    }
    match configured.map(|v| v.trim().to_lowercase()).as_deref() {
        Some("local") => AuthMode::Local,
        Some("token") => AuthMode::Token,
        _ => AuthMode::Local,
    }
}

fn invalid_auth_mode(configured: Option<&str>, secret_present: bool) -> Option<String> {
    match configured
        .map(|value| value.trim().to_lowercase())
        .as_deref()
    {
        None | Some("local") | Some("token") => None,
        Some("proxy") if secret_present => None,
        Some(value) => Some(format!(
            "SPILLIO_AUTH_MODE must be 'local' or 'token', got '{value}'"
        )),
    }
}

fn implicit_local_auth_error(configured: Option<&str>, secret_present: bool) -> Option<String> {
    if secret_present {
        return None;
    }
    match configured
        .map(|value| value.trim().to_lowercase())
        .as_deref()
    {
        Some("local") | Some("token") => None,
        _ => Some("local auth requires explicit SPILLIO_AUTH_MODE=local".to_string()),
    }
}

/// Build the acting user from verified token claims. The subject is always
/// derived from the email; the token never dictates it.
fn user_from_claims(claims: TokenClaims) -> Result<CurrentUser, ApiError> {
    let email = claims.email.trim().to_lowercase();
    if !email.contains('@') {
        return Err(ApiError::unauthorized("token missing a valid email"));
    }
    let display_name = claims
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| display_name_from_email(&email));
    Ok(CurrentUser {
        subject: subject_for_email(&email),
        email,
        display_name,
    })
}

/// Resolve the acting user from the on-behalf-of header (local/dev only). The
/// subject is always derived from the email; the client never dictates it.
fn from_on_behalf_of(headers: &HeaderMap) -> Result<CurrentUser, ApiError> {
    let email = header_value(headers, HEADER_ON_BEHALF_OF)
        .ok_or_else(|| ApiError::unauthorized("missing on-behalf-of header"))?
        .to_lowercase();
    if !email.contains('@') {
        return Err(ApiError::unauthorized("invalid on-behalf-of email"));
    }
    let display_name =
        header_value(headers, HEADER_USER_NAME).unwrap_or_else(|| display_name_from_email(&email));
    Ok(CurrentUser {
        subject: subject_for_email(&email),
        email,
        display_name,
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// Extract the WS token offered as a `Sec-WebSocket-Protocol` entry alongside
/// the `WS_SUBPROTOCOL` marker. Browsers cannot set request headers on a
/// WebSocket handshake, so the token rides as a negotiated subprotocol.
pub fn ws_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(axum::http::header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|value| value.to_str().ok())?;
    raw.split(',')
        .map(str::trim)
        .find(|entry| !entry.is_empty() && *entry != WS_SUBPROTOCOL)
        .map(ToOwned::to_owned)
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn display_name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email).trim();
    if local.is_empty() {
        "Spill user".to_string()
    } else {
        local.to_string()
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
impl AuthState {
    /// Build a token-mode AuthState with an explicit secret (no env, no network).
    pub fn token_test(secret: Option<&str>) -> Self {
        Self(Arc::new(AuthInner {
            mode: AuthMode::Token,
            secret: secret.map(str::to_string),
            config_error: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};

    fn mint(
        secret: &str,
        email: &str,
        name: Option<&str>,
        retro: Option<&str>,
        exp: usize,
    ) -> String {
        #[derive(Serialize)]
        struct Claims<'a> {
            email: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            retro: Option<&'a str>,
            exp: usize,
        }
        encode(
            &Header::new(Algorithm::HS256),
            &Claims {
                email,
                name,
                retro,
                exp,
            },
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    fn far_future() -> usize {
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600) as usize
    }

    #[test]
    fn subject_for_email_is_stable_and_normalized() {
        let a = subject_for_email("Ava@Example.com");
        let b = subject_for_email("  ava@example.com ");
        assert_eq!(a, b);
        assert!(a.starts_with("email:"));
        assert_eq!(a.len(), "email:".len() + 64);
    }

    #[test]
    fn token_round_trips_with_claims() {
        let auth = AuthState::token_test(Some("s3cret"));
        let token = mint(
            "s3cret",
            "ava@example.com",
            Some("Ava"),
            Some("retro-1"),
            far_future(),
        );
        let claims = auth.verify_token(&token).unwrap();
        assert_eq!(claims.email, "ava@example.com");
        assert_eq!(claims.name.as_deref(), Some("Ava"));
        assert_eq!(claims.retro.as_deref(), Some("retro-1"));
    }

    #[test]
    fn token_rejects_wrong_secret_and_expiry() {
        let auth = AuthState::token_test(Some("right"));
        let wrong = mint("wrong", "ava@example.com", None, None, far_future());
        assert!(auth.verify_token(&wrong).is_err());
        let expired = mint("right", "ava@example.com", None, None, 1);
        assert!(auth.verify_token(&expired).is_err());
    }

    #[test]
    fn token_without_secret_configured_is_internal_error() {
        let auth = AuthState::token_test(None);
        let token = mint("s3cret", "ava@example.com", None, None, far_future());
        assert!(auth.verify_token(&token).is_err());
    }

    #[tokio::test]
    async fn token_mode_yields_identity_from_claims() {
        let auth = AuthState::token_test(Some("s3cret"));
        let token = mint("s3cret", "Ava@Example.com", Some("Ava"), None, far_future());
        let mut headers = HeaderMap::new();
        headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
        // A spoofed on-behalf-of must be ignored — identity comes from the token.
        headers.insert(HEADER_ON_BEHALF_OF, "attacker@example.com".parse().unwrap());
        let user = auth.authenticate(&headers).await.unwrap();
        assert_eq!(user.email, "ava@example.com");
        assert_eq!(user.display_name, "Ava");
        assert_eq!(user.subject, subject_for_email("ava@example.com"));
    }

    #[tokio::test]
    async fn token_mode_rejects_ws_scoped_token_for_rest() {
        let auth = AuthState::token_test(Some("s3cret"));
        let token = mint(
            "s3cret",
            "Ava@Example.com",
            Some("Ava"),
            Some("retro-1"),
            far_future(),
        );
        let mut headers = HeaderMap::new();
        headers.insert("authorization", format!("Bearer {token}").parse().unwrap());

        assert!(auth.authenticate(&headers).await.is_err());
    }

    #[tokio::test]
    async fn token_mode_rejects_header_only_request() {
        let auth = AuthState::token_test(Some("s3cret"));
        let mut headers = HeaderMap::new();
        headers.insert(HEADER_ON_BEHALF_OF, "ava@example.com".parse().unwrap());
        assert!(auth.authenticate(&headers).await.is_err());
    }

    #[tokio::test]
    async fn ws_token_required_only_in_token_mode() {
        assert!(AuthState::token_test(Some("s")).ws_token_required());
        assert!(!AuthState::default().ws_token_required());
    }

    #[test]
    fn token_mode_requires_secret() {
        assert!(AuthState::token_test(None).config_error().is_some());
        assert!(AuthState::default().config_error().is_none());
    }

    #[test]
    fn token_secret_forces_token_mode_even_when_env_says_local() {
        assert_eq!(resolve_mode_value(Some("local"), true), AuthMode::Token);
        assert_eq!(resolve_mode_value(Some("proxy"), true), AuthMode::Token);
        assert_eq!(resolve_mode_value(None, true), AuthMode::Token);
        assert_eq!(resolve_mode_value(Some("local"), false), AuthMode::Local);
    }

    #[test]
    fn local_auth_requires_explicit_opt_in_without_secret() {
        assert!(implicit_local_auth_error(None, false).is_some());
        assert!(implicit_local_auth_error(Some("local"), false).is_none());
        assert!(implicit_local_auth_error(Some("token"), false).is_none());
        assert!(implicit_local_auth_error(None, true).is_none());
    }

    #[test]
    fn unknown_configured_auth_mode_is_config_error() {
        assert!(invalid_auth_mode(Some("proxy"), false).is_some());
        assert!(invalid_auth_mode(Some("proxy"), true).is_none());
        assert!(invalid_auth_mode(Some(" LOCAL "), false).is_none());
        assert!(invalid_auth_mode(Some("token"), false).is_none());
    }
}
