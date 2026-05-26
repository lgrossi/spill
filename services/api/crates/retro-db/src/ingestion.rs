use crate::*;

impl RetroRepository {
    pub async fn ingest_item(
        &self,
        input: IngestItemInput,
    ) -> Result<IngestedItemRecord, sqlx::Error> {
        let participant_id = self
            .ensure_participant(input.retro_id, &input.subject, &input.display_name)
            .await?;

        if let Some(idempotency_key) = input.idempotency_key.as_deref()
            && let Some(existing) = self
                .fetch_ingested_by_idempotency(participant_id, &input.source, idempotency_key)
                .await?
        {
            return Ok(existing);
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
}
