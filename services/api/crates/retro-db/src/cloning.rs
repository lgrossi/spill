use sqlx::PgPool;

use crate::{
    CloneRetroInput, ParticipantRecord, ReadyInfo, RetroBoard, RetroColumnRecord, RetroColumnRow,
    RetroRecord, VotingInfo,
};

#[derive(Debug, sqlx::FromRow)]
struct CloneSource {
    title: String,
    vote_limit: i32,
    action_discussion_limit: i32,
    clustering_mode: String,
    default_scheduled_at: Option<String>,
}

pub(super) async fn clone_retro(
    pool: &PgPool,
    input: CloneRetroInput,
) -> Result<Option<RetroBoard>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let Some(source) = sqlx::query_as::<_, CloneSource>(
        "SELECT
            title,
            vote_limit,
            action_discussion_limit,
            clustering_mode,
            to_char((COALESCE(scheduled_at, created_at) + INTERVAL '14 days') AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS default_scheduled_at
         FROM retros
         WHERE id = $1",
    )
    .bind(input.source_retro_id)
    .fetch_optional(&mut *tx)
    .await? else {
        return Ok(None);
    };

    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Next: {}", source.title));
    let scheduled_at = input
        .scheduled_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or(source.default_scheduled_at.as_deref())
        .unwrap_or("");
    let creator_email = input.creator_email.trim().to_lowercase();

    let retro = sqlx::query_as::<_, RetroRecord>(
        "INSERT INTO retros (title, scheduled_at, vote_limit, action_discussion_limit, clustering_mode, creator_email)
         VALUES ($1, NULLIF($2, '')::timestamptz, $3, $4, $5, $6)
         RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
            to_char(scheduled_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS scheduled_at,
            to_char(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS completed_at",
    )
    .bind(title)
    .bind(scheduled_at)
    .bind(source.vote_limit)
    .bind(source.action_discussion_limit)
    .bind(source.clustering_mode)
    .bind(&creator_email)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO board_grants (retro_id, principal_email, role)
         SELECT $2, principal_email, role
         FROM board_grants
         WHERE retro_id = $1
         ON CONFLICT (retro_id, principal_email) DO UPDATE
           SET role = CASE WHEN board_grants.role = 'host' THEN board_grants.role ELSE EXCLUDED.role END",
    )
    .bind(input.source_retro_id)
    .bind(retro.id)
    .execute(&mut *tx)
    .await?;

    if !creator_email.is_empty() {
        sqlx::query(
            "INSERT INTO board_grants (retro_id, principal_email, role)
             VALUES ($1, $2, 'host')
             ON CONFLICT (retro_id, principal_email) DO UPDATE SET role = 'host'",
        )
        .bind(retro.id)
        .bind(&creator_email)
        .execute(&mut *tx)
        .await?;
    }

    let participant = sqlx::query_as::<_, ParticipantRecord>(
        "INSERT INTO participants (retro_id, external_subject, display_name, role)
         VALUES ($1, $2, $3, 'host')
         ON CONFLICT (retro_id, external_subject) DO UPDATE
         SET display_name = EXCLUDED.display_name
         RETURNING id, retro_id, external_subject, display_name, role",
    )
    .bind(retro.id)
    .bind(input.creator_subject.trim())
    .bind(input.creator_display_name.trim())
    .fetch_one(&mut *tx)
    .await?;

    let columns = sqlx::query_as::<_, RetroColumnRow>(
        "INSERT INTO retro_columns (retro_id, column_key, title, position, order_direction, accent_color)
         SELECT $2, column_key, title, position, order_direction, accent_color
         FROM retro_columns
         WHERE retro_id = $1
         ORDER BY position
         RETURNING id, retro_id, column_key, title, position, order_direction, accent_color",
    )
    .bind(input.source_retro_id)
    .bind(retro.id)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(RetroColumnRecord::from)
    .collect();

    tx.commit().await?;

    Ok(Some(RetroBoard {
        retro,
        participants: vec![participant],
        columns,
        ready: ReadyInfo::default(),
        voting: VotingInfo::default(),
        clusters: Vec::new(),
        actions: Vec::new(),
        deck: Vec::new(),
        ai_artifacts: Vec::new(),
        meeting_notes: Vec::new(),
        deliveries: Vec::new(),
    }))
}
