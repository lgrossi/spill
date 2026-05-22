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
            voting: VotingInfo::default(),
        })
    }

    pub async fn fetch_board(&self, id: Uuid) -> Result<Option<RetroBoard>, sqlx::Error> {
        let Some(retro) = self.fetch_retro(id).await? else {
            return Ok(None);
        };
        let columns = self.fetch_columns(id).await?;
        let ready = self.ready_info(id, "").await?;
        let voting = self.voting_info(id, "").await?;
        Ok(Some(RetroBoard {
            retro,
            columns,
            ready,
            voting,
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
        let voting = self.voting_info(id, subject).await?;
        Ok(Some(RetroBoard {
            retro,
            columns,
            ready,
            voting,
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
             RETURNING id, retro_id, column_id, author_participant_id, body_text, gif_url, gif_alt_text, state, position, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
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
    ) -> Result<Option<CardRecord>, sqlx::Error> {
        sqlx::query_as::<_, CardRecord>(
            "UPDATE cards c
             SET body_text = $3, gif_url = $4, gif_alt_text = $5, updated_at = NOW()
             FROM participants p
             WHERE c.id = $1
               AND c.author_participant_id = p.id
               AND p.external_subject = $2
               AND c.state = 'draft'
             RETURNING c.id, c.retro_id, c.column_id, c.author_participant_id, c.body_text, c.gif_url, c.gif_alt_text, c.state, c.position, 0::BIGINT AS vote_count, 0::BIGINT AS current_user_vote_count, false AS hidden",
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
                COALESCE(SUM(v.count), 0)::BIGINT AS vote_count,
                COALESCE(SUM(CASE WHEN vp.external_subject = $2 THEN v.count ELSE 0 END), 0)::BIGINT AS current_user_vote_count,
                (r.phase = 'writing' AND c.state = 'draft' AND p.external_subject IS DISTINCT FROM $2) AS hidden
             FROM cards c
             JOIN participants p ON p.id = c.author_participant_id
             JOIN retros r ON r.id = c.retro_id
             LEFT JOIN votes v ON v.target_card_id = c.id
             LEFT JOIN participants vp ON vp.id = v.participant_id
             WHERE c.retro_id = $1
             GROUP BY c.id, r.phase, p.external_subject
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
    pub vote_count: i64,
    pub current_user_vote_count: i64,
    pub hidden: bool,
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
    pub body_text: Option<String>,
    pub gif_url: Option<String>,
    pub gif_alt_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CastVoteInput {
    pub retro_id: Uuid,
    pub card_id: Uuid,
    pub subject: String,
    pub display_name: String,
    pub count: i32,
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
}
