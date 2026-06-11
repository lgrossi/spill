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
    let publish_target = publish_target_for_phase(&phase)?;
    let direct = publish_target.direct;
    let placement = publish_target.placement;
    let dest = publish_target.destination;

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
            idempotency_key: idempotency_key_for(card),
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

struct PublishTarget {
    direct: bool,
    placement: &'static str,
    destination: &'static str,
}

fn publish_target_for_phase(phase: &str) -> Result<PublishTarget> {
    match phase {
        "scheduled" => Ok(PublishTarget {
            direct: false,
            placement: "user_deck",
            destination: "your deck (board not open yet — accept when writing starts)",
        }),
        "writing" | "voting" => Ok(PublishTarget {
            direct: true,
            placement: "retro_draft",
            destination: "board columns",
        }),
        _ => bail!("board phase is {phase:?}; cannot publish (need scheduled/writing/voting)"),
    }
}

fn idempotency_key_for(card: &CardInput) -> Option<String> {
    card.idempotency_key.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_idempotency_key_stays_absent() {
        let card = CardInput {
            column_id: Uuid::nil(),
            text: Some("new thought".to_owned()),
            kind: None,
            gif_url: None,
            idempotency_key: None,
        };

        assert_eq!(idempotency_key_for(&card), None);
    }

    #[test]
    fn explicit_idempotency_key_is_preserved() {
        let card = CardInput {
            column_id: Uuid::nil(),
            text: Some("same thought".to_owned()),
            kind: None,
            gif_url: None,
            idempotency_key: Some("event-123".to_owned()),
        };

        assert_eq!(idempotency_key_for(&card), Some("event-123".to_owned()));
    }

    #[test]
    fn publish_rejects_deck_phases_after_writing() {
        assert!(publish_target_for_phase("scheduled").is_ok());
        assert!(publish_target_for_phase("writing").is_ok());
        assert!(publish_target_for_phase("voting").is_ok());
        assert!(publish_target_for_phase("discussion").is_err());
        assert!(publish_target_for_phase("action_discussion").is_err());
        assert!(publish_target_for_phase("completed").is_err());
    }
}
