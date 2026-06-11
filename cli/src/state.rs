use anyhow::Result;
use serde_json::json;

use crate::api::ApiClient;
use crate::model::{Board, Overview};

/// Find the board to prepare for (scheduled/writing), the previous retro in the
/// same series, the derived window, and the board's real columns.
pub fn run(client: &ApiClient) -> Result<()> {
    let overview: Overview = client.get("/api/retros")?;

    let mut active: Vec<_> = overview
        .active
        .into_iter()
        .filter(|r| r.phase == "scheduled" || r.phase == "writing")
        .collect();
    active.sort_by(|a, b| a.planned_for.cmp(&b.planned_for));

    let Some(target) = active.into_iter().next() else {
        print(&json!({
            "target": null,
            "note": "no scheduled or writing board for this user",
        }))?;
        return Ok(());
    };

    let series = target.group_name.clone();
    let mut done: Vec<_> = overview
        .completed
        .into_iter()
        .filter(|r| r.group_name == series && r.happened_at.is_some())
        .collect();
    done.sort_by(|a, b| b.happened_at.cmp(&a.happened_at));
    let previous = done.into_iter().next();

    let board: Board = client.get(&format!("/api/retros/{}", target.id))?;
    let columns: Vec<_> = board
        .columns
        .iter()
        .map(|c| json!({"id": c.id, "key": c.column_key, "title": c.title, "position": c.position}))
        .collect();

    let since = previous
        .as_ref()
        .and_then(|p| p.happened_at.as_ref())
        .map(|h| h.chars().take(10).collect::<String>());

    print(&json!({
        "target": {
            "id": target.id,
            "title": target.title,
            "phase": target.phase,
            "planned_for": target.planned_for,
            "series": series,
        },
        "previous": previous.as_ref().map(|p| json!({
            "id": p.id, "title": p.title, "happened_at": p.happened_at,
        })),
        "window": { "since": since, "until": target.planned_for },
        "columns": columns,
    }))
}

fn print(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
