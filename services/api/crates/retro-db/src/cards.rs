use uuid::Uuid;

use crate::*;

impl RetroRepository {
    pub async fn create_draft_card(
        &self,
        input: DraftCardInput,
    ) -> Result<CardRecord, sqlx::Error> {
        let participant_id = self
            .ensure_participant(
                input.retro_id,
                &input.author_subject,
                &input.author_display_name,
            )
            .await?;

        sqlx::query_as::<_, CardRecord>(
            "INSERT INTO cards (retro_id, column_id, author_participant_id, body_text, gif_url, gif_alt_text, state, position)
             VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                (SELECT CASE WHEN phase = 'writing' THEN 'draft' ELSE 'revealed' END FROM retros WHERE id = $1),
                (SELECT COALESCE(MAX(position) + 1, 0) FROM cards WHERE retro_id = $1 AND column_id = $2)
             )
             RETURNING id, retro_id, column_id, author_participant_id, body_text, gif_url, gif_alt_text, state, position, NULL::UUID AS cluster_id, NULL::UUID AS parent_card_id, NULL::TEXT AS cluster_details, NULL::TEXT AS cluster_title, NULL::TEXT AS cluster_category, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
        )
        .bind(input.retro_id)
        .bind(input.column_id)
        .bind(participant_id)
        .bind(input.body_text.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(input.gif_url.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(input.gif_alt_text.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_draft_card(
        &self,
        card_id: Uuid,
        subject: &str,
        body_text: Option<&str>,
        gif_url: Option<&str>,
        gif_alt_text: Option<&str>,
        cluster_details: Option<&str>,
    ) -> Result<Option<CardRecord>, sqlx::Error> {
        sqlx::query_as::<_, CardRecord>(
            "UPDATE cards c
             SET body_text = CASE
                     WHEN c.cluster_id IS NOT NULL AND c.parent_card_id IS NULL THEN COALESCE($3, c.body_text)
                     ELSE $3
                 END,
                 gif_url = CASE WHEN c.cluster_id IS NOT NULL AND c.parent_card_id IS NULL THEN NULL ELSE $4 END,
                 gif_alt_text = CASE WHEN c.cluster_id IS NOT NULL AND c.parent_card_id IS NULL THEN NULL ELSE $5 END,
                 cluster_details = CASE WHEN c.cluster_id IS NOT NULL AND c.parent_card_id IS NULL THEN NULL ELSE $6 END,
                 updated_at = NOW()
             FROM participants p, retros r
             WHERE c.id = $1
               AND c.author_participant_id = p.id
               AND r.id = c.retro_id
               AND r.phase <> 'completed'
               AND c.parent_card_id IS NULL
               AND ((c.state = 'draft' AND p.external_subject = $2) OR c.state = 'revealed')
             RETURNING c.id, c.retro_id, c.column_id, c.author_participant_id, c.body_text, c.gif_url, c.gif_alt_text, c.state, c.position, c.cluster_id, c.parent_card_id, c.cluster_details, NULL::TEXT AS cluster_title, NULL::TEXT AS cluster_category, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
        )
        .bind(card_id)
        .bind(subject)
        .bind(body_text.map(str::trim).filter(|value| !value.is_empty()))
        .bind(gif_url.map(str::trim).filter(|value| !value.is_empty()))
        .bind(gif_alt_text.map(str::trim).filter(|value| !value.is_empty()))
        .bind(cluster_details.map(str::trim).filter(|value| !value.is_empty()))
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn move_draft_card(
        &self,
        retro_id: Uuid,
        card_id: Uuid,
        column_id: Uuid,
        before_card_id: Option<Uuid>,
        subject: &str,
    ) -> Result<Option<CardRecord>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let moved = sqlx::query_as::<_, CardRecord>(
            "UPDATE cards c
             SET column_id = $3,
	                 position = (
	                     SELECT COALESCE(MAX(position) + 1, 0)
	                     FROM cards
	                     WHERE retro_id = $4 AND column_id = $3
	                 ),
                 updated_at = NOW()
             FROM participants p, retro_columns rc, retros r
             WHERE c.id = $1
               AND c.author_participant_id = p.id
               AND c.retro_id = $4
               AND r.id = c.retro_id
               AND rc.id = $3
               AND rc.retro_id = $4
               AND r.phase <> 'completed'
               AND (c.state = 'revealed' OR (c.state = 'draft' AND p.external_subject = $2))
             RETURNING c.id, c.retro_id, c.column_id, c.author_participant_id, c.body_text, c.gif_url, c.gif_alt_text, c.state, c.position, c.cluster_id, c.parent_card_id, c.cluster_details, NULL::TEXT AS cluster_title, NULL::TEXT AS cluster_category, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
        )
        .bind(card_id)
        .bind(subject)
        .bind(column_id)
        .bind(retro_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(_) = moved else {
            tx.rollback().await?;
            return Ok(None);
        };

        let existing_card_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT id
             FROM cards
             WHERE retro_id = $1
               AND column_id = $2
               AND id <> $3
             ORDER BY position ASC, id ASC",
        )
        .bind(retro_id)
        .bind(column_id)
        .bind(card_id)
        .fetch_all(&mut *tx)
        .await?;

        let mut ordered_card_ids = Vec::with_capacity(existing_card_ids.len() + 1);
        let mut inserted = false;
        for existing_card_id in existing_card_ids {
            if Some(existing_card_id) == before_card_id {
                ordered_card_ids.push(card_id);
                inserted = true;
            }
            ordered_card_ids.push(existing_card_id);
        }
        if !inserted {
            ordered_card_ids.push(card_id);
        }

        for (position, ordered_card_id) in ordered_card_ids.iter().enumerate() {
            sqlx::query("UPDATE cards SET position = $1, updated_at = NOW() WHERE id = $2")
                .bind(position as i32)
                .bind(ordered_card_id)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query(
            "UPDATE cards SET column_id = $1, updated_at = NOW() WHERE parent_card_id = $2",
        )
        .bind(column_id)
        .bind(card_id)
        .execute(&mut *tx)
        .await?;

        let moved = sqlx::query_as::<_, CardRecord>(
            "SELECT c.id, c.retro_id, c.column_id, c.author_participant_id, c.body_text, c.gif_url, c.gif_alt_text, c.state, c.position, c.cluster_id, c.parent_card_id, c.cluster_details, NULL::TEXT AS cluster_title, NULL::TEXT AS cluster_category, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden
             FROM cards c
             WHERE c.id = $1",
        )
        .bind(card_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(moved))
    }

    pub async fn delete_draft_card(
        &self,
        card_id: Uuid,
        subject: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM cards c
             USING participants p, retros r
             WHERE c.id = $1
               AND c.author_participant_id = p.id
               AND r.id = c.retro_id
               AND r.phase <> 'completed'
               AND (
                 (c.state = 'draft' AND p.external_subject = $2)
                 OR (c.state = 'revealed' AND c.parent_card_id IS NULL)
               )",
        )
        .bind(card_id)
        .bind(subject)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn remove_cluster_member(
        &self,
        retro_id: Uuid,
        member_card_id: Uuid,
    ) -> Result<Option<CardRecord>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let removed = sqlx::query_as::<_, CardRecord>(
            "UPDATE cards member
             SET parent_card_id = NULL,
                 cluster_id = NULL,
                 column_id = parent.column_id,
                 position = (
                    SELECT COALESCE(MAX(position) + 1, 0)
                    FROM cards
                    WHERE retro_id = $1 AND column_id = parent.column_id AND parent_card_id IS NULL
                 ),
                 updated_at = NOW()
             FROM cards parent, retros r
             WHERE member.id = $2
               AND member.retro_id = $1
               AND parent.id = member.parent_card_id
               AND r.id = member.retro_id
               AND r.phase <> 'completed'
             RETURNING member.id, member.retro_id, member.column_id, member.author_participant_id, member.body_text, member.gif_url, member.gif_alt_text, member.state, member.position, member.cluster_id, member.parent_card_id, member.cluster_details, NULL::TEXT AS cluster_title, NULL::TEXT AS cluster_category, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
        )
        .bind(retro_id)
        .bind(member_card_id)
        .fetch_optional(&mut *tx)
        .await?;

        sqlx::query(
            "WITH singleton_groups AS (
                 SELECT parent.id, parent.column_id
                 FROM cards parent
                 JOIN cards child ON child.parent_card_id = parent.id
                 WHERE parent.retro_id = $1
                   AND parent.cluster_id IS NOT NULL
                   AND parent.parent_card_id IS NULL
                 GROUP BY parent.id, parent.column_id
                 HAVING COUNT(child.id) = 1
             )
             UPDATE cards member
             SET parent_card_id = NULL,
                 cluster_id = NULL,
                 column_id = singleton_groups.column_id,
                 position = (
                     SELECT COALESCE(MAX(position) + 1, 0)
                     FROM cards
                     WHERE retro_id = $1
                       AND column_id = singleton_groups.column_id
                       AND parent_card_id IS NULL
                 ),
                 updated_at = NOW()
             FROM singleton_groups
             WHERE member.retro_id = $1
               AND member.parent_card_id = singleton_groups.id",
        )
        .bind(retro_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "DELETE FROM cards parent
             WHERE parent.retro_id = $1
               AND parent.cluster_id IS NOT NULL
               AND parent.parent_card_id IS NULL
               AND NOT EXISTS (
                   SELECT 1 FROM cards child WHERE child.parent_card_id = parent.id
               )",
        )
        .bind(retro_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(removed)
    }
}
