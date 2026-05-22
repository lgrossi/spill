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
        let columns = input.template.column_titles();
        let mut tx = self.pool.begin().await?;

        let retro = sqlx::query_as::<_, RetroRecord>(
            "INSERT INTO retros (title, vote_limit, action_discussion_limit)
             VALUES ($1, $2, $3)
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
            let record = sqlx::query_as::<_, RetroColumnRow>(
                "INSERT INTO retro_columns (retro_id, column_key, title, position)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, retro_id, column_key, title, position, order_direction",
            )
            .bind(retro.id)
            .bind(column_key(title, position))
            .bind(title.trim())
            .bind(position as i32)
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
        self.ensure_participant(id, subject, display_name).await?;
        let mut columns = self.fetch_columns(id).await?;
        let clusters = self.fetch_clusters(id).await?;
        let cards = self.fetch_cards_for_user(id, subject).await?;
        let actions = self.fetch_actions(id).await?;
        let deck = self.fetch_deck(id, subject).await?;
        let ai_artifacts = self.fetch_ai_artifacts(id).await?;
        let meeting_notes = self.fetch_meeting_notes(id).await?;
        let deliveries = self.fetch_deliveries(id).await?;
        for column in &mut columns {
            column.cards = cards
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
            "SELECT id, retro_id, column_key, title, position, order_direction
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

    pub async fn list_retros(&self) -> Result<RetroOverview, sqlx::Error> {
        let rows = sqlx::query_as::<_, RetroSummaryRow>(
            "SELECT
                r.id,
                r.title,
                r.phase,
                r.vote_limit,
                r.action_discussion_limit,
                COUNT(DISTINCT p.id)::BIGINT AS participant_count,
                COUNT(DISTINCT c.id)::BIGINT AS column_count,
                COUNT(DISTINCT a.id) FILTER (WHERE a.status <> 'rejected')::BIGINT AS unresolved_action_count,
                COALESCE(jsonb_agg(DISTINCT tag.value) FILTER (WHERE tag.value IS NOT NULL), '[]'::jsonb) AS recurring_tags
             FROM retros r
             LEFT JOIN participants p ON p.retro_id = r.id
             LEFT JOIN retro_columns c ON c.retro_id = r.id
             LEFT JOIN action_items a ON a.retro_id = r.id
             LEFT JOIN LATERAL jsonb_array_elements_text(a.tags) AS tag(value) ON true
             GROUP BY r.id
             ORDER BY r.created_at DESC",
        )
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
                'draft',
                (SELECT COALESCE(MAX(position) + 1, 0) FROM cards WHERE retro_id = $1 AND column_id = $2)
             )
             RETURNING id, retro_id, column_id, author_participant_id, body_text, gif_url, gif_alt_text, state, position, NULL::UUID AS cluster_id, NULL::TEXT AS cluster_title, NULL::TEXT AS cluster_category, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
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
    ) -> Result<Option<CardRecord>, sqlx::Error> {
        sqlx::query_as::<_, CardRecord>(
            "UPDATE cards c
             SET body_text = $3, gif_url = $4, gif_alt_text = $5, updated_at = NOW()
             FROM participants p
             WHERE c.id = $1
               AND c.author_participant_id = p.id
               AND p.external_subject = $2
               AND c.state = 'draft'
             RETURNING c.id, c.retro_id, c.column_id, c.author_participant_id, c.body_text, c.gif_url, c.gif_alt_text, c.state, c.position, c.cluster_id, NULL::TEXT AS cluster_title, NULL::TEXT AS cluster_category, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
        )
        .bind(card_id)
        .bind(subject)
        .bind(body_text.map(str::trim).filter(|value| !value.is_empty()))
        .bind(gif_url.map(str::trim).filter(|value| !value.is_empty()))
        .bind(gif_alt_text.map(str::trim).filter(|value| !value.is_empty()))
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete_draft_card(
        &self,
        card_id: Uuid,
        subject: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM cards c
             USING participants p
             WHERE c.id = $1
               AND c.author_participant_id = p.id
               AND p.external_subject = $2
               AND c.state = 'draft'",
        )
        .bind(card_id)
        .bind(subject)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
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
             WHERE id = $1 AND phase = 'discussion'
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

    pub async fn start_action_discussion(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<ActionItemRecord>, ActionError> {
        let mut tx = self.pool.begin().await?;
        let retro = sqlx::query_as::<_, RetroRecord>(
            "UPDATE retros
             SET phase = 'action_discussion'
             WHERE id = $1 AND phase = 'voting'
             RETURNING id, title, phase, vote_limit, action_discussion_limit",
        )
        .bind(retro_id)
        .fetch_one(&mut *tx)
        .await?;

        let candidates = sqlx::query_as::<_, ActionCandidate>(
            "SELECT c.id, COALESCE(c.body_text, c.gif_alt_text, 'Untitled card') AS title, COALESCE(SUM(v.count), 0)::BIGINT AS vote_count
             FROM cards c
             LEFT JOIN votes v ON v.target_card_id = c.id
             WHERE c.retro_id = $1 AND c.state = 'revealed'
             GROUP BY c.id
             ORDER BY vote_count DESC, c.created_at ASC
             LIMIT $2",
        )
        .bind(retro_id)
        .bind(retro.action_discussion_limit as i64)
        .fetch_all(&mut *tx)
        .await?;

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

    pub async fn set_action_status(
        &self,
        retro_id: Uuid,
        action_id: Uuid,
        status: &str,
    ) -> Result<Option<ActionItemRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, ActionItemRow>(
            "UPDATE action_items
             SET status = $3, confirmed_at = CASE WHEN $3 = 'confirmed' THEN NOW() ELSE confirmed_at END
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
    pub cluster_title: Option<String>,
    pub cluster_category: Option<String>,
    pub vote_count: i64,
    pub current_user_vote_count: i64,
    pub hidden: bool,
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
    pub participant_count: i64,
    pub column_count: i64,
    pub unresolved_action_count: i64,
    pub recurring_tags: Vec<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct RetroSummaryRow {
    id: Uuid,
    title: String,
    phase: String,
    vote_limit: i32,
    action_discussion_limit: i32,
    participant_count: i64,
    column_count: i64,
    unresolved_action_count: i64,
    recurring_tags: Json<Vec<String>>,
}

impl From<RetroSummaryRow> for RetroSummary {
    fn from(row: RetroSummaryRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            phase: row.phase,
            vote_limit: row.vote_limit,
            action_discussion_limit: row.action_discussion_limit,
            participant_count: row.participant_count,
            column_count: row.column_count,
            unresolved_action_count: row.unresolved_action_count,
            recurring_tags: row.recurring_tags.0,
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
pub struct UpdateActionInput {
    pub retro_id: Uuid,
    pub action_id: Uuid,
    pub title: String,
    pub details: Option<String>,
}

#[derive(Debug)]
pub enum ActionError {
    Sqlx(sqlx::Error),
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
            Self::Standard => ["Mood", "Went well", "Went wrong", "Actions"]
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
            ["Mood", "Went well", "Went wrong", "Actions"]
        );

        let overview = repo.list_retros().await.unwrap();
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
                        "Actions".to_owned(),
                    ],
                },
                vote_limit: 5,
                action_discussion_limit: 2,
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
            .update_draft_card(card.id, "ava", Some("text only now"), None, None)
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
    async fn action_discussion_creates_editable_top_voted_actions(pool: PgPool) {
        let repo = RetroRepository::new(pool);
        let created = repo
            .create_retro(CreateRetroInput {
                title: "Actions retro".to_owned(),
                creator_subject: "ava".to_owned(),
                creator_display_name: "Ava".to_owned(),
                template: RetroTemplate::Standard,
                vote_limit: 3,
                action_discussion_limit: 2,
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

        let overview = repo.list_retros().await.unwrap();
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
