use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GET /api/retros
#[derive(Deserialize)]
pub struct Overview {
    #[serde(default)]
    pub active: Vec<Summary>,
    #[serde(default)]
    pub completed: Vec<Summary>,
}

#[derive(Deserialize)]
pub struct Summary {
    pub id: Uuid,
    pub title: String,
    pub phase: String,
    #[serde(default)]
    pub planned_for: Option<String>,
    #[serde(default)]
    pub happened_at: Option<String>,
    #[serde(default)]
    pub group_name: Option<String>,
}

/// GET /api/retros/{id}
#[derive(Deserialize)]
pub struct Board {
    pub retro: BoardRetro,
    #[serde(default)]
    pub columns: Vec<Column>,
}

#[derive(Deserialize)]
pub struct BoardRetro {
    pub phase: String,
}

#[derive(Deserialize)]
pub struct Column {
    pub id: Uuid,
    pub column_key: String,
    pub title: String,
    pub position: i64,
    #[serde(default)]
    pub cards: Vec<BoardCard>,
}

/// A card already on the board. In `writing` phase other people's drafts come
/// back `hidden` with a null `body_text`; the caller's own cards carry text.
#[derive(Deserialize)]
pub struct BoardCard {
    #[serde(default)]
    pub body_text: Option<String>,
    #[serde(default)]
    pub gif_url: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

/// POST /api/retros/{id}/ingest
#[derive(Deserialize)]
pub struct IngestResponse {
    pub id: Uuid,
}

/// GET /api/gifs/search
#[derive(Deserialize)]
pub struct GifSearchResponse {
    #[serde(default)]
    pub results: Vec<GifHit>,
    #[serde(default)]
    pub degraded: bool,
}

#[derive(Deserialize, Serialize)]
pub struct GifHit {
    #[serde(default)]
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub preview_url: String,
    #[serde(default)]
    pub alt_text: String,
}
