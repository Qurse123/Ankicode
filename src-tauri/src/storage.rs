//! SQLite connection, migration, and repository boundary.

use crate::{
    backup::{
        BackupDocument, BackupProblem, BackupReviewEvent, BackupSchedule, BackupSettings,
        BACKUP_VERSION,
    },
    daily_queue::{AssignmentError, DailyAssignment, DailyAssignmentItem, DayWindow, DAILY_BUDGET},
    learning::{FsrsScheduler, LearningError, Rating, ReviewEvent, ScheduleState},
    problems::{Difficulty, NewProblem, Problem, ProblemError, ProblemStatus},
    settings::{
        generate_pairing_code, AppSettings, SettingsError, SettingsUpdate, DEFAULT_RETENTION,
        DEFAULT_TIMEZONE,
    },
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};
use thiserror::Error;

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_part2",
        r#"
    CREATE TABLE problems (
        id INTEGER PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE CHECK(length(trim(slug)) > 0),
        title TEXT NOT NULL CHECK(length(trim(title)) > 0),
        url TEXT NOT NULL UNIQUE CHECK(url = 'https://leetcode.com/problems/' || slug || '/'),
        difficulty TEXT NOT NULL CHECK(difficulty IN ('easy', 'medium', 'hard')),
        updated_at INTEGER NOT NULL CHECK(typeof(updated_at) = 'integer')
    );
    CREATE TABLE user_problems (
        problem_id INTEGER PRIMARY KEY REFERENCES problems(id) ON DELETE RESTRICT,
        status TEXT NOT NULL DEFAULT 'active'
            CHECK(status IN ('active', 'paused', 'archived', 'removed')),
        added_at INTEGER NOT NULL CHECK(typeof(added_at) = 'integer'),
        updated_at INTEGER NOT NULL CHECK(typeof(updated_at) = 'integer')
    );
    CREATE TABLE review_events (
        id INTEGER PRIMARY KEY,
        problem_id INTEGER NOT NULL REFERENCES user_problems(problem_id) ON DELETE RESTRICT,
        idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
        rating INTEGER NOT NULL CHECK(rating BETWEEN 1 AND 4),
        rating_text TEXT NOT NULL CHECK(rating_text IN ('again', 'hard', 'good', 'easy')),
        reviewed_at INTEGER NOT NULL CHECK(typeof(reviewed_at) = 'integer'),
        CHECK(
            (rating = 1 AND rating_text = 'again') OR
            (rating = 2 AND rating_text = 'hard') OR
            (rating = 3 AND rating_text = 'good') OR
            (rating = 4 AND rating_text = 'easy')
        ),
        UNIQUE(problem_id, idempotency_key)
    );
    CREATE INDEX review_events_problem_time
        ON review_events(problem_id, reviewed_at, id);
    CREATE TRIGGER review_events_no_update
        BEFORE UPDATE ON review_events BEGIN
            SELECT RAISE(ABORT, 'review events are immutable');
        END;
    CREATE TRIGGER review_events_no_delete
        BEFORE DELETE ON review_events BEGIN
            SELECT RAISE(ABORT, 'review events are immutable');
        END;
    CREATE TRIGGER review_events_chronological
        BEFORE INSERT ON review_events BEGIN
            SELECT CASE WHEN NEW.reviewed_at < COALESCE((
                SELECT MAX(reviewed_at) FROM review_events
                WHERE problem_id = NEW.problem_id
            ), NEW.reviewed_at) THEN RAISE(ABORT, 'review events must be chronological') END;
        END;
    CREATE TABLE schedule_states (
        problem_id INTEGER PRIMARY KEY REFERENCES user_problems(problem_id) ON DELETE RESTRICT,
        stability REAL NOT NULL CHECK(typeof(stability) = 'real' AND stability > 0),
        difficulty REAL NOT NULL CHECK(typeof(difficulty) = 'real' AND difficulty > 0),
        due_at INTEGER NOT NULL CHECK(typeof(due_at) = 'integer'),
        last_review_at INTEGER NOT NULL CHECK(typeof(last_review_at) = 'integer')
    );
    CREATE INDEX schedule_states_due ON schedule_states(due_at, problem_id);
    CREATE TABLE daily_queue_generations (
        local_date TEXT PRIMARY KEY
            CHECK(length(local_date) = 10 AND date(local_date) = local_date),
        day_start_utc INTEGER NOT NULL CHECK(typeof(day_start_utc) = 'integer'),
        day_end_utc INTEGER NOT NULL CHECK(typeof(day_end_utc) = 'integer'),
        CHECK(day_start_utc > 0 AND day_end_utc > 0),
        CHECK(day_end_utc - day_start_utc IN (82800, 84600, 86400, 88200, 90000))
    );
    CREATE TABLE daily_assignments (
        local_date TEXT NOT NULL REFERENCES daily_queue_generations(local_date) ON DELETE RESTRICT,
        problem_id INTEGER NOT NULL REFERENCES user_problems(problem_id) ON DELETE RESTRICT,
        position INTEGER NOT NULL CHECK(position >= 0),
        cost INTEGER NOT NULL CHECK(cost IN (1, 2)),
        PRIMARY KEY(local_date, position),
        UNIQUE(local_date, problem_id)
    );
    CREATE INDEX daily_assignments_problem ON daily_assignments(problem_id, local_date);
    CREATE TRIGGER daily_assignments_validate_insert
        BEFORE INSERT ON daily_assignments BEGIN
            SELECT CASE WHEN NEW.position != COALESCE((
                SELECT MAX(position) + 1 FROM daily_assignments
                WHERE local_date = NEW.local_date
            ), 0) THEN RAISE(ABORT, 'assignment positions must be contiguous') END;
            SELECT CASE WHEN NEW.cost != (
                SELECT CASE p.difficulty
                    WHEN 'easy' THEN 1 WHEN 'medium' THEN 2 ELSE 0 END
                FROM problems p WHERE p.id = NEW.problem_id
            ) THEN RAISE(ABORT, 'assignment cost does not match difficulty') END;
            SELECT CASE WHEN NEW.cost + COALESCE((
                SELECT SUM(cost) FROM daily_assignments
                WHERE local_date = NEW.local_date
            ), 0) > 2 THEN RAISE(ABORT, 'daily assignment budget exceeded') END;
        END;
    CREATE TRIGGER daily_assignments_no_update
        BEFORE UPDATE ON daily_assignments BEGIN
            SELECT RAISE(ABORT, 'daily assignments are immutable');
        END;
    CREATE TRIGGER daily_assignments_no_delete
        BEFORE DELETE ON daily_assignments BEGIN
            SELECT RAISE(ABORT, 'daily assignments are immutable');
        END;
    CREATE TRIGGER daily_queue_generations_no_update
        BEFORE UPDATE ON daily_queue_generations BEGIN
            SELECT RAISE(ABORT, 'daily queue generations are immutable');
        END;
    CREATE TRIGGER daily_queue_generations_no_delete
        BEFORE DELETE ON daily_queue_generations BEGIN
            SELECT RAISE(ABORT, 'daily queue generations are immutable');
        END;
    CREATE TABLE integration_clients (
        id INTEGER PRIMARY KEY,
        token_hash TEXT NOT NULL UNIQUE
            CHECK(length(token_hash) = 64 AND token_hash GLOB lower(token_hash)
                  AND token_hash NOT GLOB '*[^0-9a-f]*'),
        allowed_origin TEXT NOT NULL CHECK(length(trim(allowed_origin)) > 0),
        created_at INTEGER NOT NULL CHECK(typeof(created_at) = 'integer'),
        revoked_at INTEGER CHECK(revoked_at IS NULL OR typeof(revoked_at) = 'integer')
    );
    CREATE TABLE integration_events (
        id INTEGER PRIMARY KEY,
        client_id INTEGER NOT NULL REFERENCES integration_clients(id) ON DELETE RESTRICT,
        idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
        received_at INTEGER NOT NULL CHECK(typeof(received_at) = 'integer'),
        kind TEXT NOT NULL CHECK(length(trim(kind)) > 0),
        payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
        UNIQUE(client_id, idempotency_key)
    );
    CREATE INDEX integration_events_time ON integration_events(received_at, id);
    CREATE TRIGGER integration_events_no_update
        BEFORE UPDATE ON integration_events BEGIN
            SELECT RAISE(ABORT, 'integration events are immutable');
        END;
    CREATE TRIGGER integration_events_no_delete
        BEFORE DELETE ON integration_events BEGIN
            SELECT RAISE(ABORT, 'integration events are immutable');
        END;
    "#,
    ),
    (
        "0002_part3",
        r#"
    CREATE TABLE app_settings (
        id INTEGER PRIMARY KEY CHECK(id = 1),
        timezone_id TEXT NOT NULL CHECK(length(trim(timezone_id)) > 0),
        desired_retention REAL NOT NULL CHECK(
            typeof(desired_retention) = 'real'
            AND desired_retention > 0
            AND desired_retention < 1
        ),
        onboarding_completed INTEGER NOT NULL CHECK(onboarding_completed IN (0, 1)),
        pairing_code TEXT NOT NULL CHECK(length(trim(pairing_code)) > 0),
        updated_at INTEGER NOT NULL CHECK(typeof(updated_at) = 'integer')
    );
    "#,
    ),
];

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("learning projection error: {0}")]
    Learning(#[from] LearningError),
    #[error("problem validation error: {0}")]
    Problem(#[from] ProblemError),
    #[error("settings error: {0}")]
    Settings(#[from] SettingsError),
    #[error("invalid persisted value: {0}")]
    InvalidData(String),
    #[error("problem {0} was not found")]
    ProblemNotFound(i64),
    #[error("problem slug {0} was not found")]
    ProblemSlugNotFound(String),
    #[error("a review did not produce a schedule")]
    MissingProjection,
    #[error("review idempotency key {key} has a different payload")]
    ReviewIdempotencyConflict { key: String },
    #[error(
        "persisted day {local_date} has UTC bounds {stored_start}..{stored_end}, not {requested_start}..{requested_end}"
    )]
    DayWindowMismatch {
        local_date: String,
        stored_start: i64,
        stored_end: i64,
        requested_start: i64,
        requested_end: i64,
    },
    #[error("migration {version} checksum drift: expected {expected}, found {actual}")]
    MigrationChecksumMismatch {
        version: String,
        expected: String,
        actual: String,
    },
    #[error("invalid persisted assignment: {0}")]
    InvalidAssignment(#[from] AssignmentError),
    #[error("backup validation failed: {0}")]
    InvalidBackup(String),
}

pub struct Database {
    connection: Connection,
}

impl Database {
    /// Opens one owned SQLite connection, enables foreign keys, and migrates it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Self::initialize(Connection::open(path)?)
    }

    /// Opens and migrates an isolated in-memory SQLite database.
    pub fn in_memory() -> Result<Self, StorageError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(connection: Connection) -> Result<Self, StorageError> {
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "busy_timeout", 5_000)?;
        let mut database = Self { connection };
        database.run_migrations()?;
        Ok(database)
    }

    /// Applies checksummed migrations under SQLite IMMEDIATE writer locks.
    pub fn run_migrations(&mut self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                checksum TEXT NOT NULL,
                applied_at INTEGER NOT NULL DEFAULT (unixepoch())
                    CHECK(typeof(applied_at) = 'integer')
            );",
        )?;
        for (version, sql) in MIGRATIONS {
            let expected = migration_checksum(sql);
            let transaction = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let actual = transaction
                .query_row(
                    "SELECT checksum FROM schema_migrations WHERE version = ?1",
                    [version],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if let Some(actual) = actual {
                if actual != expected {
                    return Err(StorageError::MigrationChecksumMismatch {
                        version: (*version).to_owned(),
                        expected,
                        actual,
                    });
                }
            } else {
                transaction.execute_batch(sql)?;
                transaction.execute(
                    "INSERT INTO schema_migrations(version, checksum) VALUES (?1, ?2)",
                    params![version, expected],
                )?;
            }
            transaction.commit()?;
        }
        self.ensure_default_settings()?;
        Ok(())
    }

    /// Seeds the singleton settings row when missing after migrations.
    pub fn ensure_default_settings(&mut self) -> Result<(), StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists = transaction
            .query_row("SELECT 1 FROM app_settings WHERE id = 1", [], |_| Ok(()))
            .optional()?
            .is_some();
        if !exists {
            let now = utc_now();
            transaction.execute(
                "INSERT INTO app_settings(
                    id, timezone_id, desired_retention, onboarding_completed, pairing_code, updated_at
                 ) VALUES (1, ?1, ?2, 0, ?3, ?4)",
                params![
                    DEFAULT_TIMEZONE,
                    DEFAULT_RETENTION,
                    generate_pairing_code(),
                    now
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn schema_migration_count(&self) -> Result<u32, StorageError> {
        self.connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .map_err(Into::into)
    }

    pub fn get_settings(&self) -> Result<AppSettings, StorageError> {
        let settings = self.connection.query_row(
            "SELECT timezone_id, desired_retention, onboarding_completed, pairing_code, updated_at
             FROM app_settings WHERE id = 1",
            [],
            |row| {
                Ok(AppSettings {
                    timezone_id: row.get(0)?,
                    desired_retention: row.get(1)?,
                    onboarding_completed: row.get::<_, i64>(2)? != 0,
                    pairing_code: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn update_settings(
        &mut self,
        update: &SettingsUpdate,
        now: i64,
    ) -> Result<AppSettings, StorageError> {
        update.validate()?;
        let previous = self.get_settings()?;
        let timezone_changed = previous.timezone_id != update.timezone_id;
        let retention_changed =
            (previous.desired_retention - update.desired_retention).abs() > f64::EPSILON;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "UPDATE app_settings
             SET timezone_id = ?1, desired_retention = ?2, updated_at = ?3
             WHERE id = 1",
            params![update.timezone_id, update.desired_retention, now],
        )?;
        if timezone_changed {
            clear_daily_queue_tables(&transaction)?;
        }
        if retention_changed {
            let scheduler = FsrsScheduler::new(update.desired_retention as f32)?;
            rebuild_all_projections_in(&transaction, &scheduler)?;
        }
        transaction.commit()?;
        self.get_settings()
    }

    pub fn complete_onboarding(
        &mut self,
        update: &SettingsUpdate,
        now: i64,
    ) -> Result<AppSettings, StorageError> {
        update.validate()?;
        let previous = self.get_settings()?;
        let timezone_changed = previous.timezone_id != update.timezone_id;
        let retention_changed =
            (previous.desired_retention - update.desired_retention).abs() > f64::EPSILON;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "UPDATE app_settings
             SET timezone_id = ?1, desired_retention = ?2,
                 onboarding_completed = 1, updated_at = ?3
             WHERE id = 1",
            params![update.timezone_id, update.desired_retention, now],
        )?;
        if timezone_changed {
            clear_daily_queue_tables(&transaction)?;
        }
        if retention_changed {
            let scheduler = FsrsScheduler::new(update.desired_retention as f32)?;
            rebuild_all_projections_in(&transaction, &scheduler)?;
        }
        transaction.commit()?;
        self.get_settings()
    }

    pub fn regenerate_pairing_code(&mut self, now: i64) -> Result<AppSettings, StorageError> {
        let code = generate_pairing_code();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "UPDATE app_settings SET pairing_code = ?1, updated_at = ?2 WHERE id = 1",
            params![code, now],
        )?;
        transaction.commit()?;
        self.get_settings()
    }

    pub fn export_backup(&self) -> Result<BackupDocument, StorageError> {
        let settings = self.get_settings()?;
        let problems = self.list_problems()?;
        let mut backup_problems = Vec::with_capacity(problems.len());
        let mut review_events = Vec::new();
        let mut schedules = Vec::new();
        for problem in &problems {
            backup_problems.push(BackupProblem {
                slug: problem.slug.clone(),
                title: problem.title.clone(),
                url: problem.url.clone(),
                difficulty: problem.difficulty,
                status: problem.status,
                added_at: problem.added_at,
                updated_at: problem.updated_at,
            });
            for event in self.list_review_events(problem.id)? {
                review_events.push(BackupReviewEvent {
                    problem_slug: problem.slug.clone(),
                    idempotency_key: event.idempotency_key().to_owned(),
                    rating: event.rating(),
                    reviewed_at: event.reviewed_at(),
                });
            }
            if let Some(schedule) = self.get_schedule(problem.id)? {
                schedules.push(BackupSchedule::from_state(problem.slug.clone(), &schedule));
            }
        }
        Ok(BackupDocument {
            version: BACKUP_VERSION,
            settings: BackupSettings::from(&settings),
            problems: backup_problems,
            review_events,
            schedules: Some(schedules),
        })
    }

    /// Validates and replaces local learning data in one IMMEDIATE transaction.
    pub fn import_backup(
        &mut self,
        document: &BackupDocument,
        now: i64,
    ) -> Result<AppSettings, StorageError> {
        validate_backup(document)?;
        let scheduler = FsrsScheduler::new(document.settings.desired_retention as f32)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        clear_learning_tables(&transaction)?;

        for problem in &document.problems {
            NewProblem::new(
                problem.slug.clone(),
                problem.title.clone(),
                problem.url.clone(),
                problem.difficulty,
            )?;
            transaction.execute(
                "INSERT INTO problems(slug, title, url, difficulty, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    problem.slug,
                    problem.title,
                    problem.url,
                    problem.difficulty.as_db_str(),
                    problem.updated_at
                ],
            )?;
            let problem_id: i64 = transaction.query_row(
                "SELECT id FROM problems WHERE slug = ?1",
                [&problem.slug],
                |row| row.get(0),
            )?;
            transaction.execute(
                "INSERT INTO user_problems(problem_id, status, added_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    problem_id,
                    problem.status.as_db_str(),
                    problem.added_at,
                    problem.updated_at
                ],
            )?;
        }

        let mut slug_to_id = HashMap::new();
        {
            let mut statement = transaction.prepare("SELECT id, slug FROM problems")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (id, slug) = row?;
                slug_to_id.insert(slug, id);
            }
        }

        let mut events_by_problem: HashMap<i64, Vec<&BackupReviewEvent>> = HashMap::new();
        for event in &document.review_events {
            let problem_id = *slug_to_id
                .get(&event.problem_slug)
                .ok_or_else(|| StorageError::ProblemSlugNotFound(event.problem_slug.clone()))?;
            events_by_problem.entry(problem_id).or_default().push(event);
        }
        for events in events_by_problem.values_mut() {
            events.sort_by_key(|event| (event.reviewed_at, event.idempotency_key.as_str()));
        }
        for (problem_id, events) in &events_by_problem {
            for event in events {
                ReviewEvent::new(
                    event.idempotency_key.clone(),
                    event.rating,
                    event.reviewed_at,
                )?;
                transaction.execute(
                    "INSERT INTO review_events(
                        problem_id, idempotency_key, rating, rating_text, reviewed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        problem_id,
                        event.idempotency_key,
                        event.rating.fsrs_value(),
                        event.rating.as_db_str(),
                        event.reviewed_at
                    ],
                )?;
            }
        }

        transaction.execute(
            "UPDATE app_settings
             SET timezone_id = ?1, desired_retention = ?2, onboarding_completed = ?3,
                 pairing_code = ?4, updated_at = ?5
             WHERE id = 1",
            params![
                document.settings.timezone_id,
                document.settings.desired_retention,
                i64::from(document.settings.onboarding_completed),
                document.settings.pairing_code,
                now
            ],
        )?;

        rebuild_all_projections_in(&transaction, &scheduler)?;
        restore_immutable_triggers(&transaction)?;
        transaction.commit()?;
        self.get_settings()
    }

    /// Rebuilds every schedule projection using the provided scheduler.
    pub fn rebuild_all_projections(
        &mut self,
        scheduler: &FsrsScheduler,
    ) -> Result<(), StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        rebuild_all_projections_in(&transaction, scheduler)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn upsert_problem(
        &mut self,
        new_problem: &NewProblem,
        now: i64,
    ) -> Result<Problem, StorageError> {
        new_problem.validate()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO problems(slug, title, url, difficulty, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(slug) DO UPDATE SET
                title = excluded.title, url = excluded.url,
                difficulty = excluded.difficulty, updated_at = excluded.updated_at",
            params![
                new_problem.slug,
                new_problem.title,
                new_problem.url,
                new_problem.difficulty.as_db_str(),
                now
            ],
        )?;
        let problem_id: i64 = transaction.query_row(
            "SELECT id FROM problems WHERE slug = ?1",
            [&new_problem.slug],
            |row| row.get(0),
        )?;
        transaction.execute(
            "INSERT INTO user_problems(problem_id, status, added_at, updated_at)
             VALUES (?1, 'active', ?2, ?2)
             ON CONFLICT(problem_id) DO UPDATE SET
                status = CASE
                    WHEN user_problems.status = 'removed' THEN 'active'
                    ELSE user_problems.status
                END,
                updated_at = CASE
                    WHEN user_problems.status = 'removed' THEN excluded.updated_at
                    ELSE user_problems.updated_at
                END",
            params![problem_id, now],
        )?;
        transaction.commit()?;
        self.get_problem(problem_id)?
            .ok_or(StorageError::ProblemNotFound(problem_id))
    }

    pub fn get_problem(&self, problem_id: i64) -> Result<Option<Problem>, StorageError> {
        query_problem(&self.connection, problem_id)
    }

    pub fn list_problems(&self) -> Result<Vec<Problem>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT p.id, p.slug, p.title, p.url, p.difficulty,
                    u.status, u.added_at, u.updated_at
             FROM problems p JOIN user_problems u ON u.problem_id = p.id
             ORDER BY u.added_at, p.id",
        )?;
        let rows = statement.query_map([], raw_problem)?;
        let mut problems = Vec::new();
        for row in rows {
            problems.push(parse_problem(row?)?);
        }
        Ok(problems)
    }

    pub fn set_problem_status(
        &mut self,
        problem_id: i64,
        status: ProblemStatus,
        now: i64,
    ) -> Result<(), StorageError> {
        let changed = self.connection.execute(
            "UPDATE user_problems SET status = ?1, updated_at = ?2 WHERE problem_id = ?3",
            params![status.as_db_str(), now, problem_id],
        )?;
        if changed == 0 {
            return Err(StorageError::ProblemNotFound(problem_id));
        }
        Ok(())
    }

    /// Appends a keyed review and updates its projection in one transaction.
    pub fn record_review(
        &mut self,
        problem_id: i64,
        event: ReviewEvent,
        scheduler: &FsrsScheduler,
    ) -> Result<ScheduleState, StorageError> {
        event.validate()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = transaction
            .query_row(
                "SELECT rating, reviewed_at FROM review_events
                 WHERE problem_id = ?1 AND idempotency_key = ?2",
                params![problem_id, event.idempotency_key()],
                |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        if let Some((rating, reviewed_at)) = existing {
            if rating != event.rating().fsrs_value() || reviewed_at != event.reviewed_at() {
                return Err(StorageError::ReviewIdempotencyConflict {
                    key: event.idempotency_key().to_owned(),
                });
            }
            // Return the projection as of this event, not later schedule state.
            let events = list_review_events_from(&transaction, problem_id)?;
            let mut prefix = Vec::new();
            for historical in events {
                prefix.push(historical.clone());
                if historical.idempotency_key() == event.idempotency_key() {
                    break;
                }
            }
            return scheduler
                .project(&prefix)?
                .ok_or(StorageError::MissingProjection);
        }

        if let Some(previous) = transaction
            .query_row(
                "SELECT reviewed_at FROM review_events
                 WHERE problem_id = ?1 ORDER BY reviewed_at DESC, id DESC LIMIT 1",
                [problem_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            if event.reviewed_at() < previous {
                return Err(StorageError::Learning(LearningError::OutOfOrder {
                    previous,
                    current: event.reviewed_at(),
                }));
            }
        }
        transaction.execute(
            "INSERT INTO review_events(
                problem_id, idempotency_key, rating, rating_text, reviewed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                problem_id,
                event.idempotency_key(),
                event.rating().fsrs_value(),
                event.rating().as_db_str(),
                event.reviewed_at()
            ],
        )?;
        let events = list_review_events_from(&transaction, problem_id)?;
        let state = scheduler
            .project(&events)?
            .ok_or(StorageError::MissingProjection)?;
        upsert_schedule(&transaction, problem_id, &state)?;
        transaction.commit()?;
        Ok(state)
    }

    /// Returns immutable reviews ordered by UTC timestamp, then insertion ID.
    pub fn list_review_events(&self, problem_id: i64) -> Result<Vec<ReviewEvent>, StorageError> {
        list_review_events_from(&self.connection, problem_id)
    }

    pub fn get_schedule(&self, problem_id: i64) -> Result<Option<ScheduleState>, StorageError> {
        schedule_from(&self.connection, problem_id)
    }

    /// Replays all persisted reviews and atomically replaces the projection.
    pub fn rebuild_projection(
        &mut self,
        problem_id: i64,
        scheduler: &FsrsScheduler,
    ) -> Result<Option<ScheduleState>, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let events = list_review_events_from(&transaction, problem_id)?;
        let state = scheduler.project(&events)?;
        if let Some(ref state) = state {
            upsert_schedule(&transaction, problem_id, state)?;
        } else {
            transaction.execute(
                "DELETE FROM schedule_states WHERE problem_id = ?1",
                [problem_id],
            )?;
        }
        transaction.commit()?;
        Ok(state)
    }

    pub fn load_daily_assignment(
        &self,
        local_date: &str,
    ) -> Result<Option<DailyAssignment>, StorageError> {
        load_assignment(&self.connection, local_date)
    }

    /// Loads or permanently creates the assignment for this exact civil window.
    pub fn generate_daily_assignment(
        &mut self,
        window: &DayWindow,
    ) -> Result<DailyAssignment, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let persisted_bounds = transaction
            .query_row(
                "SELECT day_start_utc, day_end_utc FROM daily_queue_generations
                 WHERE local_date = ?1",
                [window.local_date()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        if let Some((stored_start, stored_end)) = persisted_bounds {
            if stored_start != window.start_utc() || stored_end != window.end_utc() {
                return Err(StorageError::DayWindowMismatch {
                    local_date: window.local_date().to_owned(),
                    stored_start,
                    stored_end,
                    requested_start: window.start_utc(),
                    requested_end: window.end_utc(),
                });
            }
            let existing = load_assignment(&transaction, window.local_date())?
                .ok_or_else(|| StorageError::InvalidData("missing day marker".to_owned()))?;
            transaction.commit()?;
            return Ok(existing);
        }
        transaction.execute(
            "INSERT INTO daily_queue_generations(local_date, day_start_utc, day_end_utc)
             VALUES (?1, ?2, ?3)",
            params![window.local_date(), window.start_utc(), window.end_utc()],
        )?;

        let mut candidates = Vec::new();
        {
            let mut due = transaction.prepare(
                "SELECT p.id, p.difficulty
                 FROM problems p
                 JOIN user_problems u ON u.problem_id = p.id
                 JOIN schedule_states s ON s.problem_id = p.id
                 WHERE u.status = 'active' AND p.difficulty != 'hard' AND s.due_at < ?1
                 ORDER BY s.due_at, u.added_at, p.id",
            )?;
            for row in due.query_map([window.end_utc()], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })? {
                candidates.push(row?);
            }
        }
        {
            let mut new_items = transaction.prepare(
                "SELECT p.id, p.difficulty
                 FROM problems p
                 JOIN user_problems u ON u.problem_id = p.id
                 LEFT JOIN schedule_states s ON s.problem_id = p.id
                 WHERE u.status = 'active' AND p.difficulty != 'hard'
                   AND s.problem_id IS NULL
                 ORDER BY u.added_at, p.id",
            )?;
            for row in new_items.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })? {
                candidates.push(row?);
            }
        }

        let mut items = Vec::new();
        let mut remaining = DAILY_BUDGET;
        for (problem_id, difficulty_text) in candidates {
            let difficulty = difficulty_text
                .parse::<Difficulty>()
                .map_err(|error| StorageError::InvalidData(error.to_string()))?;
            let Some(cost) = difficulty.queue_cost() else {
                continue;
            };
            if cost > remaining {
                continue;
            }
            let position = u8::try_from(items.len())
                .map_err(|_| StorageError::InvalidData("assignment too large".to_owned()))?;
            transaction.execute(
                "INSERT INTO daily_assignments(local_date, problem_id, position, cost)
                 VALUES (?1, ?2, ?3, ?4)",
                params![window.local_date(), problem_id, position, cost],
            )?;
            items.push(DailyAssignmentItem {
                problem_id,
                position,
                cost,
            });
            remaining -= cost;
            if remaining == 0 {
                break;
            }
        }
        transaction.commit()?;
        Ok(DailyAssignment {
            local_date: window.local_date().to_owned(),
            items,
        })
    }
}

type RawProblem = (i64, String, String, String, String, String, i64, i64);

fn raw_problem(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawProblem> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

fn parse_problem(raw: RawProblem) -> Result<Problem, StorageError> {
    Ok(Problem {
        id: raw.0,
        slug: raw.1,
        title: raw.2,
        url: raw.3,
        difficulty: raw
            .4
            .parse()
            .map_err(|error: ProblemError| StorageError::InvalidData(error.to_string()))?,
        status: raw
            .5
            .parse()
            .map_err(|error: ProblemError| StorageError::InvalidData(error.to_string()))?,
        added_at: raw.6,
        updated_at: raw.7,
    })
}

fn query_problem(
    connection: &Connection,
    problem_id: i64,
) -> Result<Option<Problem>, StorageError> {
    let raw = connection
        .query_row(
            "SELECT p.id, p.slug, p.title, p.url, p.difficulty,
                    u.status, u.added_at, u.updated_at
             FROM problems p JOIN user_problems u ON u.problem_id = p.id
             WHERE p.id = ?1",
            [problem_id],
            raw_problem,
        )
        .optional()?;
    raw.map(parse_problem).transpose()
}

fn list_review_events_from(
    connection: &Connection,
    problem_id: i64,
) -> Result<Vec<ReviewEvent>, StorageError> {
    let mut statement = connection.prepare(
        "SELECT idempotency_key, rating, reviewed_at FROM review_events
         WHERE problem_id = ?1 ORDER BY reviewed_at, id",
    )?;
    let rows = statement.query_map([problem_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut events = Vec::new();
    for row in rows {
        let (idempotency_key, rating, reviewed_at) = row?;
        events.push(ReviewEvent::new(
            idempotency_key,
            Rating::try_from(rating)?,
            reviewed_at,
        )?);
    }
    Ok(events)
}

fn upsert_schedule(
    transaction: &Transaction<'_>,
    problem_id: i64,
    state: &ScheduleState,
) -> Result<(), StorageError> {
    transaction.execute(
        "INSERT INTO schedule_states(
            problem_id, stability, difficulty, due_at, last_review_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(problem_id) DO UPDATE SET
            stability = excluded.stability, difficulty = excluded.difficulty,
            due_at = excluded.due_at, last_review_at = excluded.last_review_at",
        params![
            problem_id,
            state.stability,
            state.difficulty,
            state.due_at,
            state.last_review_at
        ],
    )?;
    Ok(())
}

fn schedule_from(
    connection: &Connection,
    problem_id: i64,
) -> Result<Option<ScheduleState>, StorageError> {
    connection
        .query_row(
            "SELECT stability, difficulty, due_at, last_review_at
             FROM schedule_states WHERE problem_id = ?1",
            [problem_id],
            |row| {
                Ok(ScheduleState {
                    stability: row.get(0)?,
                    difficulty: row.get(1)?,
                    due_at: row.get(2)?,
                    last_review_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_assignment(
    connection: &Connection,
    local_date: &str,
) -> Result<Option<DailyAssignment>, StorageError> {
    let generated = connection
        .query_row(
            "SELECT 1 FROM daily_queue_generations WHERE local_date = ?1",
            [local_date],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !generated {
        return Ok(None);
    }
    let mut statement = connection.prepare(
        "SELECT problem_id, position, cost FROM daily_assignments
         WHERE local_date = ?1 ORDER BY position",
    )?;
    let rows = statement.query_map([local_date], |row| {
        Ok(DailyAssignmentItem {
            problem_id: row.get(0)?,
            position: row.get(1)?,
            cost: row.get(2)?,
        })
    })?;
    let items = rows.collect::<Result<Vec<_>, _>>()?;
    let assignment = DailyAssignment {
        local_date: local_date.to_owned(),
        items,
    };
    assignment.validate()?;
    Ok(Some(assignment))
}

fn migration_checksum(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn utc_now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn validate_backup(document: &BackupDocument) -> Result<(), StorageError> {
    if document.version != BACKUP_VERSION {
        return Err(StorageError::InvalidBackup(format!(
            "unsupported version {}",
            document.version
        )));
    }
    let settings = AppSettings {
        timezone_id: document.settings.timezone_id.clone(),
        desired_retention: document.settings.desired_retention,
        onboarding_completed: document.settings.onboarding_completed,
        pairing_code: document.settings.pairing_code.clone(),
        updated_at: 0,
    };
    settings.validate()?;

    let mut slugs = HashMap::new();
    for problem in &document.problems {
        NewProblem::new(
            problem.slug.clone(),
            problem.title.clone(),
            problem.url.clone(),
            problem.difficulty,
        )?;
        if slugs.insert(problem.slug.clone(), ()).is_some() {
            return Err(StorageError::InvalidBackup(format!(
                "duplicate problem slug {}",
                problem.slug
            )));
        }
    }
    for event in &document.review_events {
        if !slugs.contains_key(&event.problem_slug) {
            return Err(StorageError::InvalidBackup(format!(
                "review references missing slug {}",
                event.problem_slug
            )));
        }
        ReviewEvent::new(
            event.idempotency_key.clone(),
            event.rating,
            event.reviewed_at,
        )?;
    }
    if let Some(schedules) = &document.schedules {
        for schedule in schedules {
            if !slugs.contains_key(&schedule.problem_slug) {
                return Err(StorageError::InvalidBackup(format!(
                    "schedule references missing slug {}",
                    schedule.problem_slug
                )));
            }
        }
    }
    Ok(())
}

fn drop_immutable_delete_triggers(transaction: &Transaction<'_>) -> Result<(), StorageError> {
    transaction.execute_batch(
        "
        DROP TRIGGER IF EXISTS review_events_no_delete;
        DROP TRIGGER IF EXISTS review_events_no_update;
        DROP TRIGGER IF EXISTS daily_assignments_no_delete;
        DROP TRIGGER IF EXISTS daily_assignments_no_update;
        DROP TRIGGER IF EXISTS daily_queue_generations_no_delete;
        DROP TRIGGER IF EXISTS daily_queue_generations_no_update;
        ",
    )?;
    Ok(())
}

fn restore_immutable_triggers(transaction: &Transaction<'_>) -> Result<(), StorageError> {
    transaction.execute_batch(
        "
        CREATE TRIGGER review_events_no_update
            BEFORE UPDATE ON review_events BEGIN
                SELECT RAISE(ABORT, 'review events are immutable');
            END;
        CREATE TRIGGER review_events_no_delete
            BEFORE DELETE ON review_events BEGIN
                SELECT RAISE(ABORT, 'review events are immutable');
            END;
        CREATE TRIGGER daily_assignments_no_update
            BEFORE UPDATE ON daily_assignments BEGIN
                SELECT RAISE(ABORT, 'daily assignments are immutable');
            END;
        CREATE TRIGGER daily_assignments_no_delete
            BEFORE DELETE ON daily_assignments BEGIN
                SELECT RAISE(ABORT, 'daily assignments are immutable');
            END;
        CREATE TRIGGER daily_queue_generations_no_update
            BEFORE UPDATE ON daily_queue_generations BEGIN
                SELECT RAISE(ABORT, 'daily queue generations are immutable');
            END;
        CREATE TRIGGER daily_queue_generations_no_delete
            BEFORE DELETE ON daily_queue_generations BEGIN
                SELECT RAISE(ABORT, 'daily queue generations are immutable');
            END;
        ",
    )?;
    Ok(())
}

fn clear_daily_queue_tables(transaction: &Transaction<'_>) -> Result<(), StorageError> {
    drop_immutable_delete_triggers(transaction)?;
    transaction.execute_batch(
        "
        DELETE FROM daily_assignments;
        DELETE FROM daily_queue_generations;
        ",
    )?;
    restore_immutable_triggers(transaction)?;
    Ok(())
}

fn clear_learning_tables(transaction: &Transaction<'_>) -> Result<(), StorageError> {
    drop_immutable_delete_triggers(transaction)?;
    transaction.execute_batch(
        "
        DELETE FROM daily_assignments;
        DELETE FROM daily_queue_generations;
        DELETE FROM schedule_states;
        DELETE FROM review_events;
        DELETE FROM user_problems;
        DELETE FROM problems;
        ",
    )?;
    Ok(())
}

fn rebuild_all_projections_in(
    transaction: &Transaction<'_>,
    scheduler: &FsrsScheduler,
) -> Result<(), StorageError> {
    let problem_ids = {
        let mut statement = transaction.prepare("SELECT problem_id FROM user_problems")?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for problem_id in problem_ids {
        let events = list_review_events_from(transaction, problem_id)?;
        let state = scheduler.project(&events)?;
        if let Some(ref state) = state {
            upsert_schedule(transaction, problem_id, state)?;
        } else {
            transaction.execute(
                "DELETE FROM schedule_states WHERE problem_id = ?1",
                [problem_id],
            )?;
        }
    }
    Ok(())
}
