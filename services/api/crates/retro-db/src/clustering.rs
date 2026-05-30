use crate::{
    domain_mapping::{cluster_key, manual_cluster_title},
    *,
};

impl RetroRepository {
    pub async fn cluster_board_if_auto(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<ClusterRecord>, ClusterError> {
        let retro = sqlx::query_as::<_, ClusteringRetro>(
            "SELECT id, phase, clustering_mode, clustering_status FROM retros WHERE id = $1",
        )
        .bind(retro_id)
        .fetch_one(&self.pool)
        .await?;

        if retro.clustering_mode != "auto_on_vote_start" || retro.clustering_status != "not_run" {
            return Ok(Vec::new());
        }

        self.cluster_board(retro_id).await
    }

    pub async fn mark_clustering_failed(&self, retro_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE retros SET clustering_status = 'failed' WHERE id = $1")
            .bind(retro_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn continue_unclustered(&self, retro_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE retros SET clustering_status = 'completed' WHERE id = $1")
            .bind(retro_id)
            .execute(&self.pool)
            .await?;
        Ok(())
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

    pub async fn cluster_cards(
        &self,
        input: ClusterCardsInput,
    ) -> Result<ClusterRecord, ClusterError> {
        if input.card_id == input.target_card_id {
            return Err(ClusterError::Invalid(
                "cannot cluster a card with itself".to_owned(),
            ));
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
            return Err(ClusterError::Invalid(
                "cluster target is not available".to_owned(),
            ));
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
        let (cluster_id, cluster_parent_id) = if target.parent_card_id.is_some()
            || target.cluster_id.is_some()
        {
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
}
