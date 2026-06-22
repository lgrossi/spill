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
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                card_edit_policy,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
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
                   AND NOT EXISTS (
                       SELECT 1 FROM action_items ai
                       WHERE ai.source_card_id = c.id
                   )
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

        // Manually added actions-column cards are actions too. Backfill an
        // action_item for any revealed top-level actions-column card that does
        // not have one yet (e.g. cards added during writing/voting), so every
        // action is a single concept from here on.
        let manual_card_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT c.id
             FROM cards c
             JOIN retro_columns col ON col.id = c.column_id
             WHERE c.retro_id = $1
               AND (col.column_key = 'actions' OR lower(col.title) LIKE '%action%')
               AND c.state = 'revealed'
               AND c.parent_card_id IS NULL
               AND NOT EXISTS (
                   SELECT 1 FROM action_items ai
                   WHERE ai.retro_id = $1 AND ai.source_card_id = c.id
               )
             ORDER BY c.position, c.created_at",
        )
        .bind(retro_id)
        .fetch_all(&mut *tx)
        .await?;
        for card_id in manual_card_ids {
            ensure_action_item_for_card(&mut tx, retro_id, card_id).await?;
        }

        tx.commit().await?;
        Ok(self.fetch_actions(retro_id).await?)
    }

    pub async fn complete_retro(&self, retro_id: Uuid) -> Result<Option<RetroRecord>, ActionError> {
        let retro = sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'completed', completed_at = NOW(), happened_at = NOW()
             WHERE id = $1 AND phase = 'action_discussion'
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                card_edit_policy,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
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

/// Ensure a revealed, top-level card sitting in an actions column has a backing
/// action_item. Manually added action cards become first-class actions this way,
/// exactly like the auto top-voted cards already do. No-op if the card is not an
/// eligible actions-column card or already has an action_item.
pub(crate) async fn ensure_action_item_for_card(
    conn: &mut sqlx::PgConnection,
    retro_id: Uuid,
    card_id: Uuid,
) -> Result<(), sqlx::Error> {
    let title = sqlx::query_scalar::<_, String>(
        "SELECT COALESCE(NULLIF(btrim(c.body_text), ''), c.gif_alt_text, 'Untitled action')
         FROM cards c
         JOIN retro_columns col ON col.id = c.column_id
         WHERE c.id = $2
           AND c.retro_id = $1
           AND (col.column_key = 'actions' OR lower(col.title) LIKE '%action%')
           AND c.state = 'revealed'
           AND c.parent_card_id IS NULL
           AND NOT EXISTS (
               SELECT 1 FROM action_items ai
               WHERE ai.retro_id = $1 AND ai.source_card_id = c.id
           )",
    )
    .bind(retro_id)
    .bind(card_id)
    .fetch_optional(&mut *conn)
    .await?;

    let Some(title) = title else {
        return Ok(());
    };

    let tags = action_tags(&title);
    sqlx::query(
        "INSERT INTO action_items (retro_id, source_card_id, title, details, status, position, tags)
         VALUES (
            $1,
            $2,
            $3,
            NULL,
            'confirmed',
            (SELECT COALESCE(MAX(position) + 1, 0) FROM action_items WHERE retro_id = $1),
            $4
         )",
    )
    .bind(retro_id)
    .bind(card_id)
    .bind(&title)
    .bind(Json(tags))
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Drop the open action_item linked to a card. Used when a card stops being an
/// action (moved out of the Actions column, or pulled into a cluster). Done and
/// rejected actions are kept so completed outcomes are not lost.
pub(crate) async fn discard_open_action_item_for_card(
    conn: &mut sqlx::PgConnection,
    card_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM action_items
         WHERE source_card_id = $1 AND status NOT IN ('done', 'rejected')",
    )
    .bind(card_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Reconcile a card's action_item with its current placement: ensure one exists
/// while the card is a revealed, top-level Actions-column card, and drop the open
/// one once it no longer is. Title is left untouched here so reordering or moving
/// a card never rewrites an action's text.
pub(crate) async fn reconcile_action_item_for_card(
    conn: &mut sqlx::PgConnection,
    retro_id: Uuid,
    card_id: Uuid,
) -> Result<(), sqlx::Error> {
    let eligible = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1
            FROM cards c
            JOIN retro_columns col ON col.id = c.column_id
            WHERE c.id = $2
              AND c.retro_id = $1
              AND (col.column_key = 'actions' OR lower(col.title) LIKE '%action%')
              AND c.state = 'revealed'
              AND c.parent_card_id IS NULL
         )",
    )
    .bind(retro_id)
    .bind(card_id)
    .fetch_one(&mut *conn)
    .await?;

    if eligible {
        ensure_action_item_for_card(conn, retro_id, card_id).await
    } else {
        discard_open_action_item_for_card(conn, card_id).await
    }
}

/// Keep a card-backed action's title (and tags) in step with edits to its
/// source card, so the open action and wrap-up never show stale text.
pub(crate) async fn sync_action_item_title_for_card(
    conn: &mut sqlx::PgConnection,
    card_id: Uuid,
    previous_title: Option<&str>,
) -> Result<(), sqlx::Error> {
    let Some(previous_title) = previous_title else {
        return Ok(());
    };
    let updated = sqlx::query_scalar::<_, String>(
        "UPDATE action_items ai
         SET title = COALESCE(NULLIF(btrim(c.body_text), ''), c.gif_alt_text, 'Untitled action')
         FROM cards c
         WHERE ai.source_card_id = $1
           AND c.id = ai.source_card_id
           AND ai.status NOT IN ('done', 'rejected')
           AND ai.title = $2
         RETURNING ai.title",
    )
    .bind(card_id)
    .bind(previous_title)
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(title) = updated {
        sqlx::query("UPDATE action_items SET tags = $2 WHERE source_card_id = $1")
            .bind(card_id)
            .bind(Json(action_tags(&title)))
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}
