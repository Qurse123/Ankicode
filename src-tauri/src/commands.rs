//! Tauri command handlers for the desktop learning UI.

use crate::{
    backup::BackupDocument,
    daily_queue::{DayWindow, DayWindowError},
    learning::{FsrsScheduler, Rating, ReviewEvent, ScheduleState},
    problems::{Difficulty, NewProblem, Problem, ProblemError, ProblemStatus},
    settings::{AppSettings, SettingsUpdate},
    storage::{Database, StorageError},
};
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;
use thiserror::Error;

pub struct AppState {
    pub inner: Mutex<AppInner>,
}

pub struct AppInner {
    pub db: Database,
    pub scheduler: FsrsScheduler,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    DayWindow(#[from] DayWindowError),
    #[error(transparent)]
    Problem(#[from] ProblemError),
    #[error("failed to open URL: {0}")]
    OpenUrl(String),
    #[error("internal lock poisoned")]
    LockPoisoned,
    #[error("learning error: {0}")]
    Learning(String),
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDto {
    pub settings: AppSettingsDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsDto {
    pub timezone_id: String,
    pub desired_retention: f64,
    pub onboarding_completed: bool,
    pub pairing_code: String,
    pub updated_at: i64,
}

impl From<AppSettings> for AppSettingsDto {
    fn from(settings: AppSettings) -> Self {
        Self {
            timezone_id: settings.timezone_id,
            desired_retention: settings.desired_retention,
            onboarding_completed: settings.onboarding_completed,
            pairing_code: settings.pairing_code,
            updated_at: settings.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayViewDto {
    pub local_date: String,
    pub items: Vec<TodayItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayItemDto {
    pub problem_id: i64,
    pub slug: String,
    pub title: String,
    pub url: String,
    pub difficulty: Difficulty,
    pub cost: u8,
    pub position: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemListItemDto {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub url: String,
    pub difficulty: Difficulty,
    pub status: ProblemStatus,
    pub added_at: i64,
    pub updated_at: i64,
    pub due_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemDetailDto {
    pub problem: Problem,
    pub schedule: Option<ScheduleState>,
    pub history: Vec<ReviewEvent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteOnboardingArgs {
    pub timezone_id: String,
    pub desired_retention: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsArgs {
    pub timezone_id: String,
    pub desired_retention: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProblemArgs {
    pub url: String,
    pub title: Option<String>,
    pub difficulty: Difficulty,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetStatusArgs {
    pub problem_id: i64,
    pub status: ProblemStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordRatingArgs {
    pub problem_id: i64,
    pub rating: Rating,
    pub idempotency_key: String,
}

fn with_state<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&mut AppInner) -> Result<T, CommandError>,
) -> Result<T, CommandError> {
    let mut guard = state.inner.lock().map_err(|_| CommandError::LockPoisoned)?;
    f(&mut guard)
}

fn now() -> i64 {
    Utc::now().timestamp()
}

fn local_date_for_timezone(timezone_id: &str) -> Result<String, CommandError> {
    let timezone: Tz = timezone_id
        .parse()
        .map_err(|_| DayWindowError::InvalidTimeZone(timezone_id.to_owned()))?;
    let local = timezone.from_utc_datetime(&Utc::now().naive_utc());
    Ok(local.format("%Y-%m-%d").to_string())
}

#[tauri::command]
pub fn get_bootstrap(state: State<'_, AppState>) -> Result<BootstrapDto, CommandError> {
    with_state(&state, |inner| {
        Ok(BootstrapDto {
            settings: inner.db.get_settings()?.into(),
        })
    })
}

#[tauri::command]
pub fn complete_onboarding(
    state: State<'_, AppState>,
    args: CompleteOnboardingArgs,
) -> Result<AppSettingsDto, CommandError> {
    with_state(&state, |inner| {
        let update = SettingsUpdate {
            timezone_id: args.timezone_id,
            desired_retention: args.desired_retention,
        };
        let settings = inner.db.complete_onboarding(&update, now())?;
        inner.scheduler = FsrsScheduler::new(settings.desired_retention as f32)
            .map_err(|error| CommandError::Learning(error.to_string()))?;
        Ok(settings.into())
    })
}

#[tauri::command]
pub fn get_today(state: State<'_, AppState>) -> Result<TodayViewDto, CommandError> {
    with_state(&state, |inner| {
        let settings = inner.db.get_settings()?;
        let local_date = local_date_for_timezone(&settings.timezone_id)?;
        let window = DayWindow::from_local_date(&local_date, &settings.timezone_id)?;
        let assignment = inner.db.generate_daily_assignment(&window)?;
        let mut items = Vec::with_capacity(assignment.items.len());
        for item in assignment.items {
            let problem = inner
                .db
                .get_problem(item.problem_id)?
                .ok_or(StorageError::ProblemNotFound(item.problem_id))?;
            items.push(TodayItemDto {
                problem_id: problem.id,
                slug: problem.slug,
                title: problem.title,
                url: problem.url,
                difficulty: problem.difficulty,
                cost: item.cost,
                position: item.position,
            });
        }
        Ok(TodayViewDto {
            local_date: assignment.local_date,
            items,
        })
    })
}

#[tauri::command]
pub fn list_problems_view(
    state: State<'_, AppState>,
) -> Result<Vec<ProblemListItemDto>, CommandError> {
    with_state(&state, |inner| {
        let problems = inner.db.list_problems()?;
        let mut rows = Vec::with_capacity(problems.len());
        for problem in problems {
            let due_at = inner.db.get_schedule(problem.id)?.map(|state| state.due_at);
            rows.push(ProblemListItemDto {
                id: problem.id,
                slug: problem.slug,
                title: problem.title,
                url: problem.url,
                difficulty: problem.difficulty,
                status: problem.status,
                added_at: problem.added_at,
                updated_at: problem.updated_at,
                due_at,
            });
        }
        Ok(rows)
    })
}

#[tauri::command]
pub fn add_problem_from_url(
    state: State<'_, AppState>,
    args: AddProblemArgs,
) -> Result<Problem, CommandError> {
    with_state(&state, |inner| {
        let new_problem = NewProblem::from_url(&args.url, args.title.as_deref(), args.difficulty)?;
        Ok(inner.db.upsert_problem(&new_problem, now())?)
    })
}

#[tauri::command]
pub fn set_problem_status_cmd(
    state: State<'_, AppState>,
    args: SetStatusArgs,
) -> Result<(), CommandError> {
    with_state(&state, |inner| {
        inner
            .db
            .set_problem_status(args.problem_id, args.status, now())?;
        Ok(())
    })
}

#[tauri::command]
pub fn get_problem_detail(
    state: State<'_, AppState>,
    problem_id: i64,
) -> Result<ProblemDetailDto, CommandError> {
    with_state(&state, |inner| {
        let problem = inner
            .db
            .get_problem(problem_id)?
            .ok_or(StorageError::ProblemNotFound(problem_id))?;
        let schedule = inner.db.get_schedule(problem_id)?;
        let history = inner.db.list_review_events(problem_id)?;
        Ok(ProblemDetailDto {
            problem,
            schedule,
            history,
        })
    })
}

#[tauri::command]
pub fn record_rating(
    state: State<'_, AppState>,
    args: RecordRatingArgs,
) -> Result<ScheduleState, CommandError> {
    with_state(&state, |inner| {
        let event = ReviewEvent::new(args.idempotency_key, args.rating, now())
            .map_err(|error| CommandError::Learning(error.to_string()))?;
        Ok(inner
            .db
            .record_review(args.problem_id, event, &inner.scheduler)?)
    })
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    args: UpdateSettingsArgs,
) -> Result<AppSettingsDto, CommandError> {
    with_state(&state, |inner| {
        let update = SettingsUpdate {
            timezone_id: args.timezone_id,
            desired_retention: args.desired_retention,
        };
        let settings = inner.db.update_settings(&update, now())?;
        inner.scheduler = FsrsScheduler::new(settings.desired_retention as f32)
            .map_err(|error| CommandError::Learning(error.to_string()))?;
        Ok(settings.into())
    })
}

#[tauri::command]
pub fn regenerate_pairing_code(state: State<'_, AppState>) -> Result<AppSettingsDto, CommandError> {
    with_state(&state, |inner| {
        Ok(inner.db.regenerate_pairing_code(now())?.into())
    })
}

#[tauri::command]
pub fn export_backup(state: State<'_, AppState>) -> Result<BackupDocument, CommandError> {
    with_state(&state, |inner| Ok(inner.db.export_backup()?))
}

#[tauri::command]
pub fn import_backup(
    state: State<'_, AppState>,
    document: BackupDocument,
) -> Result<AppSettingsDto, CommandError> {
    with_state(&state, |inner| {
        let settings = inner.db.import_backup(&document, now())?;
        inner.scheduler = FsrsScheduler::new(settings.desired_retention as f32)
            .map_err(|error| CommandError::Learning(error.to_string()))?;
        Ok(settings.into())
    })
}

#[tauri::command]
pub fn open_problem_url(url: String) -> Result<(), CommandError> {
    let (_, canonical) =
        crate::problems::parse_leetcode_problem_url(&url).map_err(StorageError::from)?;
    open::that(&canonical).map_err(|error| CommandError::OpenUrl(error.to_string()))?;
    Ok(())
}
