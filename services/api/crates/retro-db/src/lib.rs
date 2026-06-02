use hex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

use board_read_model::attach_cards_to_columns;
mod actions;

mod artifacts;
mod board_read_model;
mod cards;
mod clustering;
mod creation;
mod domain_mapping;
mod ingestion;
mod overview;
mod series;
mod voting;

fn email_subject(email: &str) -> String {
    let hash = hex::encode(Sha256::digest(email.trim().to_lowercase()));
    format!("email:{hash}")
}

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

#[derive(Clone)]
pub struct RetroRepository {
    pool: PgPool,
}

impl RetroRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn fetch_retro(&self, id: Uuid) -> Result<Option<RetroRecord>, sqlx::Error> {
        sqlx::query_as::<_, RetroRecord>(
            "SELECT id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                clustering_mode, clustering_status,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at
             FROM retros
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete_retro(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM retros WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn reschedule_scheduled_retro(
        &self,
        id: Uuid,
        planned_for: &str,
    ) -> Result<Option<RetroRecord>, sqlx::Error> {
        sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET planned_for = $2::date
             WHERE id = $1 AND phase = 'scheduled'
             RETURNING id, title, phase, vote_limit, action_discussion_limit, creator_email, cover_gif_url, cover_gif_alt_text,
                to_char(planned_for, 'YYYY-MM-DD') AS planned_for,
                to_char(happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at",
        )
        .bind(id)
        .bind(planned_for)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn is_scheduled_retro_due(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1
                FROM retros
                WHERE id = $1 AND phase = 'scheduled' AND planned_for <= CURRENT_DATE
            )",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn create_retro(&self, input: CreateRetroInput) -> Result<RetroBoard, sqlx::Error> {
        creation::create_retro(&self.pool, input).await
    }

    pub async fn fetch_board(&self, id: Uuid) -> Result<Option<RetroBoard>, sqlx::Error> {
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let participants = self.fetch_participants(id).await?;
        let columns = self.fetch_columns(id).await?;
        let clusters = self.fetch_clusters(id).await?;
        let actions = self.fetch_actions(id).await?;
        let deck = Vec::new();
        let ai_artifacts = self.fetch_ai_artifacts(id).await?;
        let meeting_notes = self.fetch_meeting_notes(id).await?;
        let deliveries = self.fetch_deliveries(id).await?;
        let ready = self.ready_info(id, "").await?;
        let voting = self.voting_info(id, "").await?;
        let series = self.fetch_series(id).await?;
        let next_retro = self.fetch_next_retro(id).await?;
        Ok(Some(RetroBoard {
            retro,
            series,
            next_retro,
            participants,
            columns,
            ready,
            voting,
            clusters,
            actions,
            deck,
            ai_artifacts,
            meeting_notes,
            deliveries,
        }))
    }

    pub async fn fetch_board_for_user(
        &self,
        id: Uuid,
        subject: &str,
        display_name: &str,
    ) -> Result<Option<RetroBoard>, sqlx::Error> {
        self.fetch_board_for_user_with_email(id, subject, display_name, "")
            .await
    }

    pub async fn fetch_board_for_user_with_email(
        &self,
        id: Uuid,
        subject: &str,
        display_name: &str,
        email: &str,
    ) -> Result<Option<RetroBoard>, sqlx::Error> {
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let participant_id = self.ensure_participant(id, subject, display_name).await?;
        self.record_retro_access(id, participant_id).await?;
        let participants = self.fetch_participants(id).await?;
        let mut columns = self.fetch_columns(id).await?;
        let clusters = self.fetch_clusters(id).await?;
        let cards = self.fetch_cards_for_user(id, subject).await?;
        let actions = self.fetch_actions(id).await?;
        let deck = self.fetch_deck(id, subject).await?;
        let ai_artifacts = self.fetch_ai_artifacts(id).await?;
        let meeting_notes = self.fetch_meeting_notes(id).await?;
        let deliveries = self.fetch_deliveries(id).await?;
        attach_cards_to_columns(&mut columns, cards);
        let ready = self.ready_info(id, subject).await?;
        let voting = self.voting_info(id, subject).await?;
        let series = self.fetch_series(id).await?;
        let next_retro = if let Some(next_retro) = self.fetch_next_retro(id).await? {
            if email.trim().is_empty() || self.is_board_member(next_retro.id, email).await? {
                Some(next_retro)
            } else {
                None
            }
        } else {
            None
        };
        Ok(Some(RetroBoard {
            retro,
            series,
            next_retro,
            participants,
            columns,
            ready,
            voting,
            clusters,
            actions,
            deck,
            ai_artifacts,
            meeting_notes,
            deliveries,
        }))
    }

    /// Side-effect-free board fetch used by background jobs (AI
    /// summary runner today). Skips the participant insert and the
    /// access record that `fetch_board_for_user` performs, so calling
    /// it does not pollute the participant list with synthetic
    /// service identities.
    ///
    /// User-specific projections (`ready`, `voting`, `deck`, and the
    /// per-user card masking) collapse on a non-`writing` retro to
    /// what an anonymous viewer would see — the empty subject here is
    /// safe for any completed retro, which is the only state in which
    /// the runner is expected to be called.
    pub async fn fetch_board_readonly(&self, id: Uuid) -> Result<Option<RetroBoard>, sqlx::Error> {
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let participants = self.fetch_participants(id).await?;
        let mut columns = self.fetch_columns(id).await?;
        let clusters = self.fetch_clusters(id).await?;
        let cards = self.fetch_cards_for_user(id, "").await?;
        let actions = self.fetch_actions(id).await?;
        let deck = self.fetch_deck(id, "").await?;
        let ai_artifacts = self.fetch_ai_artifacts(id).await?;
        let meeting_notes = self.fetch_meeting_notes(id).await?;
        let deliveries = self.fetch_deliveries(id).await?;
        attach_cards_to_columns(&mut columns, cards);
        let ready = self.ready_info(id, "").await?;
        let voting = self.voting_info(id, "").await?;
        let series = self.fetch_series(id).await?;
        let next_retro = self.fetch_next_retro(id).await?;
        Ok(Some(RetroBoard {
            retro,
            series,
            next_retro,
            participants,
            columns,
            ready,
            voting,
            clusters,
            actions,
            deck,
            ai_artifacts,
            meeting_notes,
            deliveries,
        }))
    }

    pub async fn fetch_columns(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<RetroColumnRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, RetroColumnRow>(
            "SELECT id, retro_id, column_key, title, position, order_direction, accent_color
             FROM retro_columns
             WHERE retro_id = $1
             ORDER BY position ASC",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn fetch_clusters(&self, retro_id: Uuid) -> Result<Vec<ClusterRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ClusterRow>(
            "SELECT id, retro_id, title, category, tags
             FROM card_clusters
             WHERE retro_id = $1
             ORDER BY created_at, id",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_retros(&self, subject: &str) -> Result<RetroOverview, sqlx::Error> {
        self.list_retros_for_user(subject, "").await
    }

    pub async fn list_retros_for_user(
        &self,
        subject: &str,
        email: &str,
    ) -> Result<RetroOverview, sqlx::Error> {
        overview::list_retros(&self.pool, subject, email).await
    }

    pub async fn authorize_retro_participant(
        &self,
        retro_id: Uuid,
        subject: &str,
        display_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let participant_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO participants (retro_id, external_subject, display_name, role)
             SELECT $1, $2, $3, 'member'
             WHERE EXISTS (SELECT 1 FROM retros WHERE id = $1)
             ON CONFLICT (retro_id, external_subject)
             DO UPDATE
             SET display_name = EXCLUDED.display_name
             RETURNING id",
        )
        .bind(retro_id)
        .bind(subject)
        .bind(display_name.trim())
        .fetch_optional(&self.pool)
        .await?;

        let Some(participant_id) = participant_id else {
            return Ok(false);
        };

        self.record_retro_access(retro_id, participant_id).await?;
        Ok(true)
    }

    pub async fn is_board_member(&self, retro_id: Uuid, email: &str) -> Result<bool, sqlx::Error> {
        // The creator's email is inserted into board_grants as 'host' on retro creation.
        // All other members are inserted when invited. One query covers both cases.
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM board_grants
                WHERE retro_id = $1 AND principal_email = lower($2)
            )",
        )
        .bind(retro_id)
        .bind(email)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn is_retro_host(
        &self,
        retro_id: Uuid,
        subject: &str,
        email: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1
                FROM participants
                WHERE retro_id = $1 AND external_subject = $2 AND role = 'host'
            ) OR EXISTS (
                SELECT 1
                FROM board_grants
                WHERE retro_id = $1 AND principal_email = lower($3) AND role = 'host'
            )",
        )
        .bind(retro_id)
        .bind(subject.trim())
        .bind(email.trim())
        .fetch_one(&self.pool)
        .await
    }

    pub async fn add_board_grant(
        &self,
        retro_id: Uuid,
        principal_email: &str,
        role: &str,
    ) -> Result<BoardGrant, sqlx::Error> {
        sqlx::query_as::<_, BoardGrant>(
            "INSERT INTO board_grants (retro_id, principal_email, role)
             VALUES ($1, lower($2), $3)
             ON CONFLICT (retro_id, principal_email) DO UPDATE
               SET role = CASE WHEN board_grants.role = 'host' THEN board_grants.role ELSE EXCLUDED.role END
             RETURNING id, retro_id, principal_email, role",
        )
        .bind(retro_id)
        .bind(principal_email)
        .bind(role)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn remove_board_grant(
        &self,
        retro_id: Uuid,
        principal_email: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM board_grants WHERE retro_id = $1 AND principal_email = lower($2)",
        )
        .bind(retro_id)
        .bind(principal_email)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_board_grants(&self, retro_id: Uuid) -> Result<Vec<BoardGrant>, sqlx::Error> {
        sqlx::query_as::<_, BoardGrant>(
            "SELECT id, retro_id, principal_email, role
             FROM board_grants
             WHERE retro_id = $1
             ORDER BY created_at ASC",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn set_clustering_mode(&self, retro_id: Uuid, mode: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE retros
             SET clustering_mode = CASE WHEN $2 = 'auto_on_vote_start' THEN 'auto_on_vote_start' ELSE 'disabled' END,
                 clustering_status = 'not_run'
             WHERE id = $1",
        )
        .bind(retro_id)
        .bind(mode.trim())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn existing_cluster_tag_context(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            "WITH source AS (
                SELECT group_id FROM retros WHERE id = $1
             ),
             tags AS (
                SELECT jsonb_array_elements_text(cc.tags) AS tag
                FROM card_clusters cc
                JOIN retros r ON r.id = cc.retro_id
                JOIN source s ON s.group_id IS NOT NULL AND s.group_id = r.group_id
                WHERE cc.retro_id <> $1
             )
             SELECT tag
             FROM tags
             WHERE btrim(tag) <> ''
             GROUP BY tag
             ORDER BY COUNT(*) DESC, tag ASC
             LIMIT 50",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn existing_board_category_context(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            "WITH source AS (
                SELECT group_id FROM retros WHERE id = $1
             ),
             categories AS (
                SELECT jsonb_array_elements_text(a.output->'board_categories') AS category
                FROM ai_artifacts a
                JOIN retros r ON r.id = a.retro_id
                JOIN source s ON s.group_id IS NOT NULL AND s.group_id = r.group_id
                WHERE a.retro_id <> $1
                  AND a.kind = 'summary'
                  AND a.status = 'succeeded'
                  AND jsonb_typeof(a.output->'board_categories') = 'array'
             )
             SELECT category
             FROM categories
             WHERE btrim(category) <> ''
             GROUP BY category
             ORDER BY COUNT(*) DESC, category ASC
             LIMIT 50",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Removes the participant row whose subject matches the given email.
    /// Called after removing a board grant so the person is evicted from the
    /// live session immediately.
    pub async fn remove_participant_by_email(
        &self,
        retro_id: Uuid,
        email: &str,
    ) -> Result<bool, sqlx::Error> {
        let subject = email_subject(email);
        let result =
            sqlx::query("DELETE FROM participants WHERE retro_id = $1 AND external_subject = $2")
                .bind(retro_id)
                .bind(subject)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn remove_participant(
        &self,
        retro_id: Uuid,
        subject: &str,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM participants WHERE retro_id = $1 AND external_subject = $2")
                .bind(retro_id)
                .bind(subject)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn ensure_participant(
        &self,
        retro_id: Uuid,
        subject: &str,
        display_name: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let record = sqlx::query_as::<_, ParticipantId>(
            "INSERT INTO participants (retro_id, external_subject, display_name, role)
             VALUES ($1, $2, $3, 'member')
             ON CONFLICT (retro_id, external_subject) DO UPDATE
             SET display_name = EXCLUDED.display_name
             RETURNING id",
        )
        .bind(retro_id)
        .bind(subject.trim())
        .bind(display_name.trim())
        .fetch_one(&self.pool)
        .await?;

        Ok(record.id)
    }

    async fn record_retro_access(
        &self,
        retro_id: Uuid,
        participant_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO retro_accesses (retro_id, participant_id, opened_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (retro_id, participant_id) DO UPDATE
             SET opened_at = EXCLUDED.opened_at",
        )
        .bind(retro_id)
        .bind(participant_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn fetch_participants(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<ParticipantRecord>, sqlx::Error> {
        sqlx::query_as::<_, ParticipantRecord>(
            "SELECT
                p.id,
                p.retro_id,
                p.external_subject,
                p.display_name,
                p.role,
                (
                    SELECT COUNT(*)::BIGINT
                    FROM cards c
                    WHERE c.retro_id = p.retro_id
                      AND c.author_participant_id = p.id
                      AND c.state = 'revealed'
                ) AS card_count,
                (
                    SELECT COALESCE(SUM(v.count), 0)::BIGINT
                    FROM votes v
                    WHERE v.retro_id = p.retro_id
                      AND v.participant_id = p.id
                ) AS vote_count
             FROM participants p
             WHERE p.retro_id = $1
             ORDER BY CASE p.role WHEN 'host' THEN 0 ELSE 1 END, p.created_at ASC, p.id ASC",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn fetch_cards_for_user(
        &self,
        retro_id: Uuid,
        subject: &str,
    ) -> Result<Vec<CardRecord>, sqlx::Error> {
        sqlx::query_as::<_, CardRecord>(
            "SELECT
                c.id,
                c.retro_id,
                c.column_id,
                c.author_participant_id,
                CASE
                    WHEN r.phase = 'writing' AND c.state = 'draft' AND p.external_subject IS DISTINCT FROM $2 THEN NULL
                    ELSE c.body_text
                END AS body_text,
                CASE
                    WHEN r.phase = 'writing' AND c.state = 'draft' AND p.external_subject IS DISTINCT FROM $2 THEN NULL
                    ELSE c.gif_url
                END AS gif_url,
                CASE
                    WHEN r.phase = 'writing' AND c.state = 'draft' AND p.external_subject IS DISTINCT FROM $2 THEN NULL
                    ELSE c.gif_alt_text
                END AS gif_alt_text,
                c.state,
                c.position,
                c.cluster_id,
                c.parent_card_id,
                c.cluster_details,
                cc.title AS cluster_title,
                cc.category AS cluster_category,
                COALESCE(SUM(v.count), 0)::BIGINT AS vote_count,
                COALESCE(SUM(CASE WHEN vp.external_subject = $2 THEN v.count ELSE 0 END), 0)::BIGINT AS current_user_vote_count,
                (r.phase = 'writing' AND c.state = 'draft' AND p.external_subject IS DISTINCT FROM $2) AS hidden
             FROM cards c
             JOIN participants p ON p.id = c.author_participant_id
             JOIN retros r ON r.id = c.retro_id
             LEFT JOIN card_clusters cc ON cc.id = c.cluster_id
             LEFT JOIN votes v ON v.target_card_id = c.id
             LEFT JOIN participants vp ON vp.id = v.participant_id
             WHERE c.retro_id = $1
             GROUP BY c.id, r.phase, p.external_subject, cc.title, cc.category
             ORDER BY c.column_id, c.position, c.created_at",
        )
        .bind(retro_id)
        .bind(subject)
        .fetch_all(&self.pool)
        .await
    }

    async fn ready_info(&self, retro_id: Uuid, subject: &str) -> Result<ReadyInfo, sqlx::Error> {
        sqlx::query_as::<_, ReadyInfo>(
            "SELECT
                COUNT(DISTINCT p.id)::BIGINT AS participant_count,
                COUNT(DISTINCT m.participant_id)::BIGINT AS ready_count,
                COALESCE(BOOL_OR(p.external_subject = $2 AND m.participant_id IS NOT NULL), false) AS current_user_ready
             FROM participants p
             JOIN retros r ON r.id = p.retro_id
             LEFT JOIN participant_ready_marks m
                ON m.participant_id = p.id
               AND m.phase = CASE WHEN r.phase = 'voting' THEN 'voting' ELSE 'writing' END
             WHERE p.retro_id = $1",
        )
        .bind(retro_id)
        .bind(subject)
        .fetch_one(&self.pool)
        .await
    }

    async fn votes_used(&self, retro_id: Uuid, participant_id: Uuid) -> Result<i32, sqlx::Error> {
        let record = sqlx::query_as::<_, VoteCount>(
            "SELECT COALESCE(SUM(count), 0)::BIGINT AS count
             FROM votes
             WHERE retro_id = $1 AND participant_id = $2",
        )
        .bind(retro_id)
        .bind(participant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.count as i32)
    }

    async fn voting_info(&self, retro_id: Uuid, subject: &str) -> Result<VotingInfo, sqlx::Error> {
        sqlx::query_as::<_, VotingInfo>(
            "SELECT
                r.vote_limit,
                COALESCE(SUM(v.count), 0)::BIGINT AS votes_used,
                GREATEST(r.vote_limit - COALESCE(SUM(v.count), 0)::INTEGER, 0) AS votes_remaining
             FROM retros r
             LEFT JOIN participants p ON p.retro_id = r.id AND p.external_subject = $2
             LEFT JOIN votes v ON v.retro_id = r.id AND v.participant_id = p.id
             WHERE r.id = $1
             GROUP BY r.id",
        )
        .bind(retro_id)
        .bind(subject)
        .fetch_one(&self.pool)
        .await
    }

    async fn fetch_actions(&self, retro_id: Uuid) -> Result<Vec<ActionItemRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ActionItemRow>(
            "SELECT id, retro_id, source_card_id, source_cluster_id, title, details, status, position, tags
             FROM action_items
             WHERE retro_id = $1
             ORDER BY position, created_at",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_deck(
        &self,
        retro_id: Uuid,
        subject: &str,
    ) -> Result<Vec<IngestedItemRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, IngestedItemRow>(
            "SELECT i.id, i.recipient_participant_id, i.retro_id, i.source, i.placement, i.target_column_id,
                i.suggested_text, i.gif_url, i.raw_payload, i.status, i.accepted_card_id, i.idempotency_key, i.source_metadata
             FROM ingested_items i
             JOIN participants p ON p.id = i.recipient_participant_id
             WHERE i.retro_id = $1 AND p.external_subject = $2 AND i.placement = 'user_deck' AND i.status = 'pending'
             ORDER BY i.created_at DESC",
        )
        .bind(retro_id)
        .bind(subject)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_ai_artifacts(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<AiArtifactRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, AiArtifactRow>(
            "SELECT id, retro_id, kind, status, input, output, error_message, retry_count
             FROM ai_artifacts
             WHERE retro_id = $1
             ORDER BY updated_at DESC, created_at DESC",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_meeting_notes(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<MeetingNoteRecord>, sqlx::Error> {
        sqlx::query_as::<_, MeetingNoteRecord>(
            "SELECT id, retro_id, author_participant_id, title, body_text
             FROM meeting_notes
             WHERE retro_id = $1
             ORDER BY created_at DESC",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn fetch_deliveries(&self, retro_id: Uuid) -> Result<Vec<DeliveryRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, DeliveryRow>(
            "SELECT id, retro_id, kind, status, output, error_message, retry_count
             FROM deliveries
             WHERE retro_id = $1
             ORDER BY updated_at DESC, created_at DESC",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_ingested_by_idempotency(
        &self,
        participant_id: Uuid,
        source: &str,
        idempotency_key: &str,
    ) -> Result<Option<IngestedItemRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, IngestedItemRow>(
            "SELECT id, recipient_participant_id, retro_id, source, placement, target_column_id,
                suggested_text, gif_url, raw_payload, status, accepted_card_id, idempotency_key, source_metadata
             FROM ingested_items
             WHERE recipient_participant_id = $1 AND source = $2 AND idempotency_key = $3",
        )
        .bind(participant_id)
        .bind(source)
        .bind(idempotency_key.trim())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    async fn fetch_series(&self, retro_id: Uuid) -> Result<Option<RetroSeriesRecord>, sqlx::Error> {
        sqlx::query_as::<_, RetroSeriesRecord>(
            "SELECT g.id, g.name
             FROM retros r
             JOIN retro_groups g ON g.id = r.group_id
             WHERE r.id = $1",
        )
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn fetch_next_retro(
        &self,
        retro_id: Uuid,
    ) -> Result<Option<NextRetroRecord>, sqlx::Error> {
        sqlx::query_as::<_, NextRetroRecord>(
            "SELECT
                next.id,
                next.title,
                next.phase,
                to_char(next.planned_for, 'YYYY-MM-DD') AS planned_for,
                g.name AS group_name
             FROM retros next
             LEFT JOIN retro_groups g ON g.id = next.group_id
             WHERE next.previous_retro_id = $1
             ORDER BY next.created_at ASC
             LIMIT 1",
        )
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RetroRecord {
    pub id: Uuid,
    pub title: String,
    pub phase: String,
    pub vote_limit: i32,
    pub action_discussion_limit: i32,
    pub creator_email: String,
    pub cover_gif_url: Option<String>,
    pub cover_gif_alt_text: Option<String>,
    #[sqlx(default)]
    pub clustering_mode: String,
    #[sqlx(default)]
    pub clustering_status: String,
    pub planned_for: String,
    pub happened_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetroBoard {
    pub retro: RetroRecord,
    pub series: Option<RetroSeriesRecord>,
    pub next_retro: Option<NextRetroRecord>,
    pub participants: Vec<ParticipantRecord>,
    pub columns: Vec<RetroColumnRecord>,
    pub ready: ReadyInfo,
    pub voting: VotingInfo,
    pub clusters: Vec<ClusterRecord>,
    pub actions: Vec<ActionItemRecord>,
    pub deck: Vec<IngestedItemRecord>,
    pub ai_artifacts: Vec<AiArtifactRecord>,
    pub meeting_notes: Vec<MeetingNoteRecord>,
    pub deliveries: Vec<DeliveryRecord>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RetroSeriesRecord {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct NextRetroRecord {
    pub id: Uuid,
    pub title: String,
    pub phase: String,
    pub planned_for: String,
    pub group_name: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct ParticipantRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub external_subject: Option<String>,
    pub display_name: String,
    pub role: String,
    pub card_count: i64,
    pub vote_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetroColumnRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub column_key: String,
    pub title: String,
    pub position: i32,
    pub order_direction: String,
    pub accent_color: Option<String>,
    #[serde(default)]
    pub cards: Vec<CardRecord>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct RetroColumnRow {
    id: Uuid,
    retro_id: Uuid,
    column_key: String,
    title: String,
    position: i32,
    order_direction: String,
    accent_color: Option<String>,
}

impl From<RetroColumnRow> for RetroColumnRecord {
    fn from(row: RetroColumnRow) -> Self {
        Self {
            id: row.id,
            retro_id: row.retro_id,
            column_key: row.column_key,
            title: row.title,
            position: row.position,
            order_direction: row.order_direction,
            accent_color: row.accent_color,
            cards: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct CardRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub column_id: Uuid,
    pub author_participant_id: Uuid,
    pub body_text: Option<String>,
    pub gif_url: Option<String>,
    pub gif_alt_text: Option<String>,
    pub state: String,
    pub position: i32,
    pub cluster_id: Option<Uuid>,
    pub parent_card_id: Option<Uuid>,
    pub cluster_details: Option<String>,
    pub cluster_title: Option<String>,
    pub cluster_category: Option<String>,
    pub vote_count: i64,
    pub current_user_vote_count: i64,
    pub hidden: bool,
    #[serde(default)]
    #[sqlx(skip)]
    pub cluster_members: Vec<ClusterMemberRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterMemberRecord {
    pub id: Uuid,
    pub author_participant_id: Uuid,
    pub body_text: Option<String>,
    pub gif_url: Option<String>,
    pub gif_alt_text: Option<String>,
    pub vote_count: i64,
    pub hidden: bool,
}

impl From<&CardRecord> for ClusterMemberRecord {
    fn from(card: &CardRecord) -> Self {
        Self {
            id: card.id,
            author_participant_id: card.author_participant_id,
            body_text: card.body_text.clone(),
            gif_url: card.gif_url.clone(),
            gif_alt_text: card.gif_alt_text.clone(),
            vote_count: card.vote_count,
            hidden: card.hidden,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub title: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ClusterRow {
    id: Uuid,
    retro_id: Uuid,
    title: Option<String>,
    category: Option<String>,
    tags: Json<Vec<String>>,
}

impl From<ClusterRow> for ClusterRecord {
    fn from(row: ClusterRow) -> Self {
        Self {
            id: row.id,
            retro_id: row.retro_id,
            title: row.title,
            category: row.category,
            tags: row.tags.0,
        }
    }
}

#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
pub struct ReadyInfo {
    pub participant_count: i64,
    pub ready_count: i64,
    pub current_user_ready: bool,
}

#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
pub struct VotingInfo {
    pub vote_limit: i32,
    pub votes_used: i64,
    pub votes_remaining: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionItemRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub source_card_id: Option<Uuid>,
    pub source_cluster_id: Option<Uuid>,
    pub title: String,
    pub details: Option<String>,
    pub status: String,
    pub position: i32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ActionItemRow {
    id: Uuid,
    retro_id: Uuid,
    source_card_id: Option<Uuid>,
    source_cluster_id: Option<Uuid>,
    title: String,
    details: Option<String>,
    status: String,
    position: i32,
    tags: Json<Vec<String>>,
}

impl From<ActionItemRow> for ActionItemRecord {
    fn from(row: ActionItemRow) -> Self {
        Self {
            id: row.id,
            retro_id: row.retro_id,
            source_card_id: row.source_card_id,
            source_cluster_id: row.source_cluster_id,
            title: row.title,
            details: row.details,
            status: row.status,
            position: row.position,
            tags: row.tags.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestedItemRecord {
    pub id: Uuid,
    pub recipient_participant_id: Uuid,
    pub retro_id: Option<Uuid>,
    pub source: String,
    pub placement: String,
    pub target_column_id: Option<Uuid>,
    pub suggested_text: Option<String>,
    pub gif_url: Option<String>,
    pub raw_payload: Value,
    pub status: String,
    pub accepted_card_id: Option<Uuid>,
    pub idempotency_key: Option<String>,
    pub source_metadata: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct IngestedItemRow {
    id: Uuid,
    recipient_participant_id: Uuid,
    retro_id: Option<Uuid>,
    source: String,
    placement: String,
    target_column_id: Option<Uuid>,
    suggested_text: Option<String>,
    gif_url: Option<String>,
    raw_payload: Json<Value>,
    status: String,
    accepted_card_id: Option<Uuid>,
    idempotency_key: Option<String>,
    source_metadata: Json<Value>,
}

impl From<IngestedItemRow> for IngestedItemRecord {
    fn from(row: IngestedItemRow) -> Self {
        Self {
            id: row.id,
            recipient_participant_id: row.recipient_participant_id,
            retro_id: row.retro_id,
            source: row.source,
            placement: row.placement,
            target_column_id: row.target_column_id,
            suggested_text: row.suggested_text,
            gif_url: row.gif_url,
            raw_payload: row.raw_payload.0,
            status: row.status,
            accepted_card_id: row.accepted_card_id,
            idempotency_key: row.idempotency_key,
            source_metadata: row.source_metadata.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AiArtifactRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub kind: String,
    pub status: String,
    pub input: Value,
    pub output: Option<Value>,
    pub error_message: Option<String>,
    pub retry_count: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct AiArtifactRow {
    id: Uuid,
    retro_id: Uuid,
    kind: String,
    status: String,
    input: Json<Value>,
    output: Option<Json<Value>>,
    error_message: Option<String>,
    retry_count: i32,
}

impl From<AiArtifactRow> for AiArtifactRecord {
    fn from(row: AiArtifactRow) -> Self {
        Self {
            id: row.id,
            retro_id: row.retro_id,
            kind: row.kind,
            status: row.status,
            input: row.input.0,
            output: row.output.map(|output| output.0),
            error_message: row.error_message,
            retry_count: row.retry_count,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct MeetingNoteRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub author_participant_id: Uuid,
    pub title: String,
    pub body_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeliveryRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub kind: String,
    pub status: String,
    pub output: Option<Value>,
    pub error_message: Option<String>,
    pub retry_count: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct DeliveryRow {
    id: Uuid,
    retro_id: Uuid,
    kind: String,
    status: String,
    output: Option<Json<Value>>,
    error_message: Option<String>,
    retry_count: i32,
}

impl From<DeliveryRow> for DeliveryRecord {
    fn from(row: DeliveryRow) -> Self {
        Self {
            id: row.id,
            retro_id: row.retro_id,
            kind: row.kind,
            status: row.status,
            output: row.output.map(|output| output.0),
            error_message: row.error_message,
            retry_count: row.retry_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RetroSummary {
    pub id: Uuid,
    pub title: String,
    pub phase: String,
    pub vote_limit: i32,
    pub action_discussion_limit: i32,
    pub created_at: String,
    pub planned_for: String,
    pub happened_at: Option<String>,
    pub group_name: Option<String>,
    pub cover_gif_url: Option<String>,
    pub cover_gif_alt_text: Option<String>,
    pub current_user_role: String,
    pub last_activity_at: String,
    pub last_opened_at: Option<String>,
    pub participant_count: i64,
    pub ready_count: i64,
    pub column_count: i64,
    pub unresolved_action_count: i64,
    pub recurring_tags: Vec<String>,
    pub open_actions: Vec<RetroActionSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetroActionSummary {
    pub id: Uuid,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct RetroSummaryRow {
    id: Uuid,
    title: String,
    phase: String,
    vote_limit: i32,
    action_discussion_limit: i32,
    created_at: String,
    planned_for: String,
    happened_at: Option<String>,
    group_name: Option<String>,
    cover_gif_url: Option<String>,
    cover_gif_alt_text: Option<String>,
    current_user_role: String,
    last_activity_at: String,
    last_opened_at: Option<String>,
    participant_count: i64,
    ready_count: i64,
    column_count: i64,
    unresolved_action_count: i64,
    recurring_tags: Json<Vec<String>>,
    open_actions: Json<Vec<RetroActionSummary>>,
}

impl From<RetroSummaryRow> for RetroSummary {
    fn from(row: RetroSummaryRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            phase: row.phase,
            vote_limit: row.vote_limit,
            action_discussion_limit: row.action_discussion_limit,
            created_at: row.created_at,
            planned_for: row.planned_for,
            happened_at: row.happened_at,
            group_name: row.group_name,
            cover_gif_url: row.cover_gif_url,
            cover_gif_alt_text: row.cover_gif_alt_text,
            current_user_role: row.current_user_role,
            last_activity_at: row.last_activity_at,
            last_opened_at: row.last_opened_at,
            participant_count: row.participant_count,
            ready_count: row.ready_count,
            column_count: row.column_count,
            unresolved_action_count: row.unresolved_action_count,
            recurring_tags: row.recurring_tags.0,
            open_actions: row.open_actions.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RetroOverview {
    pub active: Vec<RetroSummary>,
    pub completed: Vec<RetroSummary>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct BoardGrant {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub principal_email: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRetroInput {
    pub title: String,
    pub creator_subject: String,
    pub creator_email: String,
    pub creator_display_name: String,
    pub group_name: Option<String>,
    pub cover_gif_url: Option<String>,
    pub cover_gif_alt_text: Option<String>,
    pub planned_for: Option<String>,
    pub template: RetroTemplate,
    pub vote_limit: i32,
    pub action_discussion_limit: i32,
    pub column_colors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateRetroDetailsInput {
    pub retro_id: Uuid,
    pub title: Option<String>,
    pub group_name: Option<String>,
    pub cover_gif_url: Option<String>,
    pub cover_gif_alt_text: Option<String>,
    pub remove_cover_gif: bool,
}

#[derive(Debug, Clone)]
pub struct DraftCardInput {
    pub retro_id: Uuid,
    pub column_id: Uuid,
    pub author_subject: String,
    pub author_display_name: String,
    pub body_text: Option<String>,
    pub gif_url: Option<String>,
    pub gif_alt_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IngestItemInput {
    pub retro_id: Uuid,
    pub subject: String,
    pub display_name: String,
    pub source: String,
    pub placement: String,
    pub target_column_id: Option<Uuid>,
    pub suggested_text: Option<String>,
    pub gif_url: Option<String>,
    pub idempotency_key: Option<String>,
    pub raw_payload: Value,
    pub source_metadata: Value,
}

#[derive(Debug, Clone)]
pub struct CreateMeetingNoteInput {
    pub retro_id: Uuid,
    pub author_subject: String,
    pub author_display_name: String,
    pub title: String,
    pub body_text: String,
}

#[derive(Debug, Clone)]
pub struct AcceptDeckItemInput {
    pub retro_id: Uuid,
    pub item_id: Uuid,
    pub column_id: Uuid,
    pub subject: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct CastVoteInput {
    pub retro_id: Uuid,
    pub card_id: Uuid,
    pub subject: String,
    pub display_name: String,
    pub count: i32,
}

#[derive(Debug, Clone)]
pub struct ClusterCardsInput {
    pub retro_id: Uuid,
    pub card_id: Uuid,
    pub target_card_id: Uuid,
    pub subject: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct AutoClusterGroupInput {
    pub title: String,
    pub details: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub card_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct UpdateActionInput {
    pub retro_id: Uuid,
    pub action_id: Uuid,
    pub title: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateActionInput {
    pub retro_id: Uuid,
    pub title: String,
    pub details: Option<String>,
}

#[derive(Debug)]
pub enum ActionError {
    Sqlx(sqlx::Error),
    Invalid(String),
}

impl From<sqlx::Error> for ActionError {
    fn from(error: sqlx::Error) -> Self {
        Self::Sqlx(error)
    }
}

#[derive(Debug)]
pub enum VotingError {
    Sqlx(sqlx::Error),
    Invalid(String),
}

impl From<sqlx::Error> for VotingError {
    fn from(error: sqlx::Error) -> Self {
        Self::Sqlx(error)
    }
}

#[derive(Debug)]
pub enum ClusterError {
    Sqlx(sqlx::Error),
    Invalid(String),
}

impl From<sqlx::Error> for ClusterError {
    fn from(error: sqlx::Error) -> Self {
        Self::Sqlx(error)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ClusteringRetro {
    id: Uuid,
    phase: String,
    clustering_mode: String,
    clustering_status: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ClusterCandidate {
    id: Uuid,
    text: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ClusterCardTarget {
    id: Uuid,
    column_id: Uuid,
    cluster_id: Option<Uuid>,
    parent_card_id: Option<Uuid>,
    text: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ActionCandidate {
    id: Uuid,
    title: String,
    vote_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ParticipantId {
    id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
struct VoteTarget {
    #[allow(dead_code)]
    id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
struct VoteCount {
    count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RetroTemplate {
    Standard,
    Custom { columns: Vec<String> },
}

impl RetroTemplate {
    fn column_titles(&self) -> Vec<String> {
        match self {
            Self::Standard => ["How are you feeling?", "Went well", "To improve"]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            Self::Custom { columns } => columns
                .iter()
                .map(|column| column.trim())
                .filter(|column| !column.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn creates_standard_retro_with_participant_and_columns(pool: PgPool) {
        let repo = RetroRepository::new(pool);

        let created = repo
            .create_retro(CreateRetroInput {
                title: "Sprint 43".to_owned(),
                creator_subject: "user-123".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        assert_eq!(created.retro.title, "Sprint 43");
        assert_eq!(created.retro.phase, "writing");
        assert_eq!(created.retro.happened_at, None);
        assert_eq!(
            created
                .columns
                .iter()
                .map(|column| column.title.as_str())
                .collect::<Vec<_>>(),
            ["How are you feeling?", "Went well", "To improve", "Actions"]
        );

        let overview = repo.list_retros("user-123").await.unwrap();
        assert_eq!(overview.active.len(), 1);
        assert_eq!(overview.completed.len(), 0);
        assert_eq!(overview.active[0].participant_count, 1);
        assert_eq!(overview.active[0].column_count, 4);
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn create_retro_returns_created_series(pool: PgPool) {
        let repo = RetroRepository::new(pool);

        let created = repo
            .create_retro(CreateRetroInput {
                title: "Payments retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: Some("Payments".to_owned()),
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        assert_eq!(created.series.unwrap().name, "Payments");
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn future_planned_retro_starts_scheduled_then_host_can_start(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Future retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: Some("2099-05-15".to_owned()),
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .expect("create future retro");

        assert_eq!(created.retro.phase, "scheduled");
        assert_eq!(created.retro.planned_for, "2099-05-15");

        let started = repo
            .start_scheduled_retro(created.retro.id)
            .await
            .expect("start scheduled retro")
            .expect("scheduled retro transitioned");

        assert_eq!(started.phase, "writing");
        assert_eq!(started.planned_for, "2099-05-15");
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn completed_retro_sets_system_owned_happened_at(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Done retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: Some("2000-01-02".to_owned()),
                template: RetroTemplate::Standard,
                vote_limit: 0,
                action_discussion_limit: 0,
                column_colors: Vec::new(),
            })
            .await
            .expect("create retro");

        repo.reveal_board(created.retro.id).await.expect("reveal");
        repo.start_action_discussion(created.retro.id)
            .await
            .expect("start action discussion");
        let completed = repo
            .complete_retro(created.retro.id)
            .await
            .expect("complete")
            .expect("completed retro");

        assert_eq!(completed.phase, "completed");
        assert_eq!(completed.planned_for, "2000-01-02");
        assert!(completed.happened_at.is_some());
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn creates_custom_retro_with_supplied_columns(pool: PgPool) {
        let repo = RetroRepository::new(pool);

        let created = repo
            .create_retro(CreateRetroInput {
                title: "Team pulse".to_owned(),
                creator_subject: "user-456".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Lee".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Custom {
                    columns: vec![
                        "Kudos".to_owned(),
                        "Friction".to_owned(),
                        "Ideas".to_owned(),
                        "Questions".to_owned(),
                    ],
                },
                vote_limit: 5,
                action_discussion_limit: 2,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        assert_eq!(created.retro.phase, "writing");
        assert_eq!(created.retro.vote_limit, 5);
        assert_eq!(created.retro.action_discussion_limit, 2);
        assert_eq!(
            created
                .columns
                .iter()
                .map(|column| column.title.as_str())
                .collect::<Vec<_>>(),
            ["Kudos", "Friction", "Ideas", "Questions", "Actions"]
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn writing_board_hides_other_participants_drafts_until_reveal(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Privacy retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();
        let column_id = created.columns[0].id;

        repo.create_draft_card(DraftCardInput {
            retro_id: created.retro.id,
            column_id,
            author_subject: "ava".to_owned(),
            author_display_name: "Ava".to_owned(),
            body_text: Some("Ava can read this".to_owned()),
            gif_url: None,
            gif_alt_text: None,
        })
        .await
        .unwrap();
        repo.create_draft_card(DraftCardInput {
            retro_id: created.retro.id,
            column_id,
            author_subject: "lee".to_owned(),
            author_display_name: "Lee".to_owned(),
            body_text: Some("Lee private draft".to_owned()),
            gif_url: None,
            gif_alt_text: None,
        })
        .await
        .unwrap();

        let ava_board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        let ava_cards = &ava_board.columns[0].cards;
        assert_eq!(ava_cards[0].body_text.as_deref(), Some("Ava can read this"));
        assert_eq!(ava_cards[1].body_text, None);
        assert!(ava_cards[1].hidden);

        repo.reveal_board(created.retro.id).await.unwrap();
        let lee_board = repo
            .fetch_board_for_user(created.retro.id, "lee", "Lee")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lee_board.retro.phase, "discussion");
        assert_eq!(
            lee_board.columns[0].cards[0].body_text.as_deref(),
            Some("Ava can read this")
        );
        assert_eq!(
            lee_board.columns[0].cards[1].body_text.as_deref(),
            Some("Lee private draft")
        );

        let late_card = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some("Added during discussion".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();
        assert_eq!(late_card.state, "revealed");
        let lee_board = repo
            .fetch_board_for_user(created.retro.id, "lee", "Lee")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            lee_board.columns[0].cards[2].body_text.as_deref(),
            Some("Added during discussion")
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn gif_cards_can_be_attached_replaced_removed_and_hidden(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "GIF retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();
        let column_id = created.columns[0].id;

        let card = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: None,
                gif_url: Some("https://media.example/high-five.gif".to_owned()),
                gif_alt_text: Some("high five".to_owned()),
            })
            .await
            .unwrap();

        assert_eq!(
            card.gif_url.as_deref(),
            Some("https://media.example/high-five.gif")
        );

        let replaced = repo
            .update_draft_card(
                card.id,
                "ava",
                Some("now with words"),
                Some("https://media.example/thumbs-up.gif"),
                Some("thumbs up"),
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(replaced.body_text.as_deref(), Some("now with words"));
        assert_eq!(
            replaced.gif_url.as_deref(),
            Some("https://media.example/thumbs-up.gif")
        );

        let removed = repo
            .update_draft_card(card.id, "ava", Some("text only now"), None, None, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(removed.body_text.as_deref(), Some("text only now"));
        assert_eq!(removed.gif_url, None);

        repo.create_draft_card(DraftCardInput {
            retro_id: created.retro.id,
            column_id,
            author_subject: "lee".to_owned(),
            author_display_name: "Lee".to_owned(),
            body_text: None,
            gif_url: Some("https://media.example/private.gif".to_owned()),
            gif_alt_text: Some("private".to_owned()),
        })
        .await
        .unwrap();

        let ava_board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ava_board.columns[0].cards[1].gif_url, None);
        assert!(ava_board.columns[0].cards[1].hidden);
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn draft_cards_can_move_between_columns(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Move retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let card = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some("move me".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();

        let moved = repo
            .move_draft_card(
                created.retro.id,
                card.id,
                created.columns[1].id,
                None,
                "ava",
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(moved.column_id, created.columns[1].id);

        let lee_attempt = repo
            .move_draft_card(
                created.retro.id,
                card.id,
                created.columns[2].id,
                None,
                "lee",
            )
            .await
            .unwrap();
        assert!(lee_attempt.is_none());
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn writing_ready_marks_are_recorded_per_participant(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Ready retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        repo.mark_ready(created.retro.id, "ava", "Ava")
            .await
            .unwrap();
        repo.mark_ready(created.retro.id, "lee", "Lee")
            .await
            .unwrap();
        repo.mark_ready(created.retro.id, "ava", "Ava")
            .await
            .unwrap();

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.ready.ready_count, 2);
        assert_eq!(board.ready.participant_count, 2);
        assert!(board.ready.current_user_ready);
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn voting_tracks_counts_limits_remaining_and_ready_marks(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Voting retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();
        let card = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some("Vote on this".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();

        repo.reveal_board(created.retro.id).await.unwrap();
        let voting = repo.start_voting(created.retro.id).await.unwrap();
        assert_eq!(voting.phase, "voting");

        let info = repo
            .cast_vote(CastVoteInput {
                retro_id: created.retro.id,
                card_id: card.id,
                subject: "lee".to_owned(),
                display_name: "Lee".to_owned(),
                count: 2,
            })
            .await
            .unwrap();
        assert_eq!(info.votes_used, 2);
        assert_eq!(info.votes_remaining, 1);

        let too_many = repo
            .cast_vote(CastVoteInput {
                retro_id: created.retro.id,
                card_id: card.id,
                subject: "lee".to_owned(),
                display_name: "Lee".to_owned(),
                count: 2,
            })
            .await;
        assert!(matches!(too_many, Err(VotingError::Invalid(_))));

        repo.mark_ready(created.retro.id, "lee", "Lee")
            .await
            .unwrap();
        let board = repo
            .fetch_board_for_user(created.retro.id, "lee", "Lee")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.ready.ready_count, 1);
        assert!(board.ready.current_user_ready);
        assert_eq!(board.voting.votes_remaining, 1);
        assert_eq!(board.columns[0].cards[0].vote_count, 2);
        assert_eq!(board.columns[0].cards[0].current_user_vote_count, 2);
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn clustering_runs_once_and_preserves_original_cards(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Cluster retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        for text in [
            "Deploy alerts are noisy",
            "Deploy alerts need ownership",
            "Lunch was good",
        ] {
            repo.create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some(text.to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();
        }

        repo.reveal_board(created.retro.id).await.unwrap();
        sqlx::query("UPDATE retros SET clustering_mode = 'auto_on_vote_start' WHERE id = $1")
            .bind(created.retro.id)
            .execute(&repo.pool)
            .await
            .unwrap();
        let clusters = repo.cluster_board(created.retro.id).await.unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].category.as_deref(), Some("deploy"));
        assert!(clusters[0].tags.contains(&"auto-clustered".to_owned()));

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.clusters.len(), 1);
        assert_eq!(board.columns[0].cards.len(), 3);
        assert_eq!(
            board.columns[0].cards[0].cluster_category.as_deref(),
            Some("deploy")
        );
        assert_eq!(
            board.columns[0].cards[1].cluster_category.as_deref(),
            Some("deploy")
        );
        assert_eq!(board.columns[0].cards[2].cluster_id, None);

        let second = repo.cluster_board(created.retro.id).await;
        assert!(matches!(second, Err(ClusterError::Invalid(_))));
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn manual_clustering_groups_cards_during_voting(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Manual cluster retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let first = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some("Manual cluster one".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();
        let second = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "lee".to_owned(),
                author_display_name: "Lee".to_owned(),
                body_text: Some("Manual cluster two".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();

        repo.reveal_board(created.retro.id).await.unwrap();
        repo.start_voting(created.retro.id).await.unwrap();
        let cluster = repo
            .cluster_cards(ClusterCardsInput {
                retro_id: created.retro.id,
                card_id: first.id,
                target_card_id: second.id,
                subject: "ava".to_owned(),
                display_name: "Ava".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(cluster.category.as_deref(), Some("manual"));

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.columns[0].cards.len(), 1);
        let group_card = &board.columns[0].cards[0];
        assert_eq!(group_card.cluster_id, Some(cluster.id));
        assert_eq!(group_card.cluster_title.as_deref(), Some("Grouped: manual"));
        assert_eq!(
            group_card
                .cluster_members
                .iter()
                .map(|card| card.id)
                .collect::<Vec<_>>(),
            vec![first.id, second.id]
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn auto_clustering_reparents_existing_group_members(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Auto over manual retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let first = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some("Manual cluster one".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();
        let second = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "lee".to_owned(),
                author_display_name: "Lee".to_owned(),
                body_text: Some("Manual cluster two".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();
        let third = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "mia".to_owned(),
                author_display_name: "Mia".to_owned(),
                body_text: Some("AI cluster three".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();

        repo.reveal_board(created.retro.id).await.unwrap();
        repo.start_voting(created.retro.id).await.unwrap();
        repo.cluster_cards(ClusterCardsInput {
            retro_id: created.retro.id,
            card_id: first.id,
            target_card_id: second.id,
            subject: "ava".to_owned(),
            display_name: "Ava".to_owned(),
        })
        .await
        .unwrap();
        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        let existing_group = board.columns[0].cards[0].id;
        repo.cast_vote(CastVoteInput {
            retro_id: created.retro.id,
            card_id: existing_group,
            subject: "ava".to_owned(),
            display_name: "Ava".to_owned(),
            count: 1,
        })
        .await
        .unwrap();
        sqlx::query(
            "UPDATE retros
             SET clustering_mode = 'auto_on_vote_start',
                 clustering_status = 'running'
             WHERE id = $1",
        )
        .bind(created.retro.id)
        .execute(&repo.pool)
        .await
        .unwrap();

        repo.apply_auto_cluster_groups(
            created.retro.id,
            vec![AutoClusterGroupInput {
                title: "Combined theme".to_owned(),
                details: None,
                category: Some("delivery".to_owned()),
                tags: vec!["delivery".to_owned()],
                card_ids: vec![existing_group, third.id],
            }],
        )
        .await
        .unwrap();

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.columns[0].cards.len(), 1);
        let group_card = &board.columns[0].cards[0];
        assert_eq!(group_card.vote_count, 1);
        assert_eq!(
            group_card
                .cluster_members
                .iter()
                .map(|card| card.id)
                .collect::<Vec<_>>(),
            vec![first.id, second.id, third.id]
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn manual_clustering_across_columns_moves_source_to_target_column(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Cross column cluster retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let source = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[0].id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some("Source card".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();
        let target = repo
            .create_draft_card(DraftCardInput {
                retro_id: created.retro.id,
                column_id: created.columns[1].id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                body_text: Some("Target card".to_owned()),
                gif_url: None,
                gif_alt_text: None,
            })
            .await
            .unwrap();

        repo.reveal_board(created.retro.id).await.unwrap();
        let cluster = repo
            .cluster_cards(ClusterCardsInput {
                retro_id: created.retro.id,
                card_id: source.id,
                target_card_id: target.id,
                subject: "ava".to_owned(),
                display_name: "Ava".to_owned(),
            })
            .await
            .unwrap();

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert!(board.columns[0].cards.is_empty());
        assert_eq!(board.columns[1].cards.len(), 1);
        let group_card = &board.columns[1].cards[0];
        assert_eq!(group_card.column_id, created.columns[1].id);
        assert_eq!(group_card.cluster_id, Some(cluster.id));
        assert_eq!(
            group_card
                .cluster_members
                .iter()
                .map(|card| card.id)
                .collect::<Vec<_>>(),
            vec![source.id, target.id]
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn action_discussion_moves_top_voted_cards_to_actions_column(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Actions retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 2,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let mut cards = Vec::new();
        for text in ["Most important", "Second important", "No votes"] {
            cards.push(
                repo.create_draft_card(DraftCardInput {
                    retro_id: created.retro.id,
                    column_id: created.columns[0].id,
                    author_subject: "ava".to_owned(),
                    author_display_name: "Ava".to_owned(),
                    body_text: Some(text.to_owned()),
                    gif_url: None,
                    gif_alt_text: None,
                })
                .await
                .unwrap(),
            );
        }

        repo.reveal_board(created.retro.id).await.unwrap();
        repo.start_voting(created.retro.id).await.unwrap();
        repo.cast_vote(CastVoteInput {
            retro_id: created.retro.id,
            card_id: cards[0].id,
            subject: "lee".to_owned(),
            display_name: "Lee".to_owned(),
            count: 2,
        })
        .await
        .unwrap();
        repo.cast_vote(CastVoteInput {
            retro_id: created.retro.id,
            card_id: cards[1].id,
            subject: "ava".to_owned(),
            display_name: "Ava".to_owned(),
            count: 1,
        })
        .await
        .unwrap();

        let actions = repo
            .start_action_discussion(created.retro.id)
            .await
            .unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].source_card_id, Some(cards[0].id));
        assert_eq!(actions[1].source_card_id, Some(cards[1].id));
        assert!(actions[0].tags.contains(&"topvoted".to_owned()));

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        let action_column = board
            .columns
            .iter()
            .find(|column| column.title == "Actions")
            .unwrap();
        assert_eq!(
            action_column
                .cards
                .iter()
                .map(|card| card.id)
                .collect::<Vec<_>>(),
            vec![cards[0].id, cards[1].id]
        );
        assert!(
            board
                .columns
                .iter()
                .find(|column| column.id == created.columns[0].id)
                .unwrap()
                .cards
                .iter()
                .any(|card| card.id == cards[2].id)
        );

        let edited = repo
            .update_action(UpdateActionInput {
                retro_id: created.retro.id,
                action_id: actions[0].id,
                title: "Assign alert owner".to_owned(),
                details: Some("Ava by Friday".to_owned()),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(edited.title, "Assign alert owner");

        let confirmed = repo
            .set_action_status(created.retro.id, actions[0].id, "confirmed")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(confirmed.status, "confirmed");

        let rejected = repo
            .set_action_status(created.retro.id, actions[1].id, "rejected")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(rejected.status, "rejected");

        let completed = repo
            .complete_retro(created.retro.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(completed.phase, "completed");

        let overview = repo.list_retros("ava").await.unwrap();
        assert_eq!(overview.active.len(), 0);
        assert_eq!(overview.completed.len(), 1);
        assert_eq!(overview.completed[0].unresolved_action_count, 1);
        assert!(
            overview.completed[0]
                .recurring_tags
                .contains(&"topvoted".to_owned())
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn ingestion_supports_idempotent_deck_and_direct_draft_modes(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Ingest retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let deck_item = repo
            .ingest_item(IngestItemInput {
                retro_id: created.retro.id,
                subject: "ava".to_owned(),
                display_name: "Ava".to_owned(),
                source: "pi".to_owned(),
                placement: "user_deck".to_owned(),
                target_column_id: None,
                suggested_text: Some("Pi suggested mood".to_owned()),
                gif_url: None,
                idempotency_key: Some("same-event".to_owned()),
                raw_payload: serde_json::json!({"body":"Pi suggested mood"}),
                source_metadata: serde_json::json!({"tool":"pi"}),
            })
            .await
            .unwrap();
        let duplicate = repo
            .ingest_item(IngestItemInput {
                retro_id: created.retro.id,
                subject: "ava".to_owned(),
                display_name: "Ava".to_owned(),
                source: "pi".to_owned(),
                placement: "user_deck".to_owned(),
                target_column_id: None,
                suggested_text: Some("ignored duplicate body".to_owned()),
                gif_url: None,
                idempotency_key: Some("same-event".to_owned()),
                raw_payload: serde_json::json!({}),
                source_metadata: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert_eq!(deck_item.id, duplicate.id);

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.deck.len(), 1);

        let accepted = repo
            .accept_deck_item(AcceptDeckItemInput {
                retro_id: created.retro.id,
                item_id: deck_item.id,
                column_id: created.columns[0].id,
                subject: "ava".to_owned(),
                display_name: "Ava".to_owned(),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(accepted.body_text.as_deref(), Some("Pi suggested mood"));

        let direct = repo
            .ingest_item(IngestItemInput {
                retro_id: created.retro.id,
                subject: "ava".to_owned(),
                display_name: "Ava".to_owned(),
                source: "claude_code".to_owned(),
                placement: "retro_draft".to_owned(),
                target_column_id: Some(created.columns[1].id),
                suggested_text: Some("Direct private draft".to_owned()),
                gif_url: None,
                idempotency_key: Some("direct-event".to_owned()),
                raw_payload: serde_json::json!({}),
                source_metadata: serde_json::json!({"mode":"direct"}),
            })
            .await
            .unwrap();
        assert_eq!(direct.status, "accepted");
        assert!(direct.accepted_card_id.is_some());
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn ai_artifacts_track_success_failure_and_retry(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "AI retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let artifact = repo
            .create_ai_artifact(
                created.retro.id,
                "summary",
                serde_json::json!({"provider":"fake"}),
            )
            .await
            .unwrap();
        assert_eq!(artifact.status, "pending");

        let running = repo.mark_ai_running(artifact.id).await.unwrap().unwrap();
        assert_eq!(running.status, "running");

        let failed = repo
            .fail_ai_artifact(artifact.id, "fake provider failure")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.status, "failed");
        assert_eq!(
            failed.error_message.as_deref(),
            Some("fake provider failure")
        );

        let retrying = repo
            .retry_ai_artifact(created.retro.id, artifact.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrying.status, "running");
        assert_eq!(retrying.retry_count, 1);

        let completed = repo
            .complete_ai_artifact(
                artifact.id,
                serde_json::json!({"summary":"Reviewable output"}),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, "succeeded");

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.ai_artifacts.len(), 1);
        assert_eq!(
            board.ai_artifacts[0].output.as_ref().unwrap()["summary"],
            "Reviewable output"
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn meeting_notes_attach_to_retro_and_feed_summary_mood_context(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Notes retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let empty_context = repo
            .ai_input_with_note_context(created.retro.id, "summary")
            .await
            .unwrap();
        assert_eq!(empty_context["meeting_notes_included"], false);

        let note = repo
            .create_meeting_note(CreateMeetingNoteInput {
                retro_id: created.retro.id,
                author_subject: "ava".to_owned(),
                author_display_name: "Ava".to_owned(),
                title: "Planning notes".to_owned(),
                body_text: "We need clearer release ownership.".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(note.title, "Planning notes");

        let context = repo
            .ai_input_with_note_context(created.retro.id, "mood")
            .await
            .unwrap();
        assert_eq!(context["meeting_notes_included"], true);
        assert_eq!(
            context["meeting_notes"][0]["body_text"],
            "We need clearer release ownership."
        );

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.meeting_notes.len(), 1);
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn deliveries_export_summary_and_retry_failures(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Delivery retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 1,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let payload = repo.export_summary_payload(created.retro.id).await.unwrap();
        assert_eq!(payload["title"], "Delivery retro");
        assert_eq!(payload["copy_markdown"], "# Delivery retro\n\nActions: 0");

        let failed = repo
            .create_delivery(created.retro.id, "summary_export", payload, true)
            .await
            .unwrap();
        assert_eq!(failed.status, "failed");
        assert_eq!(
            failed.error_message.as_deref(),
            Some("fake delivery failure")
        );

        let retried = repo
            .retry_delivery(created.retro.id, failed.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retried.status, "succeeded");
        assert_eq!(retried.retry_count, 1);

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.deliveries.len(), 1);
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn completing_a_series_retro_plans_one_next_retro(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Platform retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: Some("Platform".to_owned()),
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: Some("2026-05-15".to_owned()),
                template: RetroTemplate::Standard,
                vote_limit: 4,
                action_discussion_limit: 2,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();
        repo.add_board_grant(created.retro.id, "lee@example.com", "member")
            .await
            .unwrap();
        repo.start_scheduled_retro(created.retro.id).await.unwrap();
        repo.reveal_board(created.retro.id).await.unwrap();
        repo.start_action_discussion(created.retro.id)
            .await
            .unwrap();
        repo.complete_retro(created.retro.id).await.unwrap();

        let next = repo
            .ensure_next_retro(created.retro.id, "ava", "ava@example.com", "Ava")
            .await
            .unwrap()
            .unwrap();
        let repeated = repo
            .ensure_next_retro(created.retro.id, "ava", "ava@example.com", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(next.retro.id, repeated.retro.id);
        assert_eq!(next.retro.phase, "scheduled");
        assert_eq!(next.retro.title, "Next: Platform retro");
        let expected_planned_for = sqlx::query_scalar::<_, String>(
            "SELECT to_char(
                GREATEST('2026-05-15'::date + INTERVAL '14 days', CURRENT_DATE + INTERVAL '7 days')::date,
                'YYYY-MM-DD'
            )",
        )
        .fetch_one(&repo.pool)
        .await
        .unwrap();
        assert_eq!(next.retro.planned_for, expected_planned_for);
        assert_eq!(next.retro.vote_limit, 4);
        assert_eq!(next.retro.action_discussion_limit, 2);
        assert_eq!(next.retro.creator_email, "ava@example.com");
        assert_eq!(next.participants[0].role, "host");
        assert_eq!(next.series.unwrap().name, "Platform");
        assert_eq!(next.columns.len(), created.columns.len());

        let wrapped = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(wrapped.series.unwrap().name, "Platform");
        assert_eq!(wrapped.next_retro.unwrap().id, next.retro.id);

        let lee_overview = repo
            .list_retros_for_user("lee", "lee@example.com")
            .await
            .unwrap();
        let lee_next = lee_overview
            .active
            .iter()
            .find(|summary| summary.id == next.retro.id)
            .unwrap();
        assert_eq!(lee_next.current_user_role, "member");
        let ava_overview = repo
            .list_retros_for_user("ava", "ava@example.com")
            .await
            .unwrap();
        let ava_next = ava_overview
            .active
            .iter()
            .find(|summary| summary.id == next.retro.id)
            .unwrap();
        assert_eq!(ava_next.current_user_role, "host");
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn next_retro_uses_previous_series_cadence(pool: PgPool) {
        let repo = RetroRepository::new(pool.clone());
        let first = repo
            .create_retro(CreateRetroInput {
                title: "Platform retro 1".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: Some("Platform".to_owned()),
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: Some("2099-05-01".to_owned()),
                template: RetroTemplate::Standard,
                vote_limit: 4,
                action_discussion_limit: 2,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();
        let second = repo
            .create_retro(CreateRetroInput {
                title: "Platform retro 2".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: Some("2099-05-08".to_owned()),
                template: RetroTemplate::Standard,
                vote_limit: 4,
                action_discussion_limit: 2,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();
        sqlx::query(
            "UPDATE retros
             SET group_id = (SELECT group_id FROM retros WHERE id = $1)
             WHERE id = $2",
        )
        .bind(first.retro.id)
        .bind(second.retro.id)
        .execute(&pool)
        .await
        .unwrap();

        let next = repo
            .ensure_next_retro(second.retro.id, "ava", "ava@example.com", "Ava")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(next.retro.planned_for, "2099-05-15");
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn new_series_does_not_inherit_unrelated_same_settings_cadence(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        repo.create_retro(CreateRetroInput {
            title: "Unrelated retro".to_owned(),
            creator_subject: "ava".to_owned(),
            creator_email: "ava@example.com".to_owned(),
            creator_display_name: "Ava".to_owned(),
            group_name: Some("Unrelated".to_owned()),
            cover_gif_url: None,
            cover_gif_alt_text: None,
            planned_for: Some("2099-05-01".to_owned()),
            template: RetroTemplate::Standard,
            vote_limit: 4,
            action_discussion_limit: 2,
            column_colors: Vec::new(),
        })
        .await
        .unwrap();
        let source = repo
            .create_retro(CreateRetroInput {
                title: "New series retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: Some("2099-05-08".to_owned()),
                template: RetroTemplate::Standard,
                vote_limit: 4,
                action_discussion_limit: 2,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        let next = repo
            .ensure_next_retro(source.retro.id, "ava", "ava@example.com", "Ava")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(next.retro.planned_for, "2099-05-22");
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn retro_details_update_title_and_group(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Old retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_email: "ava@example.com".to_owned(),
                creator_display_name: "Ava".to_owned(),
                group_name: None,
                cover_gif_url: None,
                cover_gif_alt_text: None,
                planned_for: None,
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        repo.update_retro_details(UpdateRetroDetailsInput {
            retro_id: created.retro.id,
            title: Some("New retro".to_owned()),
            group_name: Some("Payments".to_owned()),
            cover_gif_url: None,
            cover_gif_alt_text: None,
            remove_cover_gif: false,
        })
        .await
        .unwrap()
        .unwrap();

        let board = repo
            .fetch_board_for_user(created.retro.id, "ava", "Ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(board.retro.title, "New retro");
        assert_eq!(board.series.unwrap().name, "Payments");
    }
}
