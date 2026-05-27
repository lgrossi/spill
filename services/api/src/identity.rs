use axum::http::HeaderMap;
use serde::Serialize;

use crate::error::ApiError;

pub(crate) const HEADER_USER_SUBJECT: &str = "x-spillio-user-subject";
pub(crate) const HEADER_USER_NAME: &str = "x-spillio-user-name";
pub(crate) const HEADER_USER_EMAIL: &str = "x-spillio-user-email";

#[derive(Clone, Default)]
pub struct LinkAccessPolicy;

impl LinkAccessPolicy {
    pub fn can_edit_retro_link(&self, retro_id: &str) -> bool {
        !retro_id.trim().is_empty()
    }
}

#[derive(Serialize)]
pub struct CurrentUser {
    pub subject: String,
    pub email: String,
    pub display_name: String,
}

impl CurrentUser {
    pub fn from_headers(headers: &HeaderMap) -> Result<Self, ApiError> {
        let subject = required_header(headers, HEADER_USER_SUBJECT)?;
        let display_name =
            optional_header(headers, HEADER_USER_NAME).unwrap_or_else(|| subject.clone());
        let email = optional_header(headers, HEADER_USER_EMAIL).unwrap_or_default();

        Ok(Self {
            subject,
            email,
            display_name,
        })
    }
}

#[derive(Serialize)]
pub struct AccessModel {
    pub kind: &'static str,
    pub can_edit_with_link: bool,
}

fn required_header(headers: &HeaderMap, name: &'static str) -> Result<String, ApiError> {
    optional_header(headers, name)
        .ok_or_else(|| ApiError::unauthorized(format!("missing required header {name}")))
}

fn optional_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
