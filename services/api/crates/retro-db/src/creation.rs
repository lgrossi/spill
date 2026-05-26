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

    let retro = sqlx::query_as::<_, RetroRecord>(
        "INSERT INTO retros (title, vote_limit, action_discussion_limit, clustering_mode)
         VALUES ($1, $2, $3, 'disabled')
         RETURNING id, title, phase, vote_limit, action_discussion_limit",
    )
    .bind(input.title.trim())
    .bind(input.vote_limit)
    .bind(input.action_discussion_limit)
    .fetch_one(&mut *tx)
    .await?;

    let participant = sqlx::query_as::<_, ParticipantRecord>(
        "INSERT INTO participants (retro_id, external_subject, display_name, role)
         VALUES ($1, $2, $3, 'host')
         ON CONFLICT (retro_id, external_subject) DO UPDATE
         SET display_name = EXCLUDED.display_name
         RETURNING id, retro_id, display_name, role",
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
             RETURNING id, retro_id, column_key, title, position, order_direction, accent_color",
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

    Ok(RetroBoard {
        retro,
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
