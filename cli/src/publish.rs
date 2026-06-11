use std::collections::HashSet;
use std::io::Read;

use anyhow::{Context, Result, bail};
use retro_core::IngestItemRequest;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api::ApiClient;
use crate::model::{Board, IngestResponse};

const KINDS: [&str; 3] = ["mood", "wentWell", "wentWrong"];
const DEFAULT_SOURCE: &str = "claude_code";

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

pub fn run(
    client: &ApiClient,
    retro_id: &str,
    file: Option<&str>,
    source: Option<String>,
    confirm: bool,
) -> Result<()> {
    let cards: Vec<CardInput> = read_cards(file)?;
    if cards.is_empty() {
        bail!("no cards to publish");
    }
    let source = source_label(source)?;

    let board: Board = client.get(&format!("/api/retros/{retro_id}"))?;
    let phase = board.retro.phase;
    let publish_target = publish_target_for_phase(&phase)?;
    let direct = publish_target.direct;
    let placement = publish_target.placement;
    let dest = publish_target.destination;
    let prepared = prepare_cards(&cards, direct, &board.columns)?;

    if !confirm {
        bail!(
            "{} card(s) ready → {dest}; refusing without --confirm (review first)",
            prepared.len()
        );
    }

    let mut ids = Vec::with_capacity(prepared.len());
    for card in prepared {
        let request = IngestItemRequest {
            source: source.clone(),
            placement: placement.to_string(),
            target_column_id: if direct { Some(card.column_id) } else { None },
            suggested_text: card.text.clone(),
            gif_url: card.gif_url.clone(),
            idempotency_key: card.idempotency_key,
            source_metadata: json!({
                "companion": source.clone(),
                "card_kind": card.kind,
                "review_state": "approved",
                "intended_column_id": card.column_id,
            }),
            raw_payload: json!({
                "kind": card.kind,
                "text": card.text,
                "gif_url": card.gif_url,
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

struct PreparedCard {
    column_id: Uuid,
    kind: String,
    text: Option<String>,
    gif_url: Option<String>,
    idempotency_key: Option<String>,
}

fn publish_target_for_phase(phase: &str) -> Result<PublishTarget> {
    match phase {
        "writing" | "voting" => Ok(PublishTarget {
            direct: true,
            placement: "retro_draft",
            destination: "board columns",
        }),
        _ => bail!("board phase is {phase:?}; cannot publish (need writing/voting)"),
    }
}

fn prepare_cards(
    cards: &[CardInput],
    direct: bool,
    columns: &[crate::model::Column],
) -> Result<Vec<PreparedCard>> {
    let column_ids: HashSet<Uuid> = columns.iter().map(|column| column.id).collect();
    cards
        .iter()
        .enumerate()
        .map(|(index, card)| prepare_card(index, card, direct, &column_ids))
        .collect()
}

fn prepare_card(
    index: usize,
    card: &CardInput,
    direct: bool,
    column_ids: &HashSet<Uuid>,
) -> Result<PreparedCard> {
    let number = index + 1;
    let kind = card.kind.clone().unwrap_or_else(|| "wentWell".to_string());
    if !KINDS.contains(&kind.as_str()) {
        bail!("card {number} kind must be one of {}", KINDS.join(", "));
    }
    let text = clean_optional(card.text.as_deref());
    let gif_url = clean_optional(card.gif_url.as_deref());
    if text.is_none() && gif_url.is_none() {
        bail!("card {number} needs text or gif_url");
    }
    if direct && !column_ids.contains(&card.column_id) {
        bail!("card {number} column_id is not part of the target board");
    }

    Ok(PreparedCard {
        column_id: card.column_id,
        kind,
        text,
        gif_url,
        idempotency_key: idempotency_key_for(card),
    })
}

fn idempotency_key_for(card: &CardInput) -> Option<String> {
    card.idempotency_key.clone()
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn source_label(flag: Option<String>) -> Result<String> {
    let source = flag
        .or_else(|| std::env::var("SPILLIO_SOURCE").ok())
        .unwrap_or_else(|| DEFAULT_SOURCE.to_owned());
    let source = source.trim();
    if source.is_empty() {
        bail!("source cannot be empty");
    }
    Ok(source.to_owned())
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
        assert!(publish_target_for_phase("scheduled").is_err());
        assert!(publish_target_for_phase("writing").is_ok());
        assert!(publish_target_for_phase("voting").is_ok());
        assert!(publish_target_for_phase("discussion").is_err());
        assert!(publish_target_for_phase("action_discussion").is_err());
        assert!(publish_target_for_phase("completed").is_err());
    }

    #[test]
    fn clean_optional_rejects_blank_text_and_gif_values() {
        assert_eq!(clean_optional(None), None);
        assert_eq!(clean_optional(Some("   ")), None);
        assert_eq!(
            clean_optional(Some(" https://example.com/g.gif ")),
            Some("https://example.com/g.gif".to_owned())
        );
    }

    #[test]
    fn source_label_defaults_and_trims() {
        assert_eq!(
            source_label(Some(" pi ".to_owned())).unwrap(),
            "pi".to_owned()
        );
        assert_eq!(source_label(None).unwrap(), DEFAULT_SOURCE.to_owned());
        assert!(source_label(Some("   ".to_owned())).is_err());
    }

    #[test]
    fn prepare_cards_validates_the_whole_batch_before_publishing() {
        let column_id = Uuid::new_v4();
        let columns = vec![crate::model::Column {
            id: column_id,
            column_key: "wentWell".to_owned(),
            title: "Went well".to_owned(),
            position: 1,
        }];
        let cards = vec![
            CardInput {
                column_id,
                text: Some("valid".to_owned()),
                kind: Some("wentWell".to_owned()),
                gif_url: None,
                idempotency_key: None,
            },
            CardInput {
                column_id,
                text: Some("invalid".to_owned()),
                kind: Some("not-a-kind".to_owned()),
                gif_url: None,
                idempotency_key: None,
            },
        ];

        assert!(prepare_cards(&cards, true, &columns).is_err());
    }

    #[test]
    fn prepare_cards_rejects_foreign_columns_for_direct_publish() {
        let board_column = Uuid::new_v4();
        let foreign_column = Uuid::new_v4();
        let columns = vec![crate::model::Column {
            id: board_column,
            column_key: "wentWell".to_owned(),
            title: "Went well".to_owned(),
            position: 1,
        }];
        let cards = vec![CardInput {
            column_id: foreign_column,
            text: Some("wrong board".to_owned()),
            kind: Some("wentWell".to_owned()),
            gif_url: None,
            idempotency_key: None,
        }];

        assert!(prepare_cards(&cards, true, &columns).is_err());
    }
}
