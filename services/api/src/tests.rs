use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::Value;
use tower::ServiceExt;

#[test]
fn health_response_identifies_the_api() {
    let response = health_response();

    assert_eq!(response.status, "ok");
    assert_eq!(response.service, "spillio-api");
}

#[tokio::test]
async fn board_event_hub_sends_snapshot_then_mutation_events() {
    let hub = BoardEventHub::default();
    let retro_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();

    let mut first = hub.subscribe(retro_id);
    assert_eq!(
        first.recv().await.unwrap(),
        BoardEvent::BoardSnapshot { retro_id }
    );

    hub.publish(BoardEvent::CardChanged { retro_id });
    assert_eq!(
        first.recv().await.unwrap(),
        BoardEvent::CardChanged { retro_id }
    );

    let mut reconnected = hub.subscribe(retro_id);
    assert_eq!(
        reconnected.recv().await.unwrap(),
        BoardEvent::BoardSnapshot { retro_id }
    );
}

#[tokio::test]
async fn session_endpoint_returns_identity_from_platform_headers() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .header(HEADER_ON_BEHALF_OF, "user-123@example.com")
                .header(HEADER_USER_NAME, "Ava")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();

    // Subject is derived from the email; the client no longer dictates it.
    assert_eq!(body["user"]["email"], "user-123@example.com");
    assert!(
        body["user"]["subject"]
            .as_str()
            .unwrap()
            .starts_with("email:")
    );
    assert_eq!(body["user"]["display_name"], "Ava");
    assert_eq!(body["access_model"]["kind"], "link");
    assert_eq!(body["access_model"]["can_edit_with_link"], true);
}

#[tokio::test]
async fn session_endpoint_returns_structured_error_without_identity() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();

    assert_eq!(body["error"]["code"], "unauthorized");
}

// Regression: in token mode, a request carrying only identity headers (the
// original exploit) must be rejected — no token, no access.
#[tokio::test]
async fn token_mode_rejects_header_only_request() {
    let auth = identity::AuthState::token_test(None);
    let response = app_with_auth(auth)
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .header(HEADER_ON_BEHALF_OF, "user-123@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retro_endpoints_create_list_and_open_standard_board(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "user-123@example.com")
                    .header(HEADER_USER_NAME, "Ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Sprint 43","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(created["retro"]["phase"], "writing");
    assert_eq!(created["participants"][0]["display_name"], "Ava");
    assert_eq!(created["participants"][0]["role"], "host");
    assert_eq!(
        created["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["How are you feeling?", "Went well", "To improve", "Actions"]
    );

    let retro_id = created["retro"]["id"].as_str().unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "user-123@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "user-123@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let overview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(overview["active"].as_array().unwrap().len(), 1);
    assert_eq!(overview["completed"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn create_retro_rejects_invalid_invitee_role_before_persisting(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Bad invite","template":"standard","invitees":[{"email":"lee@example.com","role":"owner"}]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let overview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(overview["active"].as_array().unwrap().len(), 0);
    assert_eq!(overview["completed"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn create_retro_rejects_invalid_card_edit_policy_before_persisting(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Bad privacy","template":"standard","card_edit_policy":"private"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let overview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(overview["active"].as_array().unwrap().len(), 0);
    assert_eq!(overview["completed"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn writing_endpoints_hide_other_drafts_until_reveal(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header(HEADER_USER_NAME, "Ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Writing API retro","template":"standard","vote_limit":3,"action_discussion_limit":3,"invitees":[{"email":"lee@example.com","role":"member"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    for (subject, body) in [
        ("ava@example.com", "Ava draft"),
        ("lee@example.com", "Lee private draft"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/cards"))
                    .header(HEADER_ON_BEHALF_OF, subject)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"column_id":"{column_id}","body_text":"{body}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ava_board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let participant_ids = ava_board["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|participant| participant["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(participant_ids.len(), 2);
    assert!(
        participant_ids.contains(
            &ava_board["columns"][0]["cards"][0]["author_participant_id"]
                .as_str()
                .unwrap()
        )
    );
    assert!(
        participant_ids.contains(
            &ava_board["columns"][0]["cards"][1]["author_participant_id"]
                .as_str()
                .unwrap()
        )
    );

    assert_eq!(
        ava_board["columns"][0]["cards"][0]["body_text"],
        "Ava draft"
    );
    assert_eq!(
        ava_board["columns"][0]["cards"][1]["body_text"],
        Value::Null
    );
    assert_eq!(ava_board["columns"][0]["cards"][1]["hidden"], true);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ready"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ready"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let revealed: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(revealed["retro"]["phase"], "discussion");
    // After reveal both drafts are visible; author-grouping decides their order,
    // so assert presence rather than a fixed index.
    let revealed_bodies: Vec<&str> = revealed["columns"][0]["cards"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|card| card["body_text"].as_str())
        .collect();
    assert!(revealed_bodies.contains(&"Lee private draft"));
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn participant_mutations_require_board_membership(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header(HEADER_USER_NAME, "Alice")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Membership gate","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"uninvited write"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn due_scheduled_retro_can_be_started_by_member_but_future_start_requires_host(
    pool: sqlx::PgPool,
) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool.clone()));
    let future_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Future planned retro","template":"standard","planned_for":"2099-05-15","invitees":[{"email":"member@example.com","role":"member"}]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(future_response.status(), StatusCode::CREATED);
    let future: Value = serde_json::from_slice(
        &to_bytes(future_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(future["retro"]["phase"], "scheduled");
    let future_retro_id = future["retro"]["id"].as_str().unwrap();

    let member_future_start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{future_retro_id}/start"))
                .header(HEADER_ON_BEHALF_OF, "member@example.com")
                .header(HEADER_USER_NAME, "Member")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(member_future_start.status(), StatusCode::FORBIDDEN);

    let due_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Due planned retro","template":"standard","planned_for":"2099-06-20","invitees":[{"email":"member@example.com","role":"member"}]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(due_response.status(), StatusCode::CREATED);
    let due: Value = serde_json::from_slice(
        &to_bytes(due_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let due_retro_id = due["retro"]["id"].as_str().unwrap();
    sqlx::query("UPDATE retros SET planned_for = DATE '2000-01-02' WHERE id = $1::uuid")
        .bind(due_retro_id)
        .execute(&pool)
        .await
        .unwrap();

    let member_due_start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{due_retro_id}/start"))
                .header(HEADER_ON_BEHALF_OF, "member@example.com")
                .header(HEADER_USER_NAME, "Member")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(member_due_start.status(), StatusCode::OK);
    let started: Value = serde_json::from_slice(
        &to_bytes(member_due_start.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(started["retro"]["phase"], "writing");

    let repeated_due_start = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{due_retro_id}/start"))
                .header(HEADER_ON_BEHALF_OF, "member@example.com")
                .header(HEADER_USER_NAME, "Member")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(repeated_due_start.status(), StatusCode::OK);
    let repeated: Value = serde_json::from_slice(
        &to_bytes(repeated_due_start.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(repeated["retro"]["phase"], "writing");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_can_reschedule_only_while_retro_is_scheduled(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Future planned retro","template":"standard","planned_for":"2099-05-15","invitees":[{"email":"member@example.com","role":"member"}]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: Value = serde_json::from_slice(
        &to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let member_reschedule = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reschedule"))
                .header(HEADER_ON_BEHALF_OF, "member@example.com")
                .header(HEADER_USER_NAME, "Member")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"planned_for":"2099-05-16"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(member_reschedule.status(), StatusCode::FORBIDDEN);

    let host_reschedule = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reschedule"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"planned_for":"2099-05-16"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(host_reschedule.status(), StatusCode::OK);
    let updated: Value = serde_json::from_slice(
        &to_bytes(host_reschedule.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(updated["retro"]["phase"], "scheduled");
    assert_eq!(updated["retro"]["planned_for"], "2099-05-16");

    let start_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/start"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(start_response.status(), StatusCode::OK);

    let writing_reschedule = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reschedule"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"planned_for":"2099-05-17"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(writing_reschedule.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn create_retro_rejects_invalid_planned_date(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Bad date","template":"standard","planned_for":"not-a-date"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn scheduled_retro_rejects_card_creation_with_controlled_error(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Future planned retro","template":"standard","planned_for":"2099-05-15"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: Value = serde_json::from_slice(
        &to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    let card_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"too early"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(card_response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn gif_endpoints_search_attach_and_degrade_gracefully(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/gifs/search?q=high%20five")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let search: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    if search["degraded"].as_bool().unwrap() {
        assert_eq!(search["results"].as_array().unwrap().len(), 0);
    } else {
        assert_eq!(search["results"].as_array().unwrap().len(), 8);
        assert!(matches!(
            search["results"][0]["media_type"].as_str(),
            Some("image" | "video")
        ));
        assert!(
            search["results"][0]["url"]
                .as_str()
                .unwrap()
                .starts_with("http")
        );
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/gifs/search?q=high%20five&page=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let page_two: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    if page_two["degraded"].as_bool().unwrap() {
        assert_eq!(page_two["results"].as_array().unwrap().len(), 0);
    } else {
        assert_ne!(page_two["results"][0]["url"], search["results"][0]["url"]);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/gifs/search?q=confused&page=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let other_query: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    if other_query["degraded"].as_bool().unwrap() {
        assert_eq!(other_query["results"].as_array().unwrap().len(), 0);
    } else {
        assert_ne!(
            other_query["results"][0]["url"],
            search["results"][0]["url"]
        );
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/gifs/search?q=fail")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let failed_search: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(failed_search["degraded"], true);
    assert_eq!(failed_search["results"].as_array().unwrap().len(), 0);

    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header(HEADER_USER_NAME, "Ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"GIF API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/cards"))
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"column_id":"{column_id}","gif_url":"https://media.giphy.com/media/111ebonMs90YLu/giphy.gif","gif_alt_text":"high five"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let gif_card: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        gif_card["gif_url"],
        "https://media.giphy.com/media/111ebonMs90YLu/giphy.gif"
    );
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn voting_endpoints_track_remaining_votes_and_limits(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Voting API retro","template":"standard","vote_limit":3,"action_discussion_limit":3,"invitees":[{"email":"lee@example.com","role":"member"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"vote here"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let card: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ready"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    for path in ["reveal", "voting/start"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/votes"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"card_id":"{card_id}","count":2}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let voting: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(voting["votes_remaining"], 1);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/votes"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"card_id":"{card_id}","count":2}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ready"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["ready"]["current_user_ready"], true);
    assert_eq!(board["voting"]["votes_remaining"], 1);
    assert_eq!(board["columns"][0]["cards"][0]["vote_count"], 2);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/actions/start"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let action_board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(action_board["retro"]["phase"], "action_discussion");
    let action_id = action_board["actions"][0]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/retros/{retro_id}/actions/{action_id}/confirm"
                ))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/complete"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let completed_board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(completed_board["retro"]["phase"], "completed");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn ingestion_endpoints_support_deck_and_direct_draft_modes(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Ingestion API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let first_column_id = created["columns"][0]["id"].as_str().unwrap();
    let second_column_id = created["columns"][1]["id"].as_str().unwrap();

    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ingest"))
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"source":"pi","placement":"user_deck","suggested_text":"Deck idea","idempotency_key":"event-1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let deck_item: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let deck_item_id = deck_item["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/deck/{deck_item_id}/accept"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{first_column_id}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/ingest"))
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"source":"claude_code","placement":"retro_draft","target_column_id":"{second_column_id}","suggested_text":"Direct idea","idempotency_key":"event-2"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["deck"].as_array().unwrap().len(), 0);
    assert_eq!(board["columns"][0]["cards"][0]["body_text"], "Deck idea");
    assert_eq!(board["columns"][1]["cards"][0]["body_text"], "Direct idea");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn ai_job_endpoints_persist_reviewable_outputs_and_retry_failure(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"AI API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"summary"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let summary: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(summary["status"], "succeeded");
    assert_eq!(summary["output"]["review_required"], true);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"mood","fail":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let failed: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(failed["status"], "failed");
    let artifact_id = failed["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/retros/{retro_id}/ai-jobs/{artifact_id}/retry"
                ))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let retried: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(retried["status"], "succeeded");
    assert_eq!(retried["retry_count"], 1);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["ai_artifacts"].as_array().unwrap().len(), 2);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn meeting_notes_feed_summary_and_mood_ai_context_without_blocking_completion(
    pool: sqlx::PgPool,
) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Notes API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"summary"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let no_notes_ai: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(no_notes_ai["input"]["meeting_notes_included"], false);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/meeting-notes"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Retro notes","body_text":"Release ownership was unclear."}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"mood"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let mood_ai: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(mood_ai["input"]["meeting_notes_included"], true);
    assert_eq!(
        mood_ai["input"]["meeting_notes"][0]["body_text"],
        "Release ownership was unclear."
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["meeting_notes"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn delivery_endpoints_export_summary_and_retry_failure(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Delivery API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/deliveries"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"summary_export","fail":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let failed: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["output"]["title"], "Delivery API retro");
    let delivery_id = failed["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/retros/{retro_id}/deliveries/{delivery_id}/retry"
                ))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let retried: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(retried["status"], "succeeded");
    assert_eq!(retried["retry_count"], 1);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["deliveries"].as_array().unwrap().len(), 1);
}

// Feature 1 — uninviting a member removes their participant row immediately.
//
// Bob's subject is the `email:sha256(email)` value that the frontend derives
// from "bob@spill.test". We set that directly in the subject header so the
// participant row uses the same key that `remove_participant_by_email` targets.
const ALICE_EMAIL: &str = "alice@spill.test";
const ALICE_SUBJECT: &str =
    "email:911c1a4b6e2a0f21e6b2176e7b1ee2e9d8c713b47fada7c98a565f26f93f2122";
const BOB_EMAIL: &str = "bob@spill.test";
const BOB_SUBJECT: &str = "email:5e4ed42deade990aad0ac79434b6615d3c2dcf0ec6fb898a0e002a6206fe1396";

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn uninvite_removes_participant_row(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));

    // Alice creates the retro (she's host).
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header(HEADER_USER_NAME, "Alice")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Uninvite test","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    // Alice grants Bob a member slot.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/grants"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"email":"{BOB_EMAIL}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Bob joins — this creates his participant row with BOB_SUBJECT.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board_with_bob: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board_with_bob["participants"].as_array().unwrap().len(), 2);

    // Alice revokes Bob's grant — must also evict him from participants.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/grants/remove"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"email":"{BOB_EMAIL}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Alice re-opens the board — only she should appear as a participant.
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header(HEADER_USER_NAME, "Alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board_after: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        board_after["participants"].as_array().unwrap().len(),
        1,
        "Bob's participant row should have been removed on uninvite"
    );
    assert_eq!(board_after["participants"][0]["display_name"], "Alice");
}

// Feature 2 — host can kick any participant, participant can self-leave;
// host cannot kick themselves, non-host cannot kick others.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn participant_removal_enforces_access_rules(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header(HEADER_USER_NAME, "Alice")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Kick test","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    // Alice grants Bob access (a participant must be a board member).
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/grants"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"email":"{BOB_EMAIL}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();

    // Bob joins as a participant.
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Bob cannot kick Alice (non-host kicking others → 403).
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/retros/{retro_id}/participants/{ALICE_SUBJECT}"
                ))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Alice (host) cannot kick herself → 400.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/retros/{retro_id}/participants/{ALICE_SUBJECT}"
                ))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Bob self-leaves → 204.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/retros/{retro_id}/participants/{BOB_SUBJECT}"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Second attempt on same subject → 404.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/retros/{retro_id}/participants/{BOB_SUBJECT}"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Alice (host) can kick any remaining participant — Bob was removed, so
    // this verifies that the ready-count event is published and the board
    // can still be loaded.
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["participants"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn sitting_out_participants_cannot_create_cards_or_vote(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header(HEADER_USER_NAME, "Alice")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Participation actions","template":"standard","vote_limit":3,"action_discussion_limit":3,"invitees":[{"email":"bob@spill.test","role":"member"}]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bob_board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let bob_participant_id = bob_board["participants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|participant| participant["display_name"] == "Bob")
        .and_then(|participant| participant["id"].as_str())
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/api/retros/{retro_id}/participants/{bob_participant_id}/participation"
                ))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"is_participating":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"should not land"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header(HEADER_USER_NAME, "Alice")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"vote target"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let card: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let card_id = card["id"].as_str().unwrap();

    for path in ["ready", "reveal", "voting/start"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/votes"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"card_id":"{card_id}","count":1}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/api/retros/{retro_id}/participants/{bob_participant_id}/participation"
                ))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"is_participating":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/votes"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"card_id":"{card_id}","count":1}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/api/retros/{retro_id}/participants/{bob_participant_id}/participation"
                ))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"is_participating":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/retros/{retro_id}/votes/{card_id}"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Feature 3 — host can force-reveal even when not all participants are ready.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn force_reveal_skips_ready_gate_for_host(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header(HEADER_USER_NAME, "Alice")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Force reveal test","template":"standard","vote_limit":3,"action_discussion_limit":3,"invitees":[{"email":"bob@spill.test","role":"member"}]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    // Bob joins and writes a card but does not mark ready.
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header(HEADER_USER_NAME, "Bob")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"Bob's unready card"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    // Normal reveal without force → 400 (not everyone ready).
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"force":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Non-host force-reveal → 403.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, BOB_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"force":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Host force-reveal → 200, board advances to discussion.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, ALICE_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"force":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["retro"]["phase"], "discussion");
}

// ---------------------------------------------------------------------------
// AI provider auto-summary tests
// ---------------------------------------------------------------------------
//
// These exercise the spawn-and-forget summary runner end-to-end:
//   - complete the retro
//   - poll the board until the artifact lands on the expected status
//   - assert the persisted output / error
// A `FakeProvider` is injected via `app_with_repository_and_ai`, so no
// real HTTP is involved and no env vars are read.

use std::sync::Arc;
use std::time::Duration;

use crate::ai_provider::{AiProvider, FakeProvider};

const AUTHOR: &str = "ava@spill.test";
// Derived participant subject for AUTHOR (email:sha256(AUTHOR)).
const AUTHOR_SUBJECT: &str =
    "email:028a444c15f2f00008e2f5936831baecd1ef125bcedca7ba3a1ea7c2b61b4bc6";

/// Create a retro and march it through ready → reveal → voting →
/// actions/start so it is ready to be completed. Returns retro_id.
/// One participant (`AUTHOR`), one card — enough for the prompt
/// builder to produce something non-empty.
async fn seed_completable_retro(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Retro under test","template":"standard"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap().to_owned();
    let column_id = created["columns"][0]["id"].as_str().unwrap().to_owned();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"shipping cadence felt steady"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    // Card creation returns 201 Created.
    assert_eq!(response.status(), StatusCode::CREATED);

    for path in ["ready", "reveal", "voting/start", "actions/start"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, AUTHOR)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "phase transition {path}");
    }

    retro_id
}

async fn post_complete(app: &axum::Router, retro_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/complete"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn fetch_board(app: &axum::Router, retro_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

/// Poll the board for a summary artifact in the expected terminal
/// status. Returns the artifact JSON. Bounded at ~2s — generous for
/// `FakeProvider` (no IO) but small enough not to hide regressions.
async fn wait_for_summary_status(
    app: &axum::Router,
    retro_id: &str,
    expected_status: &str,
) -> Value {
    for _ in 0..200 {
        let board = fetch_board(app, retro_id).await;
        if let Some(artifact) = board["ai_artifacts"]
            .as_array()
            .and_then(|artifacts| artifacts.iter().find(|a| a["kind"] == "summary"))
        {
            if artifact["status"] == expected_status {
                return artifact.clone();
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for summary artifact to reach status {expected_status}");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn complete_retro_without_ai_provider_skips_summary_artifact(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), None);
    let retro_id = seed_completable_retro(&app).await;

    let board = post_complete(&app, &retro_id).await;
    assert_eq!(board["retro"]["phase"], "completed");

    // No provider → no auto-trigger; the artifact array stays empty.
    let board = fetch_board(&app, &retro_id).await;
    let artifacts = board["ai_artifacts"].as_array().unwrap();
    assert!(
        artifacts.iter().all(|a| a["kind"] != "summary"),
        "expected no summary artifact, got {artifacts:?}",
    );
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn only_host_can_complete_retro(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), None);
    let retro_id = seed_completable_retro(&app).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/complete"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn completing_retro_returns_planned_next_retro(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), None);
    let retro_id = seed_completable_retro(&app).await;
    let completed = post_complete(&app, &retro_id).await;

    assert_eq!(completed["retro"]["phase"], "completed");
    assert_eq!(completed["next_retro"]["phase"], "scheduled");
    assert_eq!(completed["next_retro"]["title"], "Next: Retro under test");
    assert_eq!(completed["series"]["name"], "Retro under test");

    let fetched = fetch_board(&app, &retro_id).await;
    assert_eq!(fetched["next_retro"]["id"], completed["next_retro"]["id"]);
}

async fn wait_for_next_retro_title(app: &axum::Router, retro_id: &str, expected_title: &str) {
    for _ in 0..200 {
        let board = fetch_board(app, retro_id).await;
        if board["next_retro"]["title"] == expected_title {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for next retro title {expected_title}");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn ai_provider_suggests_next_retro_title_after_completion(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        "\"Platform pulse check\"",
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(provider));
    let retro_id = seed_completable_retro(&app).await;
    let completed = post_complete(&app, &retro_id).await;

    assert_eq!(completed["next_retro"]["title"], "Generating title...");
    let stored = fetch_board(&app, &retro_id).await;
    assert_ne!(stored["next_retro"]["title"], "Generating title...");
    wait_for_next_retro_title(&app, &retro_id, "Platform pulse check").await;
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn failed_ai_next_title_keeps_deterministic_title(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::failing_with(
        "title service unavailable",
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(provider));
    let retro_id = seed_completable_retro(&app).await;
    let completed = post_complete(&app, &retro_id).await;

    assert_eq!(completed["next_retro"]["title"], "Generating title...");
    wait_for_next_retro_title(&app, &retro_id, "Next: Retro under test").await;
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn ai_next_title_does_not_overwrite_manual_edit(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        "AI-picked title",
    )));
    let repo = retro_db::RetroRepository::new(pool);
    let app = app_with_repository_and_ai(repo.clone(), Some(provider));
    let retro_id = seed_completable_retro(&app).await;
    let completed = post_complete(&app, &retro_id).await;
    assert_eq!(completed["next_retro"]["title"], "Generating title...");

    let next_id = completed["next_retro"]["id"].as_str().unwrap();
    repo.update_retro_details(retro_db::UpdateRetroDetailsInput {
        retro_id: Uuid::parse_str(next_id).unwrap(),
        title: Some("Manual title".to_owned()),
        group_name: None,
        cover_gif_url: None,
        cover_gif_alt_text: None,
        remove_cover_gif: false,
        vote_limit: None,
        action_discussion_limit: None,
        clustering_mode: None,
        card_edit_policy: None,
        anonymous_authors: None,
        reveal_mode: None,
    })
    .await
    .unwrap();

    wait_for_next_retro_title(&app, &retro_id, "Manual title").await;
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_can_update_retro_title_and_group(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Old retro","template":"standard","group_name":"Old group"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"New retro","group_name":"New group"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(updated["retro"]["title"], "New retro");
    assert_eq!(updated["series"]["name"], "New group");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_can_update_board_configs(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Config retro","template":"standard","planned_for":"2099-05-15","vote_limit":3,"action_discussion_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"vote_limit":8,"action_discussion_limit":0,"clustering_mode":"auto_on_vote_start"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(updated["retro"]["vote_limit"], 8);
    assert_eq!(updated["retro"]["action_discussion_limit"], 0);
    assert_eq!(updated["retro"]["clustering_mode"], "auto_on_vote_start");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_can_update_board_configs_after_scheduled_phase(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Live config retro","template":"standard","vote_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(created["retro"]["phase"], "writing");
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"vote_limit":8}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(updated["retro"]["vote_limit"], 8);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_cannot_disable_voting_after_voting_started(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Voting config retro","template":"standard","vote_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    for path in ["ready", "reveal", "voting/start"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, "host@example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "phase transition {path}");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"vote_limit":0}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_cannot_update_transition_config_after_wrap_up_started(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Action config retro","template":"standard","action_discussion_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    for path in ["ready", "reveal", "actions/start"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, "host@example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "phase transition {path}");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"action_discussion_limit":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn enabling_actions_without_actions_column_is_rejected(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"No actions retro","template":"standard","action_discussion_limit":0}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"action_discussion_limit":3}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_can_create_update_and_list_retro_cover_gif(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Covered retro","template":"standard","cover_gif_url":"https://media.example/coffee.gif","cover_gif_alt_text":"coffee spill"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    assert_eq!(
        created["retro"]["cover_gif_url"],
        "https://media.example/coffee.gif"
    );
    assert_eq!(created["retro"]["cover_gif_alt_text"], "coffee spill");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"cover_gif_url":"https://media.example/rocket.gif","cover_gif_alt_text":"tiny rocket"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        updated["retro"]["cover_gif_url"],
        "https://media.example/rocket.gif"
    );
    assert_eq!(updated["retro"]["cover_gif_alt_text"], "tiny rocket");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let overview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        overview["active"][0]["cover_gif_url"],
        "https://media.example/rocket.gif"
    );
    assert_eq!(overview["active"][0]["cover_gif_alt_text"], "tiny rocket");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn real_provider_summary_job_before_completion_fails_without_running(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        "should not be generated",
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(provider));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Premature summary retro","template":"standard"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/ai-jobs"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"summary"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let artifact: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();

    assert_eq!(artifact["status"], "failed");
    assert!(artifact["output"].is_null());
    assert!(
        artifact["error_message"]
            .as_str()
            .unwrap_or("")
            .contains("after retro completion"),
        "expected completion gate message, got {:?}",
        artifact["error_message"],
    );
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn complete_retro_with_fake_provider_persists_succeeded_summary(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        "stub summary text from fake provider",
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(provider));
    let retro_id = seed_completable_retro(&app).await;
    post_complete(&app, &retro_id).await;

    let artifact = wait_for_summary_status(&app, &retro_id, "succeeded").await;
    assert_eq!(artifact["kind"], "summary");
    assert_eq!(artifact["output"]["review_required"], false);
    assert_eq!(
        artifact["output"]["summary"],
        "stub summary text from fake provider"
    );
    assert!(artifact["error_message"].is_null());

    // The runner uses `fetch_board_readonly` and must not insert a
    // synthetic participant. The participant list should only contain
    // the human author.
    let board = fetch_board(&app, &retro_id).await;
    let subjects: Vec<&str> = board["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["external_subject"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        subjects,
        vec![AUTHOR_SUBJECT],
        "runner must not appear as a participant"
    );
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn failing_provider_records_failed_artifact_then_retry_recovers(pool: sqlx::PgPool) {
    let failing = Arc::new(AiProvider::Fake(FakeProvider::failing_with(
        "upstream is unavailable",
    )));
    let app =
        app_with_repository_and_ai(retro_db::RetroRepository::new(pool.clone()), Some(failing));
    let retro_id = seed_completable_retro(&app).await;
    post_complete(&app, &retro_id).await;

    let failed = wait_for_summary_status(&app, &retro_id, "failed").await;
    assert_eq!(failed["status"], "failed");
    assert!(
        failed["error_message"]
            .as_str()
            .unwrap_or("")
            .contains("AI provider"),
        "expected user-facing error message, got {:?}",
        failed["error_message"],
    );
    let artifact_id = failed["id"].as_str().unwrap().to_owned();

    // Now swap in a healthy provider and retry through the existing
    // /ai-jobs/{id}/retry endpoint. We rebuild the app against the
    // same pool so the retry uses the now-healthy provider while the
    // failed artifact persists across the swap.
    let healthy = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        "recovered summary",
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(healthy));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/retros/{retro_id}/ai-jobs/{artifact_id}/retry"
                ))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let recovered = wait_for_summary_status(&app, &retro_id, "succeeded").await;
    assert_eq!(recovered["output"]["summary"], "recovered summary");
    assert!(
        recovered["retry_count"].as_i64().unwrap() >= 1,
        "retry_count should advance on retry",
    );
}

/// Poll the board until `clustering_status` reaches the expected value. Bounded
/// at ~3s — generous for `FakeProvider` (no IO) but small enough to surface
/// regressions.
async fn wait_for_clustering_status(app: &axum::Router, retro_id: &str, expected: &str) -> Value {
    for _ in 0..300 {
        let board = fetch_board(app, retro_id).await;
        if board["retro"]["clustering_status"] == expected {
            return board;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for clustering_status to reach {expected}");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn auto_clustering_computes_on_reveal_and_applies_on_voting(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        r#"{"groups":[{"title":"Deploy noise","summary":"deploy alerts","card_ids":["11111111-1111-1111-1111-111111111111"],"category":"delivery","tags":["delivery"]}]}"#,
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(provider));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Cluster wiring","template":"standard","clustering_mode":"auto_on_vote_start"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap().to_owned();
    let column_id = created["columns"][1]["id"].as_str().unwrap().to_owned();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"deploy alerts are noisy"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    for path in ["ready", "reveal"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, AUTHOR)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "phase transition {path}");
    }

    // Compute runs on reveal and produces a proposal without mutating the board.
    let board = wait_for_clustering_status(&app, &retro_id, "ready").await;
    assert_eq!(board["retro"]["phase"], "discussion");
    assert_eq!(board["clusters"].as_array().unwrap().len(), 0);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/voting/start"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Moving to voting auto-applies the ready proposal.
    let board = wait_for_clustering_status(&app, &retro_id, "applied").await;
    assert_eq!(board["retro"]["phase"], "voting");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn per_column_auto_clustering_waits_until_all_columns_revealed(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        r#"{"groups":[{"title":"Deploy noise","summary":"deploy alerts","card_ids":["11111111-1111-1111-1111-111111111111"],"category":"delivery","tags":["delivery"]}]}"#,
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(provider));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Per-column cluster","template":"standard","clustering_mode":"auto_on_vote_start","reveal_mode":"per_column"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap().to_owned();
    let column_ids: Vec<String> = created["columns"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|column| column["id"].as_str().map(str::to_owned))
        .collect();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{}","body_text":"deploy alerts are noisy"}}"#,
                    column_ids[1]
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    for path in ["ready", "reveal"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, AUTHOR)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "phase transition {path}");
    }

    let board = fetch_board(&app, &retro_id).await;
    assert_eq!(board["retro"]["phase"], "discussion");
    assert_eq!(board["retro"]["clustering_status"], "not_run");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/retros/{retro_id}/columns/{}/reveal",
                    column_ids[0]
                ))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board = fetch_board(&app, &retro_id).await;
    assert_eq!(board["retro"]["clustering_status"], "not_run");

    for column_id in column_ids.iter().skip(1) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/columns/{column_id}/reveal"))
                    .header(HEADER_ON_BEHALF_OF, AUTHOR)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    wait_for_clustering_status(&app, &retro_id, "ready").await;
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn enabling_auto_clustering_during_voting_applies(pool: sqlx::PgPool) {
    let provider = Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        r#"{"groups":[{"title":"Deploy noise","summary":"deploy alerts","card_ids":["11111111-1111-1111-1111-111111111111"],"category":"delivery","tags":["delivery"]}]}"#,
    )));
    let app = app_with_repository_and_ai(retro_db::RetroRepository::new(pool), Some(provider));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Late cluster retro","template":"standard","clustering_mode":"disabled"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap().to_owned();
    let column_id = created["columns"][1]["id"].as_str().unwrap().to_owned();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"deploy alerts are noisy"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    for path in ["ready", "reveal", "voting/start"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, AUTHOR)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "phase transition {path}");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"clustering_mode":"auto_on_vote_start"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    wait_for_clustering_status(&app, &retro_id, "applied").await;
}

const HOST_EMAIL: &str = "ava@spill.test";

/// Create an auto-clustering retro hosted by `AUTHOR`, add a card, march it to a
/// `ready` clustering proposal. Returns retro_id.
async fn seed_ready_clustering_retro(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Cluster endpoints","template":"standard","clustering_mode":"auto_on_vote_start"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap().to_owned();
    let column_id = created["columns"][1]["id"].as_str().unwrap().to_owned();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cards"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"column_id":"{column_id}","body_text":"deploy alerts are noisy"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    for path in ["ready", "reveal"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/{path}"))
                    .header(HEADER_ON_BEHALF_OF, AUTHOR)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "phase transition {path}");
    }
    wait_for_clustering_status(app, &retro_id, "ready").await;
    retro_id
}

fn cluster_provider() -> Arc<AiProvider> {
    Arc::new(AiProvider::Fake(FakeProvider::responding_with(
        r#"{"groups":[{"title":"Deploy noise","summary":"deploy alerts","card_ids":["11111111-1111-1111-1111-111111111111"],"category":"delivery","tags":["delivery"]}]}"#,
    )))
}

async fn post_cluster_action(
    app: &axum::Router,
    retro_id: &str,
    action: &str,
    email: &str,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/cluster/{action}"))
                .header(HEADER_ON_BEHALF_OF, email)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn apply_clustering_requires_host_and_is_idempotent(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(
        retro_db::RetroRepository::new(pool),
        Some(cluster_provider()),
    );
    let retro_id = seed_ready_clustering_retro(&app).await;

    // Non-host cannot apply.
    let denied = post_cluster_action(&app, &retro_id, "apply", BOB_EMAIL).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    // Host applies the ready proposal.
    let response = post_cluster_action(&app, &retro_id, "apply", HOST_EMAIL).await;
    assert_eq!(response.status(), StatusCode::OK);
    let first: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(first["retro"]["clustering_status"], "applied");
    let clusters_after_first = first["clusters"].as_array().unwrap().len();
    assert!(clusters_after_first >= 1);

    // Re-applying is a no-op: no duplicate clusters, still applied.
    let response = post_cluster_action(&app, &retro_id, "apply", HOST_EMAIL).await;
    assert_eq!(response.status(), StatusCode::OK);
    let second: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(second["retro"]["clustering_status"], "applied");
    assert_eq!(
        second["clusters"].as_array().unwrap().len(),
        clusters_after_first
    );
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn saving_unchanged_auto_clustering_mode_preserves_applied_status(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(
        retro_db::RetroRepository::new(pool),
        Some(cluster_provider()),
    );
    let retro_id = seed_ready_clustering_retro(&app).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/voting/start"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    wait_for_clustering_status(&app, &retro_id, "applied").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, HOST_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"vote_limit":4,"clustering_mode":"auto_on_vote_start"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["retro"]["clustering_status"], "applied");
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn apply_clustering_rejected_after_action_discussion(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(
        retro_db::RetroRepository::new(pool.clone()),
        Some(cluster_provider()),
    );
    let retro_id = seed_ready_clustering_retro(&app).await;

    // Wrap-up has generated actions from the current cards; a stale apply now
    // must be rejected rather than reorganizing them.
    sqlx::query("UPDATE retros SET phase = 'action_discussion' WHERE id = $1")
        .bind(Uuid::parse_str(&retro_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let response = post_cluster_action(&app, &retro_id, "apply", HOST_EMAIL).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retry_clustering_requires_host_and_recomputes(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(
        retro_db::RetroRepository::new(pool.clone()),
        Some(cluster_provider()),
    );
    let retro_id = seed_ready_clustering_retro(&app).await;

    // Simulate a prior compute failure.
    sqlx::query("UPDATE retros SET clustering_status = 'failed' WHERE id = $1")
        .bind(Uuid::parse_str(&retro_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // Non-host cannot retry.
    let denied = post_cluster_action(&app, &retro_id, "retry", BOB_EMAIL).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    // Host retry recomputes a fresh proposal.
    let response = post_cluster_action(&app, &retro_id, "retry", HOST_EMAIL).await;
    assert_eq!(response.status(), StatusCode::OK);
    wait_for_clustering_status(&app, &retro_id, "ready").await;
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retry_clustering_during_voting_auto_applies(pool: sqlx::PgPool) {
    let app = app_with_repository_and_ai(
        retro_db::RetroRepository::new(pool.clone()),
        Some(cluster_provider()),
    );
    let retro_id = seed_ready_clustering_retro(&app).await;

    // Move to voting (auto-applies the ready proposal).
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/voting/start"))
                .header(HEADER_ON_BEHALF_OF, AUTHOR)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    wait_for_clustering_status(&app, &retro_id, "applied").await;

    // Simulate a clustering failure during voting.
    sqlx::query("UPDATE retros SET clustering_status = 'failed' WHERE id = $1")
        .bind(Uuid::parse_str(&retro_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // A voting-phase retry must recompute AND apply (not leave a ready proposal
    // unapplied), since apply is automatic from voting onward.
    let response = post_cluster_action(&app, &retro_id, "retry", HOST_EMAIL).await;
    assert_eq!(response.status(), StatusCode::OK);
    wait_for_clustering_status(&app, &retro_id, "applied").await;
}

// Create-time toggles for the two privacy fields. The fields default to the
// SQL defaults (collaborative / false) when omitted; explicit values land on
// the freshly-created retro without needing a follow-up PATCH.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retro_create_persists_privacy_toggles(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Privacy retro","template":"standard","vote_limit":3,"action_discussion_limit":3,"card_edit_policy":"author_only","anonymous_authors":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    // Refetch — the create response is built before the post-create updates
    // apply (matches the existing clustering pattern), so the source of truth
    // is the fresh fetch the client redirects to.
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["retro"]["card_edit_policy"], "author_only");
    assert_eq!(board["retro"]["anonymous_authors"], true);
}

// Per-board reveal_mode flows from the create form -> POST /retros body ->
// retros.reveal_mode column. The CLI/API can pick either mode; UI gates the
// affordances per mode.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retro_create_persists_reveal_mode_when_explicit(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Big-bang retro","template":"standard","vote_limit":3,"action_discussion_limit":3,"reveal_mode":"big_bang"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(created["retro"]["reveal_mode"], "big_bang");
}

// Default when reveal_mode is omitted from the create body is 'big_bang'
// (matches the SQL DEFAULT and historic non-form caller behavior). The web
// create form always submits an explicit value, so users opting through the
// UI get the per_column default from the form-side checkbox.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retro_create_defaults_reveal_mode_to_big_bang(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Default reveal mode","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(created["retro"]["reveal_mode"], "big_bang");
}

// Unknown reveal_mode values are rejected with a clear 4xx before the SQL
// CHECK fires -- workflow::validate_reveal_mode mirrors the
// validate_card_edit_policy pattern.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retro_create_rejects_unknown_reveal_mode(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Bad reveal mode","template":"standard","vote_limit":3,"action_discussion_limit":3,"reveal_mode":"telegram"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// PATCH /retros/{id} can toggle reveal_mode on an existing board, e.g. via
// the board settings dialog if the host changes their mind mid-flight.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn update_retro_details_can_toggle_reveal_mode(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header(HEADER_USER_NAME, "Host")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Toggle reveal","template":"standard","vote_limit":3,"action_discussion_limit":3,"reveal_mode":"per_column"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reveal_mode":"big_bang"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(board["retro"]["reveal_mode"], "big_bang");
    let column_id = board["columns"][0]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/columns/{column_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"force":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Per-column reveal route: host-gated, ready-gated (or force=true bypass),
// flips that column's drafts visible to other participants. Mirrors the
// existing `/reveal` integration test patterns but exercises the new
// `/columns/{column_id}/reveal` endpoint and the row-level visibility flip.
#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn reveal_column_route_is_host_only_and_discussion_only(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header(HEADER_USER_NAME, "Ava")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Per-column reveal","template":"standard","vote_limit":3,"action_discussion_limit":3,"reveal_mode":"per_column","invitees":[{"email":"lee@example.com","role":"member"}]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    // Lee joins on first read.
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .header(HEADER_USER_NAME, "Lee")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    for (subject, body) in [
        ("ava@example.com", "Ava draft"),
        ("lee@example.com", "Lee draft"),
    ] {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/cards"))
                    .header(HEADER_ON_BEHALF_OF, subject)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"column_id":"{column_id}","body_text":"{body}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Reveal during writing -> 4xx (column reveal is a discussion-phase action).
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/columns/{column_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Advance to discussion via reveal_board. Per_column mode -> phase moves
    // but no columns get stamped (host reveals each one individually).
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"force":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Non-host can't reveal a column.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/columns/{column_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Host reveals the column. No body needed -- no ready gate / force flag.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/columns/{column_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let revealed: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    // Phase stays 'discussion' -- reveal_column never advances phases.
    assert_eq!(revealed["retro"]["phase"], "discussion");
    let col = revealed["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == column_id)
        .unwrap();
    assert!(col["revealed_at"].is_string());

    // Lee now sees both authors' previously-hidden drafts.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_ON_BEHALF_OF, "lee@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let lee_board: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let bodies: Vec<&str> = lee_board["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == column_id)
        .unwrap()["cards"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["body_text"].as_str())
        .collect();
    assert!(bodies.contains(&"Ava draft"));
    assert!(bodies.contains(&"Lee draft"));

    // Re-reveal is a quiet 204 -- the host double-clicked, no need to
    // surface an error. No board body, no second event publish.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/columns/{column_id}/reveal"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // A typoed (unknown-on-this-retro) column ID still 404s -- the split
    // outcome means quiet success is reserved for legitimate double-clicks.
    let bogus_column_id = uuid::Uuid::from_u128(0xdeadbeef);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/retros/{retro_id}/columns/{bogus_column_id}/reveal"
                ))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}/details"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reveal_mode":"big_bang"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Starting voting reveals any skipped per-column columns instead of failing
    // the host's normal phase transition.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{retro_id}/voting/start"))
                .header(HEADER_ON_BEHALF_OF, "ava@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let voting: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(voting["retro"]["phase"], "voting");
    assert!(
        voting["columns"]
            .as_array()
            .unwrap()
            .iter()
            .all(|column| column["revealed_at"].is_string())
    );
}
