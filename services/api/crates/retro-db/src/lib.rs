use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

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
        sqlx::query_as::<_, RetroRecord>("SELECT id, title, phase, vote_limit, action_discussion_limit FROM retros WHERE id = $1")
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_retro(&self, input: CreateRetroInput) -> Result<RetroBoard, sqlx::Error> {
        let mut columns = input.template.column_titles();
        if input.action_discussion_limit > 0
            && !columns.iter().any(|column| column.trim().eq_ignore_ascii_case("actions"))
        {
            columns.push("Actions".to_owned());
        }
        let mut tx = self.pool.begin().await?;

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

        sqlx::query(
            "INSERT INTO participants (retro_id, external_subject, display_name, role)
             VALUES ($1, $2, $3, 'host')
             ON CONFLICT (retro_id, external_subject) DO UPDATE
             SET display_name = EXCLUDED.display_name
             RETURNING id",
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
            records.push(record.into());
        }

        tx.commit().await?;

        Ok(RetroBoard {
            retro,
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

    pub async fn fetch_board(&self, id: Uuid) -> Result<Option<RetroBoard>, sqlx::Error> {
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let columns = self.fetch_columns(id).await?;
        let clusters = self.fetch_clusters(id).await?;
        let actions = self.fetch_actions(id).await?;
        let deck = Vec::new();
        let ai_artifacts = self.fetch_ai_artifacts(id).await?;
        let meeting_notes = self.fetch_meeting_notes(id).await?;
        let deliveries = self.fetch_deliveries(id).await?;
        let ready = self.ready_info(id, "").await?;
        let voting = self.voting_info(id, "").await?;
        Ok(Some(RetroBoard {
            retro,
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
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let participant_id = self.ensure_participant(id, subject, display_name).await?;
        self.record_retro_access(id, participant_id).await?;
        let mut columns = self.fetch_columns(id).await?;
        let clusters = self.fetch_clusters(id).await?;
        let cards = self.fetch_cards_for_user(id, subject).await?;
        let actions = self.fetch_actions(id).await?;
        let deck = self.fetch_deck(id, subject).await?;
        let ai_artifacts = self.fetch_ai_artifacts(id).await?;
        let meeting_notes = self.fetch_meeting_notes(id).await?;
        let deliveries = self.fetch_deliveries(id).await?;
        let mut member_cards = std::collections::BTreeMap::<Uuid, Vec<ClusterMemberRecord>>::new();
        let mut top_level_cards = Vec::new();
        for card in cards {
            if let Some(parent_card_id) = card.parent_card_id {
                member_cards
                    .entry(parent_card_id)
                    .or_default()
                    .push(ClusterMemberRecord::from(&card));
            } else {
                top_level_cards.push(card);
            }
        }
        for card in &mut top_level_cards {
            card.cluster_members = member_cards.remove(&card.id).unwrap_or_default();
        }

        for column in &mut columns {
            column.cards = top_level_cards
                .iter()
                .filter(|card| card.column_id == column.id)
                .cloned()
                .collect();
        }
        let ready = self.ready_info(id, subject).await?;
        let voting = self.voting_info(id, subject).await?;
        Ok(Some(RetroBoard {
            retro,
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
        let rows = sqlx::query_as::<_, RetroSummaryRow>(
            "SELECT
                r.id,
                r.title,
                r.phase,
                r.vote_limit,
                r.action_discussion_limit,
                to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
                to_char(
                    GREATEST(
                        r.created_at,
                        COALESCE(r.completed_at, r.created_at),
                        COALESCE((SELECT MAX(card.updated_at) FROM cards card WHERE card.retro_id = r.id), r.created_at),
                        COALESCE((SELECT MAX(v.created_at) FROM votes v WHERE v.retro_id = r.id), r.created_at),
                        COALESCE((SELECT MAX(ai.created_at) FROM action_items ai WHERE ai.retro_id = r.id), r.created_at),
                        COALESCE((SELECT MAX(rm.ready_at) FROM participant_ready_marks rm WHERE rm.retro_id = r.id), r.created_at),
                        COALESCE((SELECT MAX(artifact.updated_at) FROM ai_artifacts artifact WHERE artifact.retro_id = r.id), r.created_at),
                        COALESCE((SELECT MAX(note.created_at) FROM meeting_notes note WHERE note.retro_id = r.id), r.created_at),
                        COALESCE((SELECT MAX(delivery.updated_at) FROM deliveries delivery WHERE delivery.retro_id = r.id), r.created_at)
                    ) AT TIME ZONE 'UTC',
                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'
                ) AS last_activity_at,
                (
                    SELECT to_char(MAX(ra.opened_at) AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                    FROM retro_accesses ra
                    JOIN participants ap ON ap.id = ra.participant_id
                    WHERE ra.retro_id = r.id AND ap.external_subject = $1
                ) AS last_opened_at,
                COUNT(DISTINCT p.id)::BIGINT AS participant_count,
                COUNT(DISTINCT c.id)::BIGINT AS column_count,
                COUNT(DISTINCT a.id) FILTER (WHERE a.status NOT IN ('rejected', 'done'))::BIGINT AS unresolved_action_count,
                COALESCE(jsonb_agg(DISTINCT tag.value) FILTER (WHERE tag.value IS NOT NULL), '[]'::jsonb) AS recurring_tags,
                COALESCE(
                    (
                        SELECT jsonb_agg(
                            jsonb_build_object(
                                'id', ai.id,
                                'title', ai.title,
                                'status', ai.status
                            )
                            ORDER BY ai.position, ai.created_at, ai.id
                        )
                        FROM action_items ai
                        WHERE ai.retro_id = r.id AND ai.status NOT IN ('rejected', 'done')
                    ),
                    '[]'::jsonb
                ) AS open_actions
             FROM retros r
             LEFT JOIN participants p ON p.retro_id = r.id
             LEFT JOIN retro_columns c ON c.retro_id = r.id
             LEFT JOIN action_items a ON a.retro_id = r.id
             LEFT JOIN LATERAL jsonb_array_elements_text(a.tags) AS tag(value) ON true
	             WHERE EXISTS (
	                 SELECT 1
	                 FROM participants scoped_participant
	                 LEFT JOIN retro_accesses scoped_access ON scoped_access.participant_id = scoped_participant.id
	                 WHERE scoped_participant.retro_id = r.id
	                   AND scoped_participant.external_subject = $1
	                   AND (scoped_participant.role = 'host' OR scoped_access.retro_id = r.id)
	             )
             GROUP BY r.id
             ORDER BY last_activity_at DESC, r.created_at DESC",
        )
        .bind(subject)
        .fetch_all(&self.pool)
        .await?;

        let summaries = rows.into_iter().map(RetroSummary::from).collect::<Vec<_>>();
        let (completed, active): (Vec<_>, Vec<_>) = summaries
            .into_iter()
            .partition(|summary| summary.phase == "completed");

        Ok(RetroOverview { active, completed })
    }

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

    pub async fn ingest_item(
        &self,
        input: IngestItemInput,
    ) -> Result<IngestedItemRecord, sqlx::Error> {
        let participant_id = self
            .ensure_participant(input.retro_id, &input.subject, &input.display_name)
            .await?;

        if let Some(idempotency_key) = input.idempotency_key.as_deref() {
            if let Some(existing) = self
                .fetch_ingested_by_idempotency(participant_id, &input.source, idempotency_key)
                .await?
            {
                return Ok(existing);
            }
        }

        let accepted_card_id = if input.placement == "retro_draft" {
            let column_id = input.target_column_id.ok_or(sqlx::Error::RowNotFound)?;
            let card = self
                .create_draft_card(DraftCardInput {
                    retro_id: input.retro_id,
                    column_id,
                    author_subject: input.subject.clone(),
                    author_display_name: input.display_name.clone(),
                    body_text: input.suggested_text.clone(),
                    gif_url: input.gif_url.clone(),
                    gif_alt_text: None,
                })
                .await?;
            Some(card.id)
        } else {
            None
        };
        let status = if accepted_card_id.is_some() {
            "accepted"
        } else {
            "pending"
        };

        let row = sqlx::query_as::<_, IngestedItemRow>(
            "INSERT INTO ingested_items (
                recipient_participant_id, retro_id, source, placement, target_column_id,
                suggested_text, gif_url, raw_payload, status, accepted_card_id, idempotency_key, source_metadata
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING id, recipient_participant_id, retro_id, source, placement, target_column_id,
                suggested_text, gif_url, raw_payload, status, accepted_card_id, idempotency_key, source_metadata",
        )
        .bind(participant_id)
        .bind(input.retro_id)
        .bind(&input.source)
        .bind(&input.placement)
        .bind(input.target_column_id)
        .bind(input.suggested_text.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(input.gif_url.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(Json(input.raw_payload))
        .bind(status)
        .bind(accepted_card_id)
        .bind(input.idempotency_key.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(Json(input.source_metadata))
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn accept_deck_item(
        &self,
        input: AcceptDeckItemInput,
    ) -> Result<Option<CardRecord>, sqlx::Error> {
        let participant_id = self
            .ensure_participant(input.retro_id, &input.subject, &input.display_name)
            .await?;
        let row = sqlx::query_as::<_, IngestedItemRow>(
            "SELECT id, recipient_participant_id, retro_id, source, placement, target_column_id,
                suggested_text, gif_url, raw_payload, status, accepted_card_id, idempotency_key, source_metadata
             FROM ingested_items
             WHERE id = $1 AND recipient_participant_id = $2 AND retro_id = $3
               AND placement = 'user_deck' AND status = 'pending'",
        )
        .bind(input.item_id)
        .bind(participant_id)
        .bind(input.retro_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(item) = row.map(IngestedItemRecord::from) else {
            return Ok(None);
        };

        let card = self
            .create_draft_card(DraftCardInput {
                retro_id: input.retro_id,
                column_id: input.column_id,
                author_subject: input.subject,
                author_display_name: input.display_name,
                body_text: item.suggested_text,
                gif_url: item.gif_url,
                gif_alt_text: None,
            })
            .await?;

        sqlx::query(
            "UPDATE ingested_items SET status = 'accepted', accepted_card_id = $2 WHERE id = $1",
        )
        .bind(input.item_id)
        .bind(card.id)
        .execute(&self.pool)
        .await?;

        Ok(Some(card))
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

        sqlx::query("UPDATE cards SET column_id = $1, updated_at = NOW() WHERE parent_card_id = $2")
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
             VALUES (
                $1,
                $2,
                (SELECT CASE WHEN phase = 'voting' THEN 'voting' ELSE 'writing' END FROM retros WHERE id = $2)
             )
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
             RETURNING id, title, phase, vote_limit, action_discussion_limit",
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

    pub async fn start_voting(&self, retro_id: Uuid) -> Result<RetroRecord, VotingError> {
        let retro = sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'voting'
             WHERE id = $1 AND phase = 'discussion' AND vote_limit > 0
             RETURNING id, title, phase, vote_limit, action_discussion_limit",
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
        let participant_id = self.ensure_participant(retro_id, subject, display_name).await?;
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

        self.voting_info(retro_id, subject).await.map_err(Into::into)
    }

    pub async fn cluster_board(&self, retro_id: Uuid) -> Result<Vec<ClusterRecord>, ClusterError> {
        let retro = sqlx::query_as::<_, ClusteringRetro>(
            "SELECT id, phase, clustering_mode, clustering_status FROM retros WHERE id = $1",
        )
        .bind(retro_id)
        .fetch_one(&self.pool)
        .await?;

        if retro.clustering_mode == "disabled" {
            return Err(ClusterError::Invalid("clustering is disabled".to_owned()));
        }
        if retro.clustering_status != "not_run" {
            return Err(ClusterError::Invalid("clustering already ran".to_owned()));
        }
        if !matches!(retro.phase.as_str(), "discussion" | "voting") {
            return Err(ClusterError::Invalid(
                "clustering requires revealed cards".to_owned(),
            ));
        }

        let candidates = sqlx::query_as::<_, ClusterCandidate>(
            "SELECT id, COALESCE(body_text, gif_alt_text, '') AS text
             FROM cards
             WHERE retro_id = $1 AND state = 'revealed'
             ORDER BY position, created_at",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await?;

        let mut groups: BTreeMap<String, Vec<ClusterCandidate>> = BTreeMap::new();
        for candidate in candidates {
            if let Some(key) = cluster_key(&candidate.text) {
                groups.entry(key).or_default().push(candidate);
            }
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE retros SET clustering_status = 'running' WHERE id = $1")
            .bind(retro_id)
            .execute(&mut *tx)
            .await?;

        let mut clusters = Vec::new();
        for (key, cards) in groups.into_iter().filter(|(_, cards)| cards.len() > 1) {
            let title = format!("Similar: {key}");
            let tags = vec![key.clone(), "auto-clustered".to_owned()];
            let row = sqlx::query_as::<_, ClusterRow>(
                "INSERT INTO card_clusters (retro_id, title, category, tags)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, retro_id, title, category, tags",
            )
            .bind(retro.id)
            .bind(&title)
            .bind(&key)
            .bind(Json(tags))
            .fetch_one(&mut *tx)
            .await?;

            for card in &cards {
                sqlx::query("UPDATE cards SET cluster_id = $1, updated_at = NOW() WHERE id = $2")
                    .bind(row.id)
                    .bind(card.id)
                    .execute(&mut *tx)
                    .await?;
            }

            clusters.push(row.into());
        }

        sqlx::query("UPDATE retros SET clustering_status = 'completed' WHERE id = $1")
            .bind(retro_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(clusters)
    }

    pub async fn cluster_cards(&self, input: ClusterCardsInput) -> Result<ClusterRecord, ClusterError> {
        if input.card_id == input.target_card_id {
            return Err(ClusterError::Invalid("cannot cluster a card with itself".to_owned()));
        }

        let mut tx = self.pool.begin().await?;
        let retro = sqlx::query_as::<_, ClusteringRetro>(
            "SELECT id, phase, clustering_mode, clustering_status FROM retros WHERE id = $1",
        )
        .bind(input.retro_id)
        .fetch_one(&mut *tx)
        .await?;

        if retro.phase == "completed" {
            return Err(ClusterError::Invalid(
                "manual clustering is unavailable after completion".to_owned(),
            ));
        }

        let participant_id = self
            .ensure_participant(input.retro_id, &input.subject, &input.display_name)
            .await?;

        let cards = sqlx::query_as::<_, ClusterCardTarget>(
            "SELECT c.id, c.column_id, c.cluster_id, c.parent_card_id, COALESCE(c.body_text, c.gif_alt_text, '') AS text
             FROM cards c
             JOIN participants p ON p.id = c.author_participant_id
             WHERE c.retro_id = $1
               AND c.id IN ($2, $3)
               AND (
                   c.state = 'revealed'
                   OR ($5 = 'writing' AND c.state = 'draft' AND p.external_subject = $4)
               )",
        )
        .bind(input.retro_id)
        .bind(input.card_id)
        .bind(input.target_card_id)
        .bind(&input.subject)
        .bind(&retro.phase)
        .fetch_all(&mut *tx)
        .await?;
        if cards.len() != 2 {
            return Err(ClusterError::Invalid("cluster target is not available".to_owned()));
        }

        let source = cards
            .iter()
            .find(|card| card.id == input.card_id)
            .ok_or_else(|| ClusterError::Invalid("cluster source is not available".to_owned()))?;
        let target = cards
            .iter()
            .find(|card| card.id == input.target_card_id)
            .ok_or_else(|| ClusterError::Invalid("cluster target is not available".to_owned()))?;

        let source_is_group_card = source.cluster_id.is_some() && source.parent_card_id.is_none();
        let (cluster_id, cluster_parent_id) = if target.parent_card_id.is_some() || target.cluster_id.is_some() {
            let parent_id = target.parent_card_id.unwrap_or(target.id);
            let cluster_id = target
                .cluster_id
                .ok_or_else(|| ClusterError::Invalid("cluster target is unavailable".to_owned()))?;
            (cluster_id, parent_id)
        } else {
            let title = manual_cluster_title(&source.text, &target.text);
            let row = sqlx::query_as::<_, ClusterRow>(
                "INSERT INTO card_clusters (retro_id, title, category, tags)
                 VALUES ($1, $2, 'manual', $3)
                 RETURNING id, retro_id, title, category, tags",
            )
            .bind(input.retro_id)
            .bind(&title)
            .bind(Json(vec!["manual".to_owned()]))
            .fetch_one(&mut *tx)
            .await?;

            let parent_card_id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO cards (retro_id, column_id, author_participant_id, cluster_id, body_text, gif_url, gif_alt_text, state, position)
                 VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    NULL,
                    NULL,
                    'revealed',
                    (SELECT COALESCE(MAX(position) + 1, 0) FROM cards WHERE retro_id = $1 AND column_id = $2)
                 )
                 RETURNING id",
            )
            .bind(input.retro_id)
            .bind(target.column_id)
            .bind(participant_id)
            .bind(row.id)
            .bind(title)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE cards
                 SET parent_card_id = $1,
                     column_id = $2,
                     cluster_id = $3,
                     updated_at = NOW()
                 WHERE retro_id = $4 AND id = $5",
            )
            .bind(parent_card_id)
            .bind(target.column_id)
            .bind(row.id)
            .bind(input.retro_id)
            .bind(target.id)
            .execute(&mut *tx)
            .await?;

            (row.id, parent_card_id)
        };

        if source_is_group_card {
            sqlx::query(
                "WITH RECURSIVE descendants AS (
                    SELECT id
                    FROM cards
                    WHERE retro_id = $4 AND parent_card_id = $5
                    UNION ALL
                    SELECT child.id
                    FROM cards child
                    JOIN descendants parent ON child.parent_card_id = parent.id
                    WHERE child.retro_id = $4
                 ),
                 group_cards AS (
                    SELECT DISTINCT parent.id
                    FROM descendants parent
                    JOIN cards child ON child.parent_card_id = parent.id
                 ),
                 leaf_cards AS (
                    SELECT descendants.id
                    FROM descendants
                    LEFT JOIN group_cards ON group_cards.id = descendants.id
                    WHERE group_cards.id IS NULL
                 )
                 UPDATE cards
                 SET cluster_id = $1,
                     parent_card_id = $2,
                     column_id = $3,
                     updated_at = NOW()
                 WHERE id IN (SELECT id FROM leaf_cards)",
            )
            .bind(cluster_id)
            .bind(cluster_parent_id)
            .bind(target.column_id)
            .bind(input.retro_id)
            .bind(source.id)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "WITH RECURSIVE descendants AS (
                    SELECT id
                    FROM cards
                    WHERE retro_id = $2 AND parent_card_id = $1
                    UNION ALL
                    SELECT child.id
                    FROM cards child
                    JOIN descendants parent ON child.parent_card_id = parent.id
                    WHERE child.retro_id = $2
                 ),
                 group_cards AS (
                    SELECT DISTINCT parent.id
                    FROM descendants parent
                    JOIN cards child ON child.parent_card_id = parent.id
                 )
                 DELETE FROM cards
                 WHERE retro_id = $2 AND (id = $1 OR id IN (SELECT id FROM group_cards))",
            )
                .bind(source.id)
                .bind(input.retro_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                "UPDATE cards
                 SET cluster_id = $1,
                     parent_card_id = $5,
                     column_id = $6,
                     updated_at = NOW()
                 WHERE retro_id = $2 AND id = $3",
            )
            .bind(cluster_id)
            .bind(input.retro_id)
            .bind(input.card_id)
            .bind(input.target_card_id)
            .bind(cluster_parent_id)
            .bind(target.column_id)
            .execute(&mut *tx)
            .await?;
        }

        let cluster = sqlx::query_as::<_, ClusterRow>(
            "SELECT id, retro_id, title, category, tags
             FROM card_clusters
             WHERE id = $1",
        )
        .bind(cluster_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(cluster.into())
    }

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
            let action_column_id =
                action_column_id.ok_or_else(|| ActionError::Invalid("action column unavailable".to_owned()))?;
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

    pub async fn create_ai_artifact(
        &self,
        retro_id: Uuid,
        kind: &str,
        input: Value,
    ) -> Result<AiArtifactRecord, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "INSERT INTO ai_artifacts (retro_id, kind, status, input)
             VALUES ($1, $2, 'pending', $3)
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(retro_id)
        .bind(kind)
        .bind(Json(input))
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn create_meeting_note(
        &self,
        input: CreateMeetingNoteInput,
    ) -> Result<MeetingNoteRecord, sqlx::Error> {
        let participant_id = self
            .ensure_participant(
                input.retro_id,
                &input.author_subject,
                &input.author_display_name,
            )
            .await?;
        let row = sqlx::query_as::<_, MeetingNoteRecord>(
            "INSERT INTO meeting_notes (retro_id, author_participant_id, title, body_text)
             VALUES ($1, $2, $3, $4)
             RETURNING id, retro_id, author_participant_id, title, body_text",
        )
        .bind(input.retro_id)
        .bind(participant_id)
        .bind(input.title.trim())
        .bind(input.body_text.trim())
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn ai_input_with_note_context(
        &self,
        retro_id: Uuid,
        kind: &str,
    ) -> Result<Value, sqlx::Error> {
        let notes = self.fetch_meeting_notes(retro_id).await?;
        Ok(serde_json::json!({
            "provider": "fake",
            "kind": kind,
            "meeting_notes_included": matches!(kind, "summary" | "mood") && !notes.is_empty(),
            "meeting_notes": if matches!(kind, "summary" | "mood") { notes } else { Vec::new() },
        }))
    }

    pub async fn mark_ai_running(
        &self,
        artifact_id: Uuid,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'running', error_message = NULL, updated_at = NOW()
             WHERE id = $1
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn retry_ai_artifact(
        &self,
        retro_id: Uuid,
        artifact_id: Uuid,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'running', error_message = NULL, retry_count = retry_count + 1, updated_at = NOW()
             WHERE id = $1 AND retro_id = $2
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn complete_ai_artifact(
        &self,
        artifact_id: Uuid,
        output: Value,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'succeeded', output = $2, error_message = NULL, updated_at = NOW()
             WHERE id = $1
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .bind(Json(output))
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn fail_ai_artifact(
        &self,
        artifact_id: Uuid,
        error_message: &str,
    ) -> Result<Option<AiArtifactRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, AiArtifactRow>(
            "UPDATE ai_artifacts
             SET status = 'failed', error_message = $2, updated_at = NOW()
             WHERE id = $1
             RETURNING id, retro_id, kind, status, input, output, error_message, retry_count",
        )
        .bind(artifact_id)
        .bind(error_message)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn create_delivery(
        &self,
        retro_id: Uuid,
        kind: &str,
        output: Value,
        fail: bool,
    ) -> Result<DeliveryRecord, sqlx::Error> {
        let status = if fail { "failed" } else { "succeeded" };
        let error_message = if fail {
            Some("fake delivery failure")
        } else {
            None
        };
        let row = sqlx::query_as::<_, DeliveryRow>(
            "INSERT INTO deliveries (retro_id, kind, status, output, error_message)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, retro_id, kind, status, output, error_message, retry_count",
        )
        .bind(retro_id)
        .bind(kind)
        .bind(status)
        .bind(Json(output))
        .bind(error_message)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn retry_delivery(
        &self,
        retro_id: Uuid,
        delivery_id: Uuid,
    ) -> Result<Option<DeliveryRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, DeliveryRow>(
            "UPDATE deliveries
             SET status = 'succeeded', error_message = NULL, retry_count = retry_count + 1, updated_at = NOW()
             WHERE id = $1 AND retro_id = $2
             RETURNING id, retro_id, kind, status, output, error_message, retry_count",
        )
        .bind(delivery_id)
        .bind(retro_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn export_summary_payload(&self, retro_id: Uuid) -> Result<Value, sqlx::Error> {
        let Some(board) = self.fetch_board(retro_id).await? else {
            return Err(sqlx::Error::RowNotFound);
        };
        let confirmed_actions = board
            .actions
            .iter()
            .filter(|action| action.status != "rejected")
            .map(|action| {
                serde_json::json!({
                    "title": action.title,
                    "details": action.details,
                    "status": action.status,
                    "external_action_link": "https://example.invalid/spillio/action-placeholder"
                })
            })
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "title": board.retro.title,
            "phase": board.retro.phase,
            "copy_markdown": format!("# {}\n\nActions: {}", board.retro.title, confirmed_actions.len()),
            "confirmed_actions": confirmed_actions,
            "meeting_notes_count": board.meeting_notes.len(),
            "ai_summary": board.ai_artifacts.iter().find(|artifact| artifact.kind == "summary").and_then(|artifact| artifact.output.clone()),
        }))
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
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RetroRecord {
    pub id: Uuid,
    pub title: String,
    pub phase: String,
    pub vote_limit: i32,
    pub action_discussion_limit: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetroBoard {
    pub retro: RetroRecord,
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
    pub body_text: Option<String>,
    pub gif_url: Option<String>,
    pub gif_alt_text: Option<String>,
    pub hidden: bool,
}

impl From<&CardRecord> for ClusterMemberRecord {
    fn from(card: &CardRecord) -> Self {
        Self {
            id: card.id,
            body_text: card.body_text.clone(),
            gif_url: card.gif_url.clone(),
            gif_alt_text: card.gif_alt_text.clone(),
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
    pub last_activity_at: String,
    pub last_opened_at: Option<String>,
    pub participant_count: i64,
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
    last_activity_at: String,
    last_opened_at: Option<String>,
    participant_count: i64,
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
            last_activity_at: row.last_activity_at,
            last_opened_at: row.last_opened_at,
            participant_count: row.participant_count,
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

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRetroInput {
    pub title: String,
    pub creator_subject: String,
    pub creator_display_name: String,
    pub template: RetroTemplate,
    pub vote_limit: i32,
    pub action_discussion_limit: i32,
    pub column_colors: Vec<String>,
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

fn column_key(title: &str, position: usize) -> String {
    let slug = title
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character.is_whitespace() || character == '-' || character == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned();

    if slug.is_empty() {
        format!("column_{position}")
    } else {
        format!("{position}_{slug}")
    }
}

fn column_accent_color(title: &str, position: usize) -> &'static str {
    let title = title.to_lowercase();
    if title.contains("action") {
        "#8757b6"
    } else if title.contains("well") || title.contains("liked") {
        "#2f9469"
    } else if title.contains("wrong") || title.contains("lacked") || title.contains("improve") {
        "#cf4f4f"
    } else if title.contains("learned") {
        "#0f5f72"
    } else if title.contains("longed") {
        "#cf8a3f"
    } else if title.contains("feeling") {
        "#0f5f72"
    } else if title.contains("mood") {
        "#cf8a3f"
    } else {
        ["#cf8a3f", "#2f9469", "#cf4f4f", "#8757b6"][position % 4]
    }
}

fn cluster_key(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .find(|word| word.len() >= 4)
}

fn manual_cluster_title(source: &str, target: &str) -> String {
    let key = cluster_key(source)
        .or_else(|| cluster_key(target))
        .unwrap_or_else(|| "cards".to_owned());
    format!("Grouped: {key}")
}

fn action_tags(text: &str) -> Vec<String> {
    let mut tags = text
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|word| word.len() >= 4)
        .take(3)
        .collect::<Vec<_>>();
    if !tags.iter().any(|tag| tag == "topvoted") {
        tags.push("topvoted".to_owned());
    }
    tags
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
                creator_display_name: "Ava".to_owned(),
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 3,
                column_colors: Vec::new(),
            })
            .await
            .unwrap();

        assert_eq!(created.retro.title, "Sprint 43");
        assert_eq!(created.retro.phase, "writing");
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
    async fn creates_custom_retro_with_supplied_columns(pool: PgPool) {
        let repo = RetroRepository::new(pool);

        let created = repo
            .create_retro(CreateRetroInput {
                title: "Team pulse".to_owned(),
                creator_subject: "user-456".to_owned(),
                creator_display_name: "Lee".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
            .move_draft_card(created.retro.id, card.id, created.columns[1].id, None, "ava")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(moved.column_id, created.columns[1].id);

        let lee_attempt = repo
            .move_draft_card(created.retro.id, card.id, created.columns[2].id, None, "lee")
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
    async fn manual_clustering_across_columns_moves_source_to_target_column(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Cross column cluster retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
            board.columns
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
                creator_display_name: "Ava".to_owned(),
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
}
