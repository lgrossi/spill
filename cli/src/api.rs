use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// HTTP client for the Spill API. In token/proxy deployments it resolves a
/// bearer token. In explicit local dev mode it sends the on-behalf-of header the
/// local API trusts.
pub struct ApiClient {
    base_url: String,
    web_url: String,
    explicit_token: Option<String>,
    local_on_behalf_of: Option<String>,
    http: Client,
}

impl ApiClient {
    pub fn new(
        base_url: String,
        web_url: String,
        token: Option<String>,
        on_behalf_of: Option<String>,
    ) -> Result<Self> {
        let explicit_token = token.or_else(|| std::env::var("SPILLIO_API_TOKEN").ok());
        let local_on_behalf_of = match explicit_token {
            Some(_) => None,
            None => local_on_behalf_of(on_behalf_of),
        };
        let http = Client::builder().build().context("build http client")?;
        Ok(Self {
            base_url,
            web_url,
            explicit_token,
            local_on_behalf_of,
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
            let url = format!("{}{}", self.base_url, path);
            let mut req = self.http.request(method.clone(), &url);
            if let Some(user) = &self.local_on_behalf_of {
                req = req.header("x-spillio-on-behalf-of", user);
            } else {
                req = req.bearer_auth(self.token()?);
            }
            if let Some(body) = body {
                req = req.json(body);
            }
            let resp = req
                .send()
                .with_context(|| format!("{method} {path} (is SPILLIO_API_URL reachable?)"))?;
            let status = resp.status();

            // A cached/login token can expire; clear it and re-auth once.
            if status == StatusCode::UNAUTHORIZED
                && self.explicit_token.is_none()
                && self.local_on_behalf_of.is_none()
                && !reauthed
            {
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

fn local_on_behalf_of(flag: Option<String>) -> Option<String> {
    flag.or_else(|| std::env::var("SPILLIO_ON_BEHALF_OF").ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}…", &text[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_on_behalf_of_trims_blank_values() {
        assert_eq!(
            local_on_behalf_of(Some(" ava@example.com ".to_owned())),
            Some("ava@example.com".to_owned())
        );
        assert_eq!(local_on_behalf_of(Some("   ".to_owned())), None);
    }
}
