use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// HTTP client for the Spill API. Resolves a bearer token (explicit flag/env,
/// then cached/login token) and transparently re-authenticates once on a 401.
pub struct ApiClient {
    base_url: String,
    web_url: String,
    explicit_token: Option<String>,
    http: Client,
}

impl ApiClient {
    pub fn new(base_url: String, web_url: String, token: Option<String>) -> Result<Self> {
        let explicit_token = token.or_else(|| std::env::var("SPILLIO_API_TOKEN").ok());
        let http = Client::builder().build().context("build http client")?;
        Ok(Self {
            base_url,
            web_url,
            explicit_token,
            http,
        })
    }

    fn token(&self) -> Result<String> {
        match &self.explicit_token {
            Some(token) => Ok(token.clone()),
            None => crate::auth::ensure_token(&self.web_url),
        }
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send::<(), T>(Method::GET, path, None)
    }

    pub fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        self.send(Method::POST, path, Some(body))
    }

    fn send<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        let mut reauthed = false;
        loop {
            let token = self.token()?;
            let url = format!("{}{}", self.base_url, path);
            let mut req = self.http.request(method.clone(), &url).bearer_auth(&token);
            if let Some(body) = body {
                req = req.json(body);
            }
            let resp = req
                .send()
                .with_context(|| format!("{method} {path} (is SPILLIO_API_URL reachable?)"))?;
            let status = resp.status();

            // A cached/login token can expire; clear it and re-auth once.
            if status == StatusCode::UNAUTHORIZED && self.explicit_token.is_none() && !reauthed {
                crate::auth::clear_token()?;
                reauthed = true;
                continue;
            }
            if !status.is_success() {
                let text = resp.text().unwrap_or_default();
                bail!(
                    "{method} {path} -> {}: {}",
                    status.as_u16(),
                    truncate(&text, 300)
                );
            }
            return resp
                .json::<T>()
                .with_context(|| format!("decode response from {path}"));
        }
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}…", &text[..max])
    }
}
