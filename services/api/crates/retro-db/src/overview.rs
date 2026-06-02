use sqlx::PgPool;

use crate::{RetroOverview, RetroSummary, RetroSummaryRow};

pub(super) async fn list_retros(
    pool: &PgPool,
    subject: &str,
    email: &str,
) -> Result<RetroOverview, sqlx::Error> {
    let rows = sqlx::query_as::<_, RetroSummaryRow>(
        "SELECT
            r.id,
            r.title,
            r.phase,
            r.vote_limit,
            r.action_discussion_limit,
            to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
            to_char(r.planned_for, 'YYYY-MM-DD') AS planned_for,
            to_char(r.happened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS happened_at,
            g.name AS group_name,
            CASE
                WHEN r.creator_email <> '' AND r.creator_email = lower($2) THEN 'host'
                WHEN EXISTS (
                    SELECT 1
                    FROM board_grants bg
                    WHERE bg.retro_id = r.id
                      AND bg.principal_email = lower($2)
                      AND bg.role = 'host'
                ) THEN 'host'
                ELSE COALESCE(
                    (
                        SELECT scoped_participant.role
                        FROM participants scoped_participant
                        WHERE scoped_participant.retro_id = r.id
                          AND scoped_participant.external_subject = $1
                        ORDER BY CASE WHEN scoped_participant.role = 'host' THEN 0 ELSE 1 END
                        LIMIT 1
                    ),
                    'member'
                )
            END AS current_user_role,
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
            COUNT(DISTINCT rm.participant_id) FILTER (
                WHERE rm.phase = CASE WHEN r.phase = 'voting' THEN 'voting' ELSE 'writing' END
            )::BIGINT AS ready_count,
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
         LEFT JOIN retro_groups g ON g.id = r.group_id
         LEFT JOIN participants p ON p.retro_id = r.id
         LEFT JOIN retro_columns c ON c.retro_id = r.id
         LEFT JOIN participant_ready_marks rm ON rm.retro_id = r.id
         LEFT JOIN action_items a ON a.retro_id = r.id
         LEFT JOIN LATERAL jsonb_array_elements_text(a.tags) AS tag(value) ON true
         WHERE EXISTS (
             SELECT 1
             FROM participants scoped_participant
             LEFT JOIN retro_accesses scoped_access ON scoped_access.participant_id = scoped_participant.id
             WHERE scoped_participant.retro_id = r.id
               AND scoped_participant.external_subject = $1
               AND (scoped_participant.role = 'host' OR scoped_access.retro_id = r.id)
         ) OR EXISTS (
             SELECT 1
             FROM board_grants scoped_grant
             WHERE scoped_grant.retro_id = r.id
               AND scoped_grant.principal_email = lower($2)
         )
         GROUP BY r.id, g.name
         ORDER BY last_activity_at DESC, r.created_at DESC",
    )
    .bind(subject)
    .bind(email.trim())
    .fetch_all(pool)
    .await?;

    let summaries = rows.into_iter().map(RetroSummary::from).collect::<Vec<_>>();
    let (completed, active): (Vec<_>, Vec<_>) = summaries
        .into_iter()
        .partition(|summary| summary.phase == "completed");

    Ok(RetroOverview { active, completed })
}
