use sqlx::PgPool;

use crate::{
    CreateRetroInput, ParticipantRecord, ReadyInfo, RetroBoard, RetroColumnRecord, RetroColumnRow,
    RetroRecord, VotingInfo,
    domain_mapping::{column_accent_color, column_key},
};

pub(super) async fn create_retro(
    pool: &PgPool,
    input: CreateRetroInput,
) -> Result<RetroBoard, sqlx::Error> {
    let mut columns = input.template.column_titles();
    if input.action_discussion_limit > 0
        && !columns
            .iter()
            .any(|column| column.trim().eq_ignore_ascii_case("actions"))
    {
        columns.push("Actions".to_owned());
    }
    let mut tx = pool.begin().await?;

    let creator_email = input.creator_email.trim().to_lowercase();
    let group_name = input
        .group_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let group_id = if let Some(group_name) = group_name {
        Some(
            sqlx::query_scalar::<_, uuid::Uuid>(
                "INSERT INTO retro_groups (name) VALUES ($1) RETURNING id",
            )
            .bind(group_name)
            .fetch_one(&mut *tx)
            .await?,
        )
    } else {
        None
    };
    let retro = sqlx::query_as::<_, RetroRecord>(
        "WITH requested AS (
            SELECT COALESCE(NULLIF($6, '')::date, CURRENT_DATE) AS planned_for
         )
         INSERT INTO retros (title, phase, planned_for, vote_limit, action_discussion_limit, clustering_mode, creator_email, group_id, cover_gif_url, cover_gif_alt_text)
         SELECT
            $1,
            CASE WHEN requested.planned_for > CURRENT_DATE THEN 'scheduled' ELSE 'writing' END,
            requested.planned_for,
            $2,
            $3,
            'disabled',
            $4,
            $5,
            NULLIF($7, ''),
            NULLIF($8, '')
         FROM requested
         RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
            card_edit_policy, anonymous_authors,
            to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
            to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
    )
    .bind(input.title.trim())
    .bind(input.vote_limit)
    .bind(input.action_discussion_limit)
    .bind(&creator_email)
    .bind(group_id)
    .bind(input.planned_for.as_deref().map(str::trim).unwrap_or(""))
    .bind(input.cover_gif_url.as_deref().map(str::trim).unwrap_or(""))
    .bind(input.cover_gif_alt_text.as_deref().map(str::trim).unwrap_or(""))
    .fetch_one(&mut *tx)
    .await?;

    if !input.creator_email.trim().is_empty() {
        sqlx::query(
            "INSERT INTO board_grants (retro_id, principal_email, role)
             VALUES ($1, lower($2), 'host')
             ON CONFLICT (retro_id, principal_email) DO NOTHING",
        )
        .bind(retro.id)
        .bind(input.creator_email.trim())
        .execute(&mut *tx)
        .await?;
    }

    let participant = sqlx::query_as::<_, ParticipantRecord>(
        "INSERT INTO participants (retro_id, external_subject, display_name, role)
         VALUES ($1, $2, $3, 'host')
         ON CONFLICT (retro_id, external_subject) DO UPDATE
         SET display_name = EXCLUDED.display_name
         RETURNING id, retro_id, external_subject, display_name, role, is_participating, 0::BIGINT AS card_count, 0::BIGINT AS vote_count",
    )
    .bind(retro.id)
    .bind(input.creator_subject.trim())
    .bind(input.creator_display_name.trim())
    .fetch_one(&mut *tx)
    .await?;

    let mut records = Vec::with_capacity(columns.len());
    for (position, title) in columns.iter().enumerate() {
        let accent_color = input
            .column_colors
            .get(position)
            .map(String::as_str)
            .filter(|color| !color.trim().is_empty())
            .unwrap_or_else(|| column_accent_color(title, position));
        let record = sqlx::query_as::<_, RetroColumnRow>(
            "INSERT INTO retro_columns (retro_id, column_key, title, position, accent_color)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, retro_id, column_key, title, position, accent_color",
        )
        .bind(retro.id)
        .bind(column_key(title, position))
        .bind(title.trim())
        .bind(position as i32)
        .bind(accent_color)
        .fetch_one(&mut *tx)
        .await?;
        records.push(RetroColumnRecord::from(record));
    }

    tx.commit().await?;

    let series = group_id.map(|id| crate::RetroSeriesRecord {
        id,
        name: group_name.unwrap_or_default().to_owned(),
    });

    Ok(RetroBoard {
        retro,
        series,
        next_retro: None,
        participants: vec![participant],
        columns: records,
        ready: ReadyInfo::default(),
        voting: VotingInfo::default(),
        clusters: Vec::new(),
        actions: Vec::new(),
        deck: Vec::new(),
        ai_artifacts: Vec::new(),
        meeting_notes: Vec::new(),
        deliveries: Vec::new(),
    })
}
