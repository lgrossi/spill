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
            let record = sqlx::query_as::<_, RetroColumnRecord>(
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
            records.push(record);
        }

        tx.commit().await?;

        Ok(RetroBoard {
            retro,
            columns: records,
        })
    }

    pub async fn fetch_board(&self, id: Uuid) -> Result<Option<RetroBoard>, sqlx::Error> {
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let columns = self.fetch_columns(id).await?;
        Ok(Some(RetroBoard { retro, columns }))
    }

    pub async fn fetch_columns(
        &self,
        retro_id: Uuid,
    ) -> Result<Vec<RetroColumnRecord>, sqlx::Error> {
        sqlx::query_as::<_, RetroColumnRecord>(
            "SELECT id, retro_id, column_key, title, position, order_direction
             FROM retro_columns
             WHERE retro_id = $1
             ORDER BY position ASC",
        )
        .bind(retro_id)
        .fetch_all(&self.pool)
        .await
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
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RetroColumnRecord {
    pub id: Uuid,
    pub retro_id: Uuid,
    pub column_key: String,
    pub title: String,
    pub position: i32,
    pub order_direction: String,
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
}
