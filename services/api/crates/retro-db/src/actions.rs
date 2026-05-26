use crate::{domain_mapping::action_tags, *};

impl RetroRepository {
    pub async fn start_action_discussion(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<ActionItemRecord>, ActionError> {
        let mut tx = self.pool.begin().await?;
        let retro = sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'action_discussion'
             WHERE id = $1 AND phase IN ('discussion', 'voting')
             RETURNING id, title, phase, vote_limit, action_discussion_limit",
        )
        .bind(retro_id)
        .fetch_one(&mut *tx)
        .await?;

        let action_column_id = if retro.action_discussion_limit > 0 {
            Some(
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT id
                 FROM retro_columns
                 WHERE retro_id = $1 AND LOWER(title) LIKE '%action%'
                 ORDER BY position ASC
                 LIMIT 1",
                )
                .bind(retro_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| ActionError::Invalid("action column unavailable".to_owned()))?,
            )
        } else {
            None
        };

        let candidates = if retro.vote_limit > 0 && retro.action_discussion_limit > 0 {
            let action_column_id = action_column_id
                .ok_or_else(|| ActionError::Invalid("action column unavailable".to_owned()))?;
            sqlx::query_as::<_, ActionCandidate>(
                "SELECT c.id, COALESCE(c.body_text, c.gif_alt_text, 'Untitled card') AS title, COALESCE(SUM(v.count), 0)::BIGINT AS vote_count
                 FROM cards c
                 LEFT JOIN votes v ON v.target_card_id = c.id
                 WHERE c.retro_id = $1 AND c.state = 'revealed' AND c.column_id <> $2
                 GROUP BY c.id
                 HAVING COALESCE(SUM(v.count), 0) > 0
                 ORDER BY vote_count DESC, c.created_at ASC
                 LIMIT $3",
            )
            .bind(retro_id)
            .bind(action_column_id)
            .bind(retro.action_discussion_limit as i64)
            .fetch_all(&mut *tx)
            .await?
        } else {
            Vec::new()
        };

        if let Some(action_column_id) = action_column_id {
            for candidate in &candidates {
                sqlx::query(
                    "UPDATE cards
                     SET column_id = $2,
                         position = (SELECT COALESCE(MAX(position) + 1, 0) FROM cards WHERE retro_id = $1 AND column_id = $2),
                         updated_at = NOW()
                     WHERE id = $3 AND retro_id = $1",
                )
                .bind(retro_id)
                .bind(action_column_id)
                .bind(candidate.id)
                .execute(&mut *tx)
                .await?;
            }
        }

        for (position, candidate) in candidates.iter().enumerate() {
            let tags = action_tags(&candidate.title);
            sqlx::query(
                "INSERT INTO action_items (retro_id, source_card_id, title, details, status, position, tags)
                 VALUES ($1, $2, $3, $4, 'proposed', $5, $6)",
            )
            .bind(retro_id)
            .bind(candidate.id)
            .bind(format!("Follow up: {}", candidate.title))
            .bind(format!("Based on {votes} vote(s).", votes = candidate.vote_count))
            .bind(position as i32)
            .bind(Json(tags))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(self.fetch_actions(retro_id).await?)
    }

    pub async fn complete_retro(&self, retro_id: Uuid) -> Result<Option<RetroRecord>, ActionError> {
        let retro = sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'completed', completed_at = NOW()
             WHERE id = $1 AND phase = 'action_discussion'
             RETURNING id, title, phase, vote_limit, action_discussion_limit",
        )
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(retro)
    }

    pub async fn update_action(
        &self,
        input: UpdateActionInput,
    ) -> Result<Option<ActionItemRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, ActionItemRow>(
            "UPDATE action_items
             SET title = $3, details = $4
             WHERE id = $1 AND retro_id = $2
             RETURNING id, retro_id, source_card_id, source_cluster_id, title, details, status, position, tags",
        )
        .bind(input.action_id)
        .bind(input.retro_id)
        .bind(input.title.trim())
        .bind(input.details.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn create_action(
        &self,
        input: CreateActionInput,
    ) -> Result<ActionItemRecord, sqlx::Error> {
        let tags = action_tags(&input.title);
        let row = sqlx::query_as::<_, ActionItemRow>(
            "INSERT INTO action_items (retro_id, title, details, status, position, tags)
             VALUES (
                $1,
                $2,
                $3,
                'confirmed',
                (SELECT COALESCE(MAX(position) + 1, 0) FROM action_items WHERE retro_id = $1),
                $4
             )
             RETURNING id, retro_id, source_card_id, source_cluster_id, title, details, status, position, tags",
        )
        .bind(input.retro_id)
        .bind(input.title.trim())
        .bind(input.details.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(Json(tags))
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn set_action_status(
        &self,
        retro_id: Uuid,
        action_id: Uuid,
        status: &str,
    ) -> Result<Option<ActionItemRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, ActionItemRow>(
            "UPDATE action_items
             SET status = $3, confirmed_at = CASE WHEN $3 IN ('confirmed', 'done') THEN COALESCE(confirmed_at, NOW()) ELSE NULL END
             WHERE id = $1 AND retro_id = $2
             RETURNING id, retro_id, source_card_id, source_cluster_id, title, details, status, position, tags",
        )
        .bind(action_id)
        .bind(retro_id)
        .bind(status)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }
}
