use uuid::Uuid;

use crate::{
    ParticipantRecord, ReadyInfo, RetroBoard, RetroColumnRecord, RetroColumnRow, RetroRecord,
    RetroRepository, RetroSeriesRecord, UpdateRetroDetailsInput, VotingInfo,
};

#[derive(sqlx::FromRow)]
struct NextSource {
    id: Uuid,
    title: String,
    vote_limit: i32,
    action_discussion_limit: i32,
    planned_for: String,
    creator_email: String,
    group_id: Option<Uuid>,
}

impl RetroRepository {
    pub async fn ensure_next_retro(
        &self,
        source_retro_id: Uuid,
        creator_subject: &str,
        creator_email: &str,
        creator_display_name: &str,
    ) -> Result<Option<RetroBoard>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        if let Some(existing_id) = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM retros WHERE previous_retro_id = $1 LIMIT 1",
        )
        .bind(source_retro_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return self.fetch_board(existing_id).await;
        }

        let Some(source) = sqlx::query_as::<_, NextSource>(
            "SELECT
                id,
                title,
                vote_limit,
                action_discussion_limit,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                creator_email,
                group_id
             FROM retros
             WHERE id = $1",
        )
        .bind(source_retro_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(None);
        };

        let group_id = if let Some(group_id) = source.group_id {
            group_id
        } else {
            let group_id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO retro_groups (name) VALUES ($1) RETURNING id",
            )
            .bind(&source.title)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query("UPDATE retros SET group_id = $2 WHERE id = $1")
                .bind(source.id)
                .bind(group_id)
                .execute(&mut *tx)
                .await?;
            group_id
        };

        let planned_for = sqlx::query_scalar::<_, String>(
            "SELECT to_char(
                GREATEST($1::date + INTERVAL '14 days', CURRENT_DATE + INTERVAL '7 days')::date,
                'YYYY-MM-DD'
            )",
        )
        .bind(&source.planned_for)
        .fetch_one(&mut *tx)
        .await?;
        let title = next_title(&source.title);
        let new_creator_email = source.creator_email.trim().to_lowercase();

        let retro = sqlx::query_as::<_, RetroRecord>(
            "INSERT INTO retros (
                title, phase, planned_for, vote_limit, action_discussion_limit,
                clustering_mode, creator_email, group_id, previous_retro_id
             )
             VALUES ($1, 'scheduled', $2::date, $3, $4, 'disabled', $5, $6, $7)
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(title)
        .bind(planned_for)
        .bind(source.vote_limit)
        .bind(source.action_discussion_limit)
        .bind(&new_creator_email)
        .bind(group_id)
        .bind(source_retro_id)
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
        .bind(source_retro_id)
        .bind(retro.id)
        .execute(&mut *tx)
        .await?;

        if !new_creator_email.is_empty() {
            sqlx::query(
                "INSERT INTO board_grants (retro_id, principal_email, role)
                 VALUES ($1, $2, 'host')
                 ON CONFLICT (retro_id, principal_email) DO UPDATE SET role = 'host'",
            )
            .bind(retro.id)
            .bind(&new_creator_email)
            .execute(&mut *tx)
            .await?;
        }

        let participant = sqlx::query_as::<_, ParticipantRecord>(
            "INSERT INTO participants (retro_id, external_subject, display_name, role)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (retro_id, external_subject) DO UPDATE
             SET display_name = EXCLUDED.display_name
             RETURNING id, retro_id, external_subject, display_name, role, 0::BIGINT AS card_count, 0::BIGINT AS vote_count",
        )
        .bind(retro.id)
        .bind(creator_subject.trim())
        .bind(creator_display_name.trim())
        .bind(if creator_email.trim().eq_ignore_ascii_case(&new_creator_email) {
            "host"
        } else {
            "member"
        })
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
        .bind(source_retro_id)
        .bind(retro.id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(RetroColumnRecord::from)
        .collect();

        let series = sqlx::query_as::<_, RetroSeriesRecord>(
            "SELECT id, name FROM retro_groups WHERE id = $1",
        )
        .bind(group_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(RetroBoard {
            retro,
            series,
            next_retro: None,
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

    pub async fn update_retro_details(
        &self,
        input: UpdateRetroDetailsInput,
    ) -> Result<Option<RetroRecord>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let Some(retro) = self.fetch_retro(input.retro_id).await? else {
            return Ok(None);
        };

        if let Some(title) = input
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            sqlx::query("UPDATE retros SET title = $2 WHERE id = $1")
                .bind(input.retro_id)
                .bind(title)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(group_name) = input
            .group_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let group_id = if let Some(existing) =
                sqlx::query_scalar::<_, Option<Uuid>>("SELECT group_id FROM retros WHERE id = $1")
                    .bind(input.retro_id)
                    .fetch_one(&mut *tx)
                    .await?
            {
                sqlx::query("UPDATE retro_groups SET name = $2 WHERE id = $1")
                    .bind(existing)
                    .bind(group_name)
                    .execute(&mut *tx)
                    .await?;
                existing
            } else {
                sqlx::query_scalar::<_, Uuid>(
                    "INSERT INTO retro_groups (name) VALUES ($1) RETURNING id",
                )
                .bind(group_name)
                .fetch_one(&mut *tx)
                .await?
            };
            sqlx::query("UPDATE retros SET group_id = $2 WHERE id = $1")
                .bind(input.retro_id)
                .bind(group_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        self.fetch_retro(retro.id).await
    }
}

fn next_title(source_title: &str) -> String {
    let trimmed = source_title.trim();
    if let Some(rest) = trimmed.strip_prefix("Next: ") {
        format!("Next: {rest}")
    } else {
        format!("Next: {trimmed}")
    }
}
