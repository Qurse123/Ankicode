use super::handlers::{accepted_submission, add_problem, health, pair};
use crate::commands::AppState;
use axum::{
    http::{header, HeaderName, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub const LOOPBACK_PORT: u16 = 17342;
pub const LOOPBACK_ADDR: &str = "127.0.0.1";

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            origin
                .to_str()
                .map(|value| value.starts_with("chrome-extension://"))
                .unwrap_or(false)
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("idempotency-key"),
            HeaderName::from_static("x-ankicode-origin"),
        ]);

    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/pair", post(pair))
        .route("/v1/problems/add", post(add_problem))
        .route("/v1/submissions/accepted", post(accepted_submission))
        .layer(cors)
        .with_state(state)
}

pub async fn serve(state: AppState) -> Result<(), std::io::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], LOOPBACK_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local = listener.local_addr()?;
    eprintln!("Ankicode loopback API listening on http://{local}");
    axum::serve(listener, build_router(state)).await
}
