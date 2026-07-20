//! JSON backup document for local learning data.

use crate::{
    learning::{Rating, ScheduleState},
    problems::{Difficulty, ProblemStatus},
    settings::AppSettings,
};
use serde::{Deserialize, Serialize};

pub const BACKUP_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupDocument {
    pub version: u32,
    pub settings: BackupSettings,
    pub problems: Vec<BackupProblem>,
    pub review_events: Vec<BackupReviewEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedules: Option<Vec<BackupSchedule>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupSettings {
    pub timezone_id: String,
    pub desired_retention: f64,
    pub onboarding_completed: bool,
    pub pairing_code: String,
}

impl From<&AppSettings> for BackupSettings {
    fn from(settings: &AppSettings) -> Self {
        Self {
            timezone_id: settings.timezone_id.clone(),
            desired_retention: settings.desired_retention,
            onboarding_completed: settings.onboarding_completed,
            pairing_code: settings.pairing_code.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupProblem {
    pub slug: String,
    pub title: String,
    pub url: String,
    pub difficulty: Difficulty,
    pub status: ProblemStatus,
    pub added_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupReviewEvent {
    pub problem_slug: String,
    pub idempotency_key: String,
    pub rating: Rating,
    pub reviewed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub problem_slug: String,
    pub stability: f32,
    pub difficulty: f32,
    pub due_at: i64,
    pub last_review_at: i64,
}

impl BackupSchedule {
    pub fn from_state(problem_slug: String, state: &ScheduleState) -> Self {
        Self {
            problem_slug,
            stability: state.stability,
            difficulty: state.difficulty,
            due_at: state.due_at,
            last_review_at: state.last_review_at,
        }
    }
}
