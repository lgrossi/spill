use serde::Deserialize;
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
}

/// POST /api/retros/{id}/ingest
#[derive(Deserialize)]
pub struct IngestResponse {
    pub id: Uuid,
}
