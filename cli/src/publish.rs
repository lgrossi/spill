use std::io::Read;

use anyhow::{Context, Result, bail};
use retro_core::IngestItemRequest;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api::ApiClient;
use crate::model::{Board, IngestResponse};

const KINDS: [&str; 3] = ["mood", "wentWell", "wentWrong"];

#[derive(Deserialize)]
struct CardInput {
    column_id: Uuid,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    gif_url: Option<String>,
    #[serde(default)]
    idempotency_key: Option<String>,
}

pub fn run(client: &ApiClient, retro_id: &str, file: Option<&str>, confirm: bool) -> Result<()> {
    let cards: Vec<CardInput> = read_cards(file)?;
    if cards.is_empty() {
        bail!("no cards to publish");
    }

    let board: Board = client.get(&format!("/api/retros/{retro_id}"))?;
    let phase = board.retro.phase;
    if phase == "completed" {
        bail!("board phase is {phase:?}; cannot publish (need scheduled/writing/voting)");
    }

    // writing/voting -> cards land directly on the columns as private drafts;
    // otherwise they wait in the user's deck until the board opens.
    let direct = phase == "writing" || phase == "voting";
    let placement = if direct { "retro_draft" } else { "user_deck" };
    let dest = if direct {
        "board columns"
    } else {
        "your deck (board not open yet — accept when writing starts)"
    };

    if !confirm {
        bail!(
            "{} card(s) ready → {dest}; refusing without --confirm (review first)",
            cards.len()
        );
    }

    let mut ids = Vec::with_capacity(cards.len());
    for (index, card) in cards.iter().enumerate() {
        let kind = card.kind.clone().unwrap_or_else(|| "wentWell".to_string());
        if !KINDS.contains(&kind.as_str()) {
            bail!(
                "card {} kind must be one of {}",
                index + 1,
                KINDS.join(", ")
            );
        }
        if card.text.as_deref().unwrap_or("").trim().is_empty() && card.gif_url.is_none() {
            bail!("card {} needs text or gif_url", index + 1);
        }

        let request = IngestItemRequest {
            source: "claude_code".to_string(),
            placement: placement.to_string(),
            target_column_id: if direct { Some(card.column_id) } else { None },
            suggested_text: card.text.clone(),
            gif_url: card.gif_url.clone(),
            idempotency_key: Some(
                card.idempotency_key
                    .clone()
                    .unwrap_or_else(|| format!("retro-{retro_id}-{}", index + 1)),
            ),
            source_metadata: json!({
                "companion": "claude_code",
                "card_kind": kind,
                "review_state": "approved",
                "intended_column_id": card.column_id,
            }),
            raw_payload: json!({
                "kind": kind,
                "text": card.text,
                "intended_column_id": card.column_id,
            }),
        };

        let item: IngestResponse =
            client.post(&format!("/api/retros/{retro_id}/ingest"), &request)?;
        ids.push(item.id);
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "published": ids.len(),
            "placement": placement,
            "where": dest,
            "ids": ids,
        }))?
    );
    Ok(())
}

fn read_cards(file: Option<&str>) -> Result<Vec<CardInput>> {
    let raw = match file {
        Some(path) => std::fs::read_to_string(path).with_context(|| format!("read {path}"))?,
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("read cards from stdin")?;
            buf
        }
    };
    serde_json::from_str(&raw).context(
        "cards must be a JSON list: [{column_id, text, kind?, gif_url?, idempotency_key?}]",
    )
}
