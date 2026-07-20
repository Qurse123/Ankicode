use crate::{problems::Difficulty, storage::PendingCompletion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequest {
    pub code: String,
    pub origin: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResponse {
    pub token: String,
    pub client_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProblemRequest {
    pub slug: String,
    pub title: String,
    pub difficulty: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProblemResponse {
    pub problem_id: i64,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedSubmissionRequest {
    pub slug: String,
    #[serde(default)]
    pub accepted_at: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedSubmissionResponse {
    pub pending_id: i64,
    pub problem_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingCompletionDto {
    pub id: i64,
    pub problem_id: i64,
    pub slug: String,
    pub title: String,
    pub difficulty: Difficulty,
    pub url: String,
    pub idempotency_key: String,
    pub accepted_at: i64,
    pub created_at: i64,
}

impl From<PendingCompletion> for PendingCompletionDto {
    fn from(value: PendingCompletion) -> Self {
        Self {
            id: value.id,
            problem_id: value.problem_id,
            slug: value.slug,
            title: value.title,
            difficulty: value.difficulty,
            url: value.url,
            idempotency_key: value.idempotency_key,
            accepted_at: value.accepted_at,
            created_at: value.created_at,
        }
    }
}

pub fn parse_difficulty(raw: &str) -> Option<Difficulty> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "easy" => Some(Difficulty::Easy),
        "medium" => Some(Difficulty::Medium),
        "hard" => Some(Difficulty::Hard),
        _ => None,
    }
}
