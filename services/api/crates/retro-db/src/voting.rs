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
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(retro_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE cards SET state = 'revealed', updated_at = NOW() WHERE retro_id = $1")
            .bind(retro_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(retro)
    }

    pub async fn start_scheduled_retro(
        &self,
        retro_id: Uuid,
    ) -> Result<Option<RetroRecord>, sqlx::Error> {
        sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'writing'
             WHERE id = $1 AND phase = 'scheduled'
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email,
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
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email,
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
