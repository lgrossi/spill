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
    clustering_mode: String,
    card_edit_policy: String,
    anonymous_authors: bool,
    reveal_mode: String,
    planned_for: String,
    creator_email: String,
    group_id: Option<Uuid>,
}

impl RetroRepository {
    pub async fn fetch_recent_series_titles(
        &self,
        source_retro_id: Uuid,
        limit: i64,
    ) -> Result<Vec<String>, sqlx::Error> {
        let titles = sqlx::query_scalar::<_, String>(
            "WITH source AS (
                SELECT id, group_id, planned_for
                FROM retros
                WHERE id = $1
             ),
             recent AS (
                SELECT r.title, r.planned_for, r.created_at
                FROM retros r
                JOIN source s ON s.group_id IS NOT NULL AND r.group_id = s.group_id
                WHERE r.previous_retro_id IS DISTINCT FROM $1
                  AND r.planned_for <= s.planned_for
                ORDER BY r.planned_for DESC, r.created_at DESC
                LIMIT GREATEST($2, 1)
             )
             SELECT title
             FROM recent
             ORDER BY planned_for ASC, created_at ASC",
        )
        .bind(source_retro_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        if titles.is_empty() {
            return Ok(self
                .fetch_retro(source_retro_id)
                .await?
                .map(|retro| vec![retro.title])
                .unwrap_or_default());
        }
        Ok(titles)
    }

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
                clustering_mode,
                card_edit_policy,
                anonymous_authors,
                reveal_mode,
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

        let planned_for = self
            .infer_next_planned_for(source.id, group_id, &source.planned_for)
            .await?;
        let title = next_title(&source.title);
        let new_creator_email = source.creator_email.trim().to_lowercase();

        let retro = sqlx::query_as::<_, RetroRecord>(
            "INSERT INTO retros (
                title, phase, planned_for, vote_limit, action_discussion_limit,
                clustering_mode, card_edit_policy, anonymous_authors, reveal_mode, creator_email, group_id, previous_retro_id
             )
             VALUES (
                $1, 'scheduled', $2::date, $3, $4,
                CASE WHEN $5 = 'auto_on_vote_start' THEN 'auto_on_vote_start' ELSE 'disabled' END,
                CASE WHEN $6 = 'author_only' THEN 'author_only' ELSE 'collaborative' END,
                $7,
                CASE WHEN $8 = 'per_column' THEN 'per_column' ELSE 'big_bang' END,
                $9, $10, $11
             )
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                clustering_mode, clustering_status, card_edit_policy, anonymous_authors, reveal_mode,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(title)
        .bind(planned_for)
        .bind(source.vote_limit)
        .bind(source.action_discussion_limit)
        .bind(&source.clustering_mode)
        .bind(&source.card_edit_policy)
        .bind(source.anonymous_authors)
        .bind(&source.reveal_mode)
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
             RETURNING id, retro_id, external_subject, display_name, role, is_participating, 0::BIGINT AS card_count, 0::BIGINT AS vote_count",
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
            "INSERT INTO retro_columns (retro_id, column_key, title, position, accent_color)
             SELECT $2, column_key, title, position, accent_color
             FROM retro_columns
             WHERE retro_id = $1
             ORDER BY position
             RETURNING id, retro_id, column_key, title, position, accent_color, NULL::TEXT AS revealed_at",
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

    pub async fn finish_next_retro_title(
        &self,
        source_retro_id: Uuid,
        title: &str,
        expected_current_title: Option<&str>,
    ) -> Result<Option<RetroRecord>, sqlx::Error> {
        sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET title = $2
             WHERE previous_retro_id = $1
               AND ($3::text IS NULL OR title = $3)
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                card_edit_policy, anonymous_authors, reveal_mode,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(source_retro_id)
        .bind(title.trim())
        .bind(expected_current_title)
        .fetch_optional(&self.pool)
        .await
    }

    async fn infer_next_planned_for(
        &self,
        source_retro_id: Uuid,
        group_id: Uuid,
        source_planned_for: &str,
    ) -> Result<String, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            "WITH source AS (
                SELECT id, planned_for
                FROM retros
                WHERE id = $1
             ),
             previous AS (
                SELECT previous.planned_for
                FROM retros previous
                JOIN source ON TRUE
                WHERE previous.id <> source.id
                  AND previous.planned_for < source.planned_for
                  AND previous.group_id = $2
                ORDER BY previous.planned_for DESC
                LIMIT 1
             ),
             cadence AS (
                SELECT COALESCE(
                    (SELECT source.planned_for - previous.planned_for FROM source, previous),
                    14
                )::int AS days
             )
             SELECT to_char(
                GREATEST(
                    $3::date + make_interval(days => (SELECT days FROM cadence)),
                    CURRENT_DATE + INTERVAL '7 days'
                )::date,
                'YYYY-MM-DD'
             )",
        )
        .bind(source_retro_id)
        .bind(group_id)
        .bind(source_planned_for)
        .fetch_one(&self.pool)
        .await
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

        if input.remove_cover_gif {
            sqlx::query(
                "UPDATE retros SET cover_gif_url = NULL, cover_gif_alt_text = NULL WHERE id = $1",
            )
            .bind(input.retro_id)
            .execute(&mut *tx)
            .await?;
        } else if let Some(cover_gif_url) = input
            .cover_gif_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            sqlx::query(
                "UPDATE retros
                 SET cover_gif_url = $2,
                     cover_gif_alt_text = NULLIF($3, '')
                 WHERE id = $1",
            )
            .bind(input.retro_id)
            .bind(cover_gif_url)
            .bind(
                input
                    .cover_gif_alt_text
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or(""),
            )
            .execute(&mut *tx)
            .await?;
        }

        if let Some(vote_limit) = input.vote_limit {
            sqlx::query("UPDATE retros SET vote_limit = $2 WHERE id = $1")
                .bind(input.retro_id)
                .bind(vote_limit)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(action_discussion_limit) = input.action_discussion_limit {
            sqlx::query("UPDATE retros SET action_discussion_limit = $2 WHERE id = $1")
                .bind(input.retro_id)
                .bind(action_discussion_limit)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(clustering_mode) = input.clustering_mode.as_deref().map(str::trim) {
            sqlx::query(
                "UPDATE retros
                 SET clustering_status = CASE
                         WHEN clustering_mode IS DISTINCT FROM CASE WHEN $2 = 'auto_on_vote_start' THEN 'auto_on_vote_start' ELSE 'disabled' END
                         THEN 'not_run'
                         ELSE clustering_status
                     END,
                     clustering_mode = CASE WHEN $2 = 'auto_on_vote_start' THEN 'auto_on_vote_start' ELSE 'disabled' END
                 WHERE id = $1",
            )
            .bind(input.retro_id)
            .bind(clustering_mode)
            .execute(&mut *tx)
            .await?;
        }

        if let Some(card_edit_policy) = input.card_edit_policy.as_deref().map(str::trim) {
            // Workflow validates the value before reaching here; the SQL CHECK
            // on retros.card_edit_policy is the final guard.
            sqlx::query("UPDATE retros SET card_edit_policy = $2 WHERE id = $1")
                .bind(input.retro_id)
                .bind(card_edit_policy)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(anonymous_authors) = input.anonymous_authors {
            sqlx::query("UPDATE retros SET anonymous_authors = $2 WHERE id = $1")
                .bind(input.retro_id)
                .bind(anonymous_authors)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(reveal_mode) = input.reveal_mode.as_deref().map(str::trim) {
            // Workflow validates against {'per_column', 'big_bang'} before
            // reaching here; the SQL CHECK on retros.reveal_mode is the
            // final guard.
            sqlx::query("UPDATE retros SET reveal_mode = $2 WHERE id = $1")
                .bind(input.retro_id)
                .bind(reveal_mode)
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
