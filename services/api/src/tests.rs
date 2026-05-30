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
                .header(HEADER_USER_SUBJECT, "user-123")
                .header(HEADER_USER_NAME, "Ava")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();

    assert_eq!(body["user"]["subject"], "user-123");
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

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn retro_endpoints_create_list_and_open_standard_board(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/retros")
                    .header(HEADER_USER_SUBJECT, "user-123")
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
                .header(HEADER_USER_SUBJECT, "user-123")
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
                .uri("/api/retros")
                .header(HEADER_USER_SUBJECT, "user-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let overview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(overview["retros"].as_array().unwrap().len(), 1);
    assert_eq!(overview["retros"][0]["title"], "Sprint 43");
    assert_eq!(overview["retros"][0]["phase"], "writing");
    assert_eq!(overview["retros"][0]["completed_at"], Value::Null);
    assert_eq!(overview["active"].as_array().unwrap().len(), 1);
    assert_eq!(overview["completed"].as_array().unwrap().len(), 0);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/retros")
                .header(HEADER_USER_SUBJECT, "uninvited-user")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let other_overview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(other_overview["retros"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_can_update_retro_title_and_schedule(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_USER_SUBJECT, "host-123")
                .header(HEADER_USER_NAME, "Host")
                .header(HEADER_USER_EMAIL, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Original retro","template":"standard","scheduled_at":"2026-06-05T10:00:00Z"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    assert_eq!(created["retro"]["scheduled_at"], "2026-06-05T10:00:00Z");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_USER_SUBJECT, "host-123")
                .header(HEADER_USER_EMAIL, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Renamed retro","scheduled_at":"2026-06-12T10:00:00Z"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(updated["retro"]["title"], "Renamed retro");
    assert_eq!(updated["retro"]["scheduled_at"], "2026-06-12T10:00:00Z");

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_USER_SUBJECT, "member-123")
                .header(HEADER_USER_EMAIL, "member@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Member rename","scheduled_at":"2026-06-19T10:00:00Z"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "retro_db::MIGRATOR")]
async fn host_can_create_next_retro_from_existing_retro(pool: sqlx::PgPool) {
    let app = app_with_repository(retro_db::RetroRepository::new(pool));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/retros")
                .header(HEADER_USER_SUBJECT, "host-123")
                .header(HEADER_USER_NAME, "Host")
                .header(HEADER_USER_EMAIL, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(
                    r##"{"title":"Sprint 43","template":"custom","columns":["Wins","Pains"],"column_colors":["#2f9469","#cf4f4f"],"vote_limit":5,"action_discussion_limit":2,"scheduled_at":"2026-06-05T10:00:00Z","invitees":[{"email":"member@example.com","role":"member"}]}"##,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let source_id = created["retro"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/retros/{source_id}/clone"))
                .header(HEADER_USER_SUBJECT, "host-123")
                .header(HEADER_USER_NAME, "Host")
                .header(HEADER_USER_EMAIL, "host@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Sprint 44"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let cloned: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let cloned_id = cloned["retro"]["id"].as_str().unwrap();
    assert_ne!(cloned_id, source_id);
    assert_eq!(cloned["retro"]["title"], "Sprint 44");
    assert_eq!(cloned["retro"]["vote_limit"], 5);
    assert_eq!(cloned["retro"]["action_discussion_limit"], 2);
    assert_eq!(cloned["retro"]["scheduled_at"], "2026-06-19T10:00:00Z");
    assert_eq!(
        cloned["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Wins", "Pains", "Actions"]
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{cloned_id}"))
                .header(HEADER_USER_SUBJECT, "member-123")
                .header(HEADER_USER_EMAIL, "member@example.com")
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
                .uri(format!("/api/retros/{source_id}/clone"))
                .header(HEADER_USER_SUBJECT, "member-123")
                .header(HEADER_USER_EMAIL, "member@example.com")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Nope"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
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
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header(HEADER_USER_NAME, "Ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Writing API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let retro_id = created["retro"]["id"].as_str().unwrap();
    let column_id = created["columns"][0]["id"].as_str().unwrap();

    for (subject, body) in [("ava", "Ava draft"), ("lee", "Lee private draft")] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/retros/{retro_id}/cards"))
                    .header(HEADER_USER_SUBJECT, subject)
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "lee")
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
                .header(HEADER_USER_SUBJECT, "ava")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let revealed: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(revealed["retro"]["phase"], "discussion");
    assert_eq!(
        revealed["columns"][0]["cards"][1]["body_text"],
        "Lee private draft"
    );
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Voting API retro","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "lee")
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
                .header(HEADER_USER_SUBJECT, "lee")
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
                .header(HEADER_USER_SUBJECT, "lee")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                    .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, "ava")
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_NAME, "Alice")
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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
                .header(HEADER_USER_SUBJECT, BOB_SUBJECT)
                .header(HEADER_USER_NAME, "Bob")
                .header(HEADER_USER_EMAIL, BOB_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_NAME, "Alice")
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_NAME, "Alice")
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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

    // Bob joins as a participant (no grant needed for this test).
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/retros/{retro_id}"))
                .header(HEADER_USER_SUBJECT, BOB_SUBJECT)
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
                .header(HEADER_USER_SUBJECT, BOB_SUBJECT)
                .header(HEADER_USER_EMAIL, BOB_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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
                .header(HEADER_USER_SUBJECT, BOB_SUBJECT)
                .header(HEADER_USER_EMAIL, BOB_EMAIL)
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
                .header(HEADER_USER_SUBJECT, BOB_SUBJECT)
                .header(HEADER_USER_EMAIL, BOB_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_NAME, "Alice")
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Force reveal test","template":"standard","vote_limit":3,"action_discussion_limit":3}"#,
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
                .header(HEADER_USER_SUBJECT, BOB_SUBJECT)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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
                .header(HEADER_USER_SUBJECT, BOB_SUBJECT)
                .header(HEADER_USER_EMAIL, BOB_EMAIL)
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
                .header(HEADER_USER_SUBJECT, ALICE_SUBJECT)
                .header(HEADER_USER_EMAIL, ALICE_EMAIL)
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

const AUTHOR: &str = "ava";

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
                .header(HEADER_USER_SUBJECT, AUTHOR)
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
                .header(HEADER_USER_SUBJECT, AUTHOR)
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
                    .header(HEADER_USER_SUBJECT, AUTHOR)
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
                .header(HEADER_USER_SUBJECT, AUTHOR)
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
                .header(HEADER_USER_SUBJECT, AUTHOR)
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
                .header(HEADER_USER_SUBJECT, AUTHOR)
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
                .header(HEADER_USER_SUBJECT, AUTHOR)
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
        vec![AUTHOR],
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
                .header(HEADER_USER_SUBJECT, AUTHOR)
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
