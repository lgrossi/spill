use anyhow::Result;
use serde_json::json;

use crate::api::ApiClient;
use crate::model::{GifHit, GifSearchResponse};

const SEARCH_PATH: &str = "/api/gifs/search";
pub const DEFAULT_KIND: &str = "gif";

/// Resolve a search phrase to the single best GIF hit, or None when the search
/// is empty or the provider degraded. Used by `publish` for `gif_query` cards.
pub fn resolve(client: &ApiClient, query: &str, kind: &str) -> Result<Option<GifHit>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(None);
    }
    let response: GifSearchResponse =
        client.get_query(SEARCH_PATH, &[("q", query), ("kind", kind)])?;
    if response.degraded {
        return Ok(None);
    }
    Ok(response.results.into_iter().next())
}

/// `spill gif "<phrase>"` — print matching GIFs as JSON for the user to choose.
pub fn run(client: &ApiClient, query: &str, kind: &str, limit: usize) -> Result<()> {
    let response: GifSearchResponse =
        client.get_query(SEARCH_PATH, &[("q", query.trim()), ("kind", kind)])?;
    let hits: Vec<&GifHit> = response.results.iter().take(limit).collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "query": query.trim(),
            "kind": kind,
            "degraded": response.degraded,
            "results": hits,
        }))?
    );
    Ok(())
}
