use uuid::Uuid;

use crate::*;

impl RetroRepository {
    pub async fn mark_ready(
        &self,
        retro_id: Uuid,
        subject: &str,
        display_name: &str,
    ) -> Result<(), sqlx::Error> {
        let participant_id = self
            .ensure_participant(retro_id, subject, display_name)
            .await?;
        sqlx::query(
            "INSERT INTO participant_ready_marks (participant_id, retro_id, phase)
             SELECT
                $1,
                $2,
                CASE WHEN phase = 'voting' THEN 'voting' ELSE 'writing' END
             FROM retros
             WHERE id = $2 AND phase IN ('writing', 'voting')
             ON CONFLICT (participant_id, phase) DO NOTHING",
        )
        .bind(participant_id)
        .bind(retro_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn unmark_ready(&self, retro_id: Uuid, subject: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM participant_ready_marks m
             USING participants p, retros r
             WHERE m.participant_id = p.id
               AND m.retro_id = r.id
               AND p.external_subject = $1
               AND m.retro_id = $2
               AND m.phase = CASE WHEN r.phase = 'voting' THEN 'voting' ELSE 'writing' END",
        )
        .bind(subject)
        .bind(retro_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn reveal_board(&self, retro_id: Uuid) -> Result<RetroRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let retro = sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'discussion'
             WHERE id = $1 AND phase = 'writing'
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                card_edit_policy, anonymous_authors, reveal_mode,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(retro_id)
        .fetch_one(&mut *tx)
        .await?;

        // big_bang reveals on writing->discussion; per_column waits for the
        // host to reveal each column individually during discussion.
        if retro.reveal_mode == "big_bang" {
            sqlx::query(
                "UPDATE cards SET state = 'revealed', updated_at = NOW() WHERE retro_id = $1",
            )
            .bind(retro_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE retro_columns SET revealed_at = NOW()
                 WHERE retro_id = $1 AND revealed_at IS NULL",
            )
            .bind(retro_id)
            .execute(&mut *tx)
            .await?;
            order_cards_by_author(&mut tx, retro_id, None).await?;
        }

        tx.commit().await?;
        Ok(retro)
    }

    /// Reveal a single column during discussion. Flips that column's drafts
    /// to 'revealed', stamps `retro_columns.revealed_at`, re-sorts by author
    /// block. No phase transition -- reveal is a presentation action inside
    /// the existing phases, not its own step.
    ///
    /// Disambiguates unknown-column (`NotFound`) from already-revealed
    /// (`AlreadyRevealed`) so the workflow layer can return a quiet 204
    /// on host double-clicks while still 404-ing on typoed IDs.
    pub async fn reveal_column(
        &self,
        retro_id: Uuid,
        column_id: Uuid,
    ) -> Result<RevealColumnOutcome, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM retros WHERE id = $1 FOR UPDATE")
            .bind(retro_id)
            .fetch_optional(&mut *tx)
            .await?;

        let stamped = sqlx::query_scalar::<_, Uuid>(
            "UPDATE retro_columns
             SET revealed_at = NOW()
             WHERE id = $2 AND retro_id = $1 AND revealed_at IS NULL
             RETURNING id",
        )
        .bind(retro_id)
        .bind(column_id)
        .fetch_optional(&mut *tx)
        .await?;

        if stamped.is_none() {
            // No stamp -> column either doesn't exist on this retro or was
            // already revealed. One follow-up SELECT disambiguates.
            let exists = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM retro_columns WHERE id = $2 AND retro_id = $1",
            )
            .bind(retro_id)
            .bind(column_id)
            .fetch_optional(&mut *tx)
            .await?;
            tx.rollback().await?;
            return Ok(if exists.is_some() {
                RevealColumnOutcome::AlreadyRevealed
            } else {
                RevealColumnOutcome::NotFound
            });
        }

        sqlx::query(
            "UPDATE cards SET state = 'revealed', updated_at = NOW()
             WHERE retro_id = $1 AND column_id = $2 AND state = 'draft'",
        )
        .bind(retro_id)
        .bind(column_id)
        .execute(&mut *tx)
        .await?;

        order_cards_by_author(&mut tx, retro_id, Some(column_id)).await?;

        tx.commit().await?;
        Ok(RevealColumnOutcome::Revealed)
    }

    pub async fn start_scheduled_retro(
        &self,
        retro_id: Uuid,
    ) -> Result<Option<RetroRecord>, sqlx::Error> {
        sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'writing'
             WHERE id = $1 AND phase = 'scheduled'
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                card_edit_policy, anonymous_authors, reveal_mode,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn start_voting(&self, retro_id: Uuid) -> Result<RetroRecord, VotingError> {
        let retro = sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'voting'
             WHERE id = $1 AND phase = 'discussion' AND vote_limit > 0
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                card_edit_policy, anonymous_authors, reveal_mode,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(retro_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(retro)
    }

    pub async fn cast_vote(&self, input: CastVoteInput) -> Result<VotingInfo, VotingError> {
        let participant_id = self
            .ensure_participant(input.retro_id, &input.subject, &input.display_name)
            .await?;
        let retro = self
            .fetch_retro(input.retro_id)
            .await?
            .ok_or_else(|| VotingError::Invalid("retro not found".to_owned()))?;

        if retro.phase != "voting" {
            return Err(VotingError::Invalid(
                "retro is not in voting phase".to_owned(),
            ));
        }
        if input.count <= 0 {
            return Err(VotingError::Invalid(
                "vote count must be positive".to_owned(),
            ));
        }

        let target = sqlx::query_as::<_, VoteTarget>(
            "SELECT id FROM cards WHERE id = $1 AND retro_id = $2 AND state = 'revealed'",
        )
        .bind(input.card_id)
        .bind(input.retro_id)
        .fetch_optional(&self.pool)
        .await?;
        if target.is_none() {
            return Err(VotingError::Invalid(
                "vote target is not available".to_owned(),
            ));
        }

        let used = self.votes_used(input.retro_id, participant_id).await?;
        let attempted = used + input.count;
        if attempted > retro.vote_limit {
            return Err(VotingError::Invalid("vote limit exceeded".to_owned()));
        }

        sqlx::query(
            "INSERT INTO votes (retro_id, participant_id, target_card_id, count)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(input.retro_id)
        .bind(participant_id)
        .bind(input.card_id)
        .bind(input.count)
        .execute(&self.pool)
        .await?;

        Ok(VotingInfo {
            vote_limit: retro.vote_limit,
            votes_used: attempted as i64,
            votes_remaining: retro.vote_limit - attempted,
        })
    }

    pub async fn remove_vote(
        &self,
        retro_id: Uuid,
        card_id: Uuid,
        subject: &str,
        display_name: &str,
    ) -> Result<VotingInfo, VotingError> {
        let participant_id = self
            .ensure_participant(retro_id, subject, display_name)
            .await?;
        let retro = self
            .fetch_retro(retro_id)
            .await?
            .ok_or_else(|| VotingError::Invalid("retro not found".to_owned()))?;
        if retro.phase != "voting" {
            return Err(VotingError::Invalid(
                "retro is not in voting phase".to_owned(),
            ));
        }

        sqlx::query(
            "DELETE FROM votes
             WHERE id = (
                SELECT id FROM votes
                WHERE retro_id = $1 AND participant_id = $2 AND target_card_id = $3
                ORDER BY created_at DESC
                LIMIT 1
             )",
        )
        .bind(retro_id)
        .bind(participant_id)
        .bind(card_id)
        .execute(&self.pool)
        .await?;

        self.voting_info(retro_id, subject)
            .await
            .map_err(Into::into)
    }
}

/// Rewrite `cards.position` per column so revealed cards form contiguous author
/// blocks. The block order is a per-retro pseudo-random shuffle (stable across
/// columns within a retro) and is reversed on odd-indexed columns, so the same
/// author is not always read first. Cards within an author block keep their
/// chronological order. Runs once, on reveal; later drag-drop/clustering own
/// `position` afterwards.
async fn order_cards_by_author(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    retro_id: Uuid,
    only_column_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    let column_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM retro_columns WHERE retro_id = $1 ORDER BY position ASC",
    )
    .bind(retro_id)
    .fetch_all(&mut **tx)
    .await?;

    // (card_id, column_id, author_id) in chronological order within each column.
    let cards = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(
        "SELECT id, column_id, author_participant_id
         FROM cards
         WHERE retro_id = $1 AND state = 'revealed'
         ORDER BY column_id, position, created_at, id",
    )
    .bind(retro_id)
    .fetch_all(&mut **tx)
    .await?;

    for (index, column_id) in column_ids.iter().enumerate() {
        if only_column_id.is_some_and(|only_column_id| only_column_id != *column_id) {
            continue;
        }
        // author -> card ids, insertion order = chronological.
        let mut blocks: Vec<(Uuid, Vec<Uuid>)> = Vec::new();
        for (card_id, card_column, author_id) in &cards {
            if card_column != column_id {
                continue;
            }
            match blocks.iter_mut().find(|(author, _)| author == author_id) {
                Some((_, ids)) => ids.push(*card_id),
                None => blocks.push((*author_id, vec![*card_id])),
            }
        }
        if blocks.is_empty() {
            continue;
        }
        blocks.sort_by_key(|(author_id, _)| (author_rank(retro_id, *author_id), *author_id));
        if index % 2 == 1 {
            blocks.reverse();
        }
        let mut position: i32 = 0;
        for (_, ids) in &blocks {
            for card_id in ids {
                sqlx::query("UPDATE cards SET position = $1, updated_at = NOW() WHERE id = $2")
                    .bind(position)
                    .bind(card_id)
                    .execute(&mut **tx)
                    .await?;
                position += 1;
            }
        }
    }
    Ok(())
}

/// FNV-1a over the retro and participant UUIDs: a deterministic,
/// version-independent pseudo-random key. Same retro yields a stable author
/// order; different retros differ.
fn author_rank(retro_id: Uuid, participant_id: Uuid) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in retro_id.as_bytes().iter().chain(participant_id.as_bytes()) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
