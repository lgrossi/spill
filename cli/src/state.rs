use anyhow::Result;
use serde_json::json;

use crate::api::ApiClient;
use crate::model::{Board, Overview};

/// Find the board to prepare for (scheduled/writing), the previous retro in the
/// same series, the derived window, and the board's real columns.
pub fn run(client: &ApiClient) -> Result<()> {
    let overview: Overview = client.get("/api/retros")?;

    let active: Vec<_> = overview
        .active
        .into_iter()
        .filter(|r| r.phase == "scheduled" || r.phase == "writing")
        .collect();

    let Some(target) = select_target(active) else {
        print(&json!({
            "target": null,
            "note": "no scheduled or writing board for this user",
        }))?;
        return Ok(());
    };

    let series = target.group_name.clone();
    let previous = previous_in_series(overview.completed, series.as_deref());

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

fn select_target(mut active: Vec<crate::model::Summary>) -> Option<crate::model::Summary> {
    active.sort_by(|a, b| match (&a.planned_for, &b.planned_for) {
        (Some(a_date), Some(b_date)) => a_date.cmp(b_date),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.title.cmp(&b.title),
    });
    active.into_iter().next()
}

fn previous_in_series(
    completed: Vec<crate::model::Summary>,
    series: Option<&str>,
) -> Option<crate::model::Summary> {
    let series = series?;
    let mut done: Vec<_> = completed
        .into_iter()
        .filter(|r| r.group_name.as_deref() == Some(series) && r.happened_at.is_some())
        .collect();
    done.sort_by(|a, b| b.happened_at.cmp(&a.happened_at));
    done.into_iter().next()
}

fn print(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn summary(
        title: &str,
        phase: &str,
        planned_for: Option<&str>,
        happened_at: Option<&str>,
        group_name: Option<&str>,
    ) -> crate::model::Summary {
        crate::model::Summary {
            id: Uuid::new_v4(),
            title: title.to_owned(),
            phase: phase.to_owned(),
            planned_for: planned_for.map(str::to_owned),
            happened_at: happened_at.map(str::to_owned),
            group_name: group_name.map(str::to_owned),
        }
    }

    #[test]
    fn select_target_prefers_scheduled_dates_over_undated_boards() {
        let target = select_target(vec![
            summary("Scratch", "writing", None, None, None),
            summary("Scheduled", "scheduled", Some("2099-05-15"), None, None),
        ])
        .expect("target");

        assert_eq!(target.title, "Scheduled");
    }

    #[test]
    fn previous_retro_requires_named_series() {
        let previous = previous_in_series(
            vec![summary(
                "Unrelated",
                "completed",
                Some("2026-05-01"),
                Some("2026-05-01"),
                None,
            )],
            None,
        );

        assert!(previous.is_none());
    }

    #[test]
    fn previous_retro_uses_latest_same_named_series() {
        let previous = previous_in_series(
            vec![
                summary(
                    "Old same series",
                    "completed",
                    Some("2026-04-01"),
                    Some("2026-04-01"),
                    Some("Platform"),
                ),
                summary(
                    "Latest same series",
                    "completed",
                    Some("2026-05-01"),
                    Some("2026-05-01"),
                    Some("Platform"),
                ),
                summary(
                    "Different series",
                    "completed",
                    Some("2026-06-01"),
                    Some("2026-06-01"),
                    Some("Payments"),
                ),
            ],
            Some("Platform"),
        )
        .expect("previous");

        assert_eq!(previous.title, "Latest same series");
    }
}
