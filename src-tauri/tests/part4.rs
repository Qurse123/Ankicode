use ankicode_lib::{
    commands::{AppInner, AppState},
    integration::{build_router, LOOPBACK_ADDR, LOOPBACK_PORT},
    learning::FsrsScheduler,
    problems::{Difficulty, NewProblem},
    storage::Database,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const ORIGIN: &str = "chrome-extension://part4-test";

fn app_state() -> AppState {
    let db = Database::in_memory().unwrap();
    AppState {
        inner: Arc::new(Mutex::new(AppInner {
            db,
            scheduler: FsrsScheduler::default(),
        })),
    }
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn pair(state: AppState, code: &str) -> (StatusCode, Value) {
    let router = build_router(state);
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/pair")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "code": code, "origin": ORIGIN }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({}));
    (status, body)
}

#[tokio::test]
async fn health_ok() {
    let router = build_router(app_state());
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn pair_success_and_failure() {
    let state = app_state();
    let code = {
        let guard = state.inner.lock().unwrap();
        guard.db.get_settings().unwrap().pairing_code
    };

    let (status, _body) = pair(state.clone(), "WRONGCODE").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = pair(state.clone(), &code).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["token"].as_str().unwrap().len() >= 64);
    assert!(body["clientId"].as_i64().unwrap() > 0);

    let reused = pair(state.clone(), &code).await;
    assert_eq!(reused.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_required_and_origin_mismatch() {
    let state = app_state();
    let code = state
        .inner
        .lock()
        .unwrap()
        .db
        .get_settings()
        .unwrap()
        .pairing_code;
    let (_, pair_body) = pair(state.clone(), &code).await;
    let token = pair_body["token"].as_str().unwrap().to_owned();

    let router = build_router(state.clone());
    let unauth = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/problems/add")
                .header("content-type", "application/json")
                .header("idempotency-key", "k1")
                .body(Body::from(
                    json!({
                        "slug": "two-sum",
                        "title": "Two Sum",
                        "difficulty": "Easy"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let mismatch = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/problems/add")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .header("origin", "chrome-extension://other")
                .header("idempotency-key", "k2")
                .body(Body::from(
                    json!({
                        "slug": "two-sum",
                        "title": "Two Sum",
                        "difficulty": "Easy"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mismatch.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn add_is_idempotent_and_accepted_creates_pending() {
    let state = app_state();
    let code = state
        .inner
        .lock()
        .unwrap()
        .db
        .get_settings()
        .unwrap()
        .pairing_code;
    let (_, pair_body) = pair(state.clone(), &code).await;
    let token = pair_body["token"].as_str().unwrap().to_owned();

    let payload = json!({
        "slug": "two-sum",
        "title": "Two Sum",
        "difficulty": "Easy"
    });

    for _ in 0..2 {
        let response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/problems/add")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .header("origin", ORIGIN)
                    .header("idempotency-key", "add-once")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let accepted = build_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/submissions/accepted")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .header("x-ankicode-origin", ORIGIN)
                .header("idempotency-key", "accepted-once")
                .body(Body::from(json!({ "slug": "two-sum" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);

    let pending = state
        .inner
        .lock()
        .unwrap()
        .db
        .list_pending_completions()
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].slug, "two-sum");

    let unknown = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/submissions/accepted")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .header("origin", ORIGIN)
                .header("idempotency-key", "accepted-missing")
                .body(Body::from(json!({ "slug": "missing-problem" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn token_survives_pairing_code_rotate() {
    let state = app_state();
    let code = state
        .inner
        .lock()
        .unwrap()
        .db
        .get_settings()
        .unwrap()
        .pairing_code;
    let (_, pair_body) = pair(state.clone(), &code).await;
    let token = pair_body["token"].as_str().unwrap().to_owned();

    {
        let mut guard = state.inner.lock().unwrap();
        guard.db.regenerate_pairing_code(1_700_000_100).unwrap();
        guard
            .db
            .upsert_problem(
                &NewProblem::new(
                    "two-sum",
                    "Two Sum",
                    "https://leetcode.com/problems/two-sum/",
                    Difficulty::Easy,
                )
                .unwrap(),
                1_700_000_100,
            )
            .unwrap();
    }

    let response = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/submissions/accepted")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .header("origin", ORIGIN)
                .header("idempotency-key", "after-rotate")
                .body(Body::from(json!({ "slug": "two-sum" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn bind_address_is_loopback_only() {
    assert_eq!(LOOPBACK_ADDR, "127.0.0.1");
    assert_eq!(LOOPBACK_PORT, 17342);
    let listener = tokio::net::TcpListener::bind((LOOPBACK_ADDR, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    assert!(addr.ip().is_loopback());
    assert!(!addr.ip().is_unspecified());
}

#[tokio::test]
async fn accepted_is_stable_per_problem() {
    let state = app_state();
    let code = state
        .inner
        .lock()
        .unwrap()
        .db
        .get_settings()
        .unwrap()
        .pairing_code;
    let (_, pair_body) = pair(state.clone(), &code).await;
    let token = pair_body["token"].as_str().unwrap().to_owned();

    let add = build_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/problems/add")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .header("origin", ORIGIN)
                .header("idempotency-key", "add:two-sum")
                .body(Body::from(
                    json!({
                        "slug": "two-sum",
                        "title": "Two Sum",
                        "difficulty": "Easy"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(add.status(), StatusCode::OK);

    for key in ["accepted:two-sum", "accepted:two-sum:reload"] {
        let response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/submissions/accepted")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .header("origin", ORIGIN)
                    .header("idempotency-key", key)
                    .body(Body::from(json!({ "slug": "two-sum" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let pending = state
        .inner
        .lock()
        .unwrap()
        .db
        .list_pending_completions()
        .unwrap();
    assert_eq!(pending.len(), 1);
}
