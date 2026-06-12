use std::io::Read;

use anyhow::{Context, Result, bail};
use retro_core::IngestItemRequest;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api::ApiClient;
use crate::gif;
use crate::model::{Board, Column, IngestResponse};

const KINDS: [&str; 3] = ["mood", "wentWell", "wentWrong"];
const DEFAULT_SOURCE: &str = "claude_code";

#[derive(Deserialize)]
struct CardInput {
    /// Target column as a UUID, column key (e.g. "1_well"), or title. Prefer
    /// `column`; `column_id` stays for backward compatibility.
    #[serde(default)]
    column: Option<String>,
    #[serde(default)]
    column_id: Option<Uuid>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    gif_url: Option<String>,
    /// Search phrase resolved to a GIF URL at publish time when `gif_url` is absent.
    #[serde(default)]
    gif_query: Option<String>,
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
    for mut card in prepared {
        resolve_gif(client, &mut card)?;
        if card.text.is_none() && card.gif_url.is_none() {
            bail!("a card resolved to neither text nor a gif (gif_query found nothing)");
        }
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
    gif_query: Option<String>,
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
    columns: &[Column],
) -> Result<Vec<PreparedCard>> {
    cards
        .iter()
        .enumerate()
        .map(|(index, card)| prepare_card(index, card, direct, columns))
        .collect()
}

fn prepare_card(
    index: usize,
    card: &CardInput,
    direct: bool,
    columns: &[Column],
) -> Result<PreparedCard> {
    let number = index + 1;
    let column = resolve_column(card, columns).with_context(|| format!("card {number}"))?;
    if direct && !columns.iter().any(|c| c.id == column.id) {
        bail!("card {number} column is not part of the target board");
    }

    let kind = match card.kind.clone() {
        Some(kind) => {
            if !KINDS.contains(&kind.as_str()) {
                bail!("card {number} kind must be one of {}", KINDS.join(", "));
            }
            kind
        }
        None => derive_kind(&column.column_key).to_owned(),
    };

    let text = clean_optional(card.text.as_deref());
    let gif_url = clean_optional(card.gif_url.as_deref());
    let gif_query = clean_optional(card.gif_query.as_deref());
    if text.is_none() && gif_url.is_none() && gif_query.is_none() {
        bail!("card {number} needs text, gif_url, or gif_query");
    }

    Ok(PreparedCard {
        column_id: column.id,
        kind,
        text,
        gif_url,
        gif_query,
        idempotency_key: card.idempotency_key.clone(),
    })
}

/// Resolve a card's target column from `column` (UUID | key | title) or the
/// legacy `column_id`. Matching is exact on key/UUID and case-insensitive on title.
fn resolve_column<'a>(card: &CardInput, columns: &'a [Column]) -> Result<&'a Column> {
    if let Some(id) = card.column_id {
        return columns
            .iter()
            .find(|c| c.id == id)
            .with_context(|| format!("column_id {id} is not part of the target board"));
    }
    let Some(selector) = card
        .column
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        bail!("needs a column (key, UUID, or title) or column_id");
    };
    columns
        .iter()
        .find(|c| {
            c.column_key == selector
                || c.id.to_string() == selector
                || c.title.eq_ignore_ascii_case(selector)
        })
        .with_context(|| format!("no board column matches {selector:?}"))
}

/// Derive a card kind from its column key so callers don't have to repeat it.
fn derive_kind(column_key: &str) -> &'static str {
    let key = column_key.to_ascii_lowercase();
    if key.contains("feeling") || key.contains("mood") {
        "mood"
    } else if key.contains("improve") || key.contains("wrong") || key.contains("bad") {
        "wentWrong"
    } else {
        "wentWell"
    }
}

/// Fill `gif_url` from `gif_query` when no URL was supplied. A miss is a warning,
/// not a failure — the text usually still carries the card.
fn resolve_gif(client: &ApiClient, card: &mut PreparedCard) -> Result<()> {
    if card.gif_url.is_some() {
        return Ok(());
    }
    let Some(query) = card.gif_query.as_deref() else {
        return Ok(());
    };
    match gif::resolve(client, query, gif::DEFAULT_KIND)? {
        Some(hit) => card.gif_url = Some(hit.url),
        None => eprintln!("spill: no gif found for {query:?}; publishing that card without one"),
    }
    Ok(())
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
        "cards must be a JSON list: [{column|column_id, text?, kind?, gif_url?, gif_query?, idempotency_key?}]",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(id: Uuid, key: &str, title: &str) -> Column {
        Column {
            id,
            column_key: key.to_owned(),
            title: title.to_owned(),
            position: 1,
            cards: vec![],
        }
    }

    fn input(column: Option<&str>, column_id: Option<Uuid>, text: &str) -> CardInput {
        CardInput {
            column: column.map(str::to_owned),
            column_id,
            text: Some(text.to_owned()),
            kind: None,
            gif_url: None,
            gif_query: None,
            idempotency_key: None,
        }
    }

    #[test]
    fn resolve_column_by_key_uuid_and_title() {
        let id = Uuid::new_v4();
        let columns = vec![column(id, "1_well", "Went well")];

        assert_eq!(
            resolve_column(&input(Some("1_well"), None, "x"), &columns)
                .unwrap()
                .id,
            id
        );
        assert_eq!(
            resolve_column(&input(Some(&id.to_string()), None, "x"), &columns)
                .unwrap()
                .id,
            id
        );
        assert_eq!(
            resolve_column(&input(Some("went well"), None, "x"), &columns)
                .unwrap()
                .id,
            id
        );
        assert_eq!(
            resolve_column(&input(None, Some(id), "x"), &columns)
                .unwrap()
                .id,
            id
        );
    }

    #[test]
    fn resolve_column_errors_on_unknown_or_missing() {
        let columns = vec![column(Uuid::new_v4(), "1_well", "Went well")];
        assert!(resolve_column(&input(Some("nope"), None, "x"), &columns).is_err());
        assert!(resolve_column(&input(None, None, "x"), &columns).is_err());
        assert!(resolve_column(&input(None, Some(Uuid::new_v4()), "x"), &columns).is_err());
    }

    #[test]
    fn derive_kind_from_column_key() {
        assert_eq!(derive_kind("0_feeling"), "mood");
        assert_eq!(derive_kind("1_well"), "wentWell");
        assert_eq!(derive_kind("2_improve"), "wentWrong");
        assert_eq!(derive_kind("3_actions"), "wentWell");
    }

    #[test]
    fn explicit_kind_is_validated_and_preserved() {
        let id = Uuid::new_v4();
        let columns = vec![column(id, "1_well", "Went well")];
        let mut card = input(Some("1_well"), None, "x");
        card.kind = Some("mood".to_owned());
        assert_eq!(prepare_card(0, &card, true, &columns).unwrap().kind, "mood");

        card.kind = Some("bogus".to_owned());
        assert!(prepare_card(0, &card, true, &columns).is_err());
    }

    #[test]
    fn omitted_kind_is_derived_from_column() {
        let id = Uuid::new_v4();
        let columns = vec![column(id, "2_improve", "To improve")];
        let prepared =
            prepare_card(0, &input(Some("2_improve"), None, "x"), true, &columns).unwrap();
        assert_eq!(prepared.kind, "wentWrong");
    }

    #[test]
    fn card_without_text_gif_or_query_is_rejected() {
        let id = Uuid::new_v4();
        let columns = vec![column(id, "1_well", "Went well")];
        let mut card = input(Some("1_well"), None, "x");
        card.text = None;
        assert!(prepare_card(0, &card, true, &columns).is_err());

        card.gif_query = Some("mic drop".to_owned());
        assert!(prepare_card(0, &card, true, &columns).is_ok());
    }

    #[test]
    fn publish_rejects_deck_phases_after_writing() {
        assert!(publish_target_for_phase("scheduled").is_err());
        assert!(publish_target_for_phase("writing").is_ok());
        assert!(publish_target_for_phase("voting").is_ok());
        assert!(publish_target_for_phase("completed").is_err());
    }

    #[test]
    fn clean_optional_rejects_blank_values() {
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
}
