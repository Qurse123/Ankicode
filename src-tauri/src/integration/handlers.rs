use super::{
    auth::{auth_status, AuthenticatedClient},
    types::{
        parse_difficulty, AcceptedSubmissionRequest, AcceptedSubmissionResponse, AddProblemRequest,
        AddProblemResponse, HealthResponse, PairRequest, PairResponse,
    },
};
use crate::{
    commands::AppState,
    problems::{canonical_problem_url, NewProblem},
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;

fn now() -> i64 {
    Utc::now().timestamp()
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Idempotency-Key header is required".to_owned(),
        ))
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

pub async fn pair(
    State(state): State<AppState>,
    Json(body): Json<PairRequest>,
) -> Result<Json<PairResponse>, (StatusCode, String)> {
    let mut guard = state.inner.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "lock poisoned".to_owned(),
        )
    })?;
    let (client_id, token) = guard
        .db
        .create_client(&body.code, &body.origin, now())
        .map_err(|error| (auth_status(&error), error.to_string()))?;
    Ok(Json(PairResponse { token, client_id }))
}

pub async fn add_problem(
    State(state): State<AppState>,
    AuthenticatedClient(client): AuthenticatedClient,
    headers: HeaderMap,
    Json(body): Json<AddProblemRequest>,
) -> Result<Json<AddProblemResponse>, (StatusCode, String)> {
    let key = idempotency_key(&headers)?;
    let difficulty = parse_difficulty(&body.difficulty).ok_or((
        StatusCode::BAD_REQUEST,
        "difficulty must be Easy, Medium, or Hard".to_owned(),
    ))?;
    let url = body
        .url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| canonical_problem_url(body.slug.trim()));
    let new_problem = NewProblem::new(
        body.slug.trim().to_owned(),
        body.title.trim().to_owned(),
        url,
        difficulty,
    )
    .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let payload = serde_json::to_string(&body)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let mut guard = state.inner.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "lock poisoned".to_owned(),
        )
    })?;
    guard
        .db
        .record_integration_event(client.id, &key, "problems.add", &payload, now())
        .map_err(|error| (auth_status(&error), error.to_string()))?;
    let problem = guard
        .db
        .upsert_problem(&new_problem, now())
        .map_err(|error| (auth_status(&error), error.to_string()))?;
    Ok(Json(AddProblemResponse {
        problem_id: problem.id,
        slug: problem.slug,
    }))
}

pub async fn accepted_submission(
    State(state): State<AppState>,
    AuthenticatedClient(client): AuthenticatedClient,
    headers: HeaderMap,
    Json(body): Json<AcceptedSubmissionRequest>,
) -> Result<Json<AcceptedSubmissionResponse>, (StatusCode, String)> {
    let key = idempotency_key(&headers)?;
    let slug = body.slug.trim();
    if slug.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "slug is required".to_owned()));
    }
    let payload = serde_json::to_string(&body)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let accepted_at = body.accepted_at.unwrap_or_else(now);

    let mut guard = state.inner.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "lock poisoned".to_owned(),
        )
    })?;
    guard
        .db
        .record_integration_event(client.id, &key, "submissions.accepted", &payload, now())
        .map_err(|error| (auth_status(&error), error.to_string()))?;
    let problem = guard
        .db
        .get_problem_by_slug(slug)
        .map_err(|error| (auth_status(&error), error.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown problem slug {slug}"),
        ))?;
    let pending = guard
        .db
        .create_pending_completion(problem.id, &key, accepted_at, now())
        .map_err(|error| (auth_status(&error), error.to_string()))?;
    Ok(Json(AcceptedSubmissionResponse {
        pending_id: pending.id,
        problem_id: pending.problem_id,
    }))
}
