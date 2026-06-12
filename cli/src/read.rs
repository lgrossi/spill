use anyhow::Result;
use serde_json::json;

use crate::api::ApiClient;
use crate::model::Board;

/// `spill read --retro-id <id>` — print the board's columns and the cards
/// visible to the caller, so a companion can calibrate voice and dedup against
/// what is already there. In `writing` phase other people's drafts come back
/// hidden (no text); those are surfaced as `{hidden: true}`.
pub fn run(client: &ApiClient, retro_id: &str) -> Result<()> {
    let board: Board = client.get(&format!("/api/retros/{retro_id}"))?;

    let columns: Vec<_> = board
        .columns
        .iter()
        .map(|column| {
            let cards: Vec<_> = column
                .cards
                .iter()
                .map(|card| {
                    if card.hidden {
                        json!({"hidden": true})
                    } else {
                        json!({"text": card.body_text, "gif_url": card.gif_url})
                    }
                })
                .collect();
            json!({
                "id": column.id,
                "key": column.column_key,
                "title": column.title,
                "position": column.position,
                "cards": cards,
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "retro_id": retro_id,
            "phase": board.retro.phase,
            "columns": columns,
        }))?
    );
    Ok(())
}
