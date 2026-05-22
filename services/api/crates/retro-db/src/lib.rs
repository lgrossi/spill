use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
        })
    }

    pub async fn fetch_board(&self, id: Uuid) -> Result<Option<RetroBoard>, sqlx::Error> {
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let columns = self.fetch_columns(id).await?;
        let ready = self.ready_info(id, "").await?;
        Ok(Some(RetroBoard {
            retro,
            columns,
            ready,
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
        let cards = self.fetch_cards_for_user(id, subject).await?;
        for column in &mut columns {
            column.cards = cards
                .iter()
                .filter(|card| card.column_id == column.id)
                .cloned()
                .collect();
        }
        let ready = self.ready_info(id, subject).await?;
        Ok(Some(RetroBoard {
            retro,
            columns,
            ready,
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

    pub async fn list_retros(&self) -> Result<RetroOverview, sqlx::Error> {
        let summaries = sqlx::query_as::<_, RetroSummary>(
            "SELECT
                r.id,
                r.title,
                r.phase,
                r.vote_limit,
                r.action_discussion_limit,
                COUNT(DISTINCT p.id)::BIGINT AS participant_count,
                COUNT(DISTINCT c.id)::BIGINT AS column_count
             FROM retros r
             LEFT JOIN participants p ON p.retro_id = r.id
             LEFT JOIN retro_columns c ON c.retro_id = r.id
             GROUP BY r.id
             ORDER BY r.created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

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
            "INSERT INTO cards (retro_id, column_id, author_participant_id, body_text, state, position)
             VALUES (
                $1,
                $2,
                $3,
                $4,
                'draft',
                (SELECT COALESCE(MAX(position) + 1, 0) FROM cards WHERE retro_id = $1 AND column_id = $2)
             )
             RETURNING id, retro_id, column_id, author_participant_id, body_text, gif_url, gif_alt_text, state, position, false AS hidden",
        )
        .bind(input.retro_id)
        .bind(input.column_id)
        .bind(participant_id)
        .bind(input.body_text.trim())
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_draft_card(
        &self,
        card_id: Uuid,
        subject: &str,
        body_text: &str,
    ) -> Result<Option<CardRecord>, sqlx::Error> {
        sqlx::query_as::<_, CardRecord>(
            "UPDATE cards c
             SET body_text = $3, updated_at = NOW()
             FROM participants p
             WHERE c.id = $1
               AND c.author_participant_id = p.id
               AND p.external_subject = $2
               AND c.state = 'draft'
             RETURNING c.id, c.retro_id, c.column_id, c.author_participant_id, c.body_text, c.gif_url, c.gif_alt_text, c.state, c.position, false AS hidden",
        )
        .bind(card_id)
        .bind(subject)
        .bind(body_text.trim())
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
             VALUES ($1, $2, 'writing')
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
                c.gif_url,
                c.gif_alt_text,
                c.state,
                c.position,
                (r.phase = 'writing' AND c.state = 'draft' AND p.external_subject IS DISTINCT FROM $2) AS hidden
             FROM cards c
             JOIN participants p ON p.id = c.author_participant_id
             JOIN retros r ON r.id = c.retro_id
             WHERE c.retro_id = $1
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
             LEFT JOIN participant_ready_marks m ON m.participant_id = p.id AND m.phase = 'writing'
             WHERE p.retro_id = $1",
        )
        .bind(retro_id)
        .bind(subject)
        .fetch_one(&self.pool)
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
}

#[derive(Debug, Clone, Serialize)]
pub struct RetroBoard {
    pub retro: RetroRecord,
    pub columns: Vec<RetroColumnRecord>,
    pub ready: ReadyInfo,
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
    pub hidden: bool,
}

#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
pub struct ReadyInfo {
    pub participant_count: i64,
    pub ready_count: i64,
    pub current_user_ready: bool,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RetroSummary {
    pub id: Uuid,
    pub title: String,
    pub phase: String,
    pub vote_limit: i32,
    pub action_discussion_limit: i32,
    pub participant_count: i64,
    pub column_count: i64,
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
    pub body_text: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ParticipantId {
    id: Uuid,
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
            body_text: "Ava can read this".to_owned(),
        })
        .await
        .unwrap();
        repo.create_draft_card(DraftCardInput {
            retro_id: created.retro.id,
            column_id,
            author_subject: "lee".to_owned(),
            author_display_name: "Lee".to_owned(),
            body_text: "Lee private draft".to_owned(),
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
}
