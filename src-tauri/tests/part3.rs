use ankicode_lib::{
    backup::{BackupDocument, BackupProblem, BackupReviewEvent, BackupSettings, BACKUP_VERSION},
    daily_queue::DayWindow,
    learning::{FsrsScheduler, Rating, ReviewEvent},
    problems::{Difficulty, NewProblem, ProblemStatus},
    settings::{generate_pairing_code, SettingsUpdate, DEFAULT_RETENTION, DEFAULT_TIMEZONE},
    storage::Database,
};
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;

const DAY: i64 = 86_400;
const T0: i64 = 1_700_000_000;

fn problem(slug: &str, difficulty: Difficulty) -> NewProblem {
    NewProblem::new(
        slug,
        slug.replace('-', " "),
        format!("https://leetcode.com/problems/{slug}/"),
        difficulty,
    )
    .unwrap()
}

#[test]
fn settings_seed_defaults_and_pairing_code_shape() {
    let db = Database::in_memory().unwrap();
    let settings = db.get_settings().unwrap();
    assert_eq!(settings.timezone_id, DEFAULT_TIMEZONE);
    assert!((settings.desired_retention - DEFAULT_RETENTION).abs() < f64::EPSILON);
    assert!(!settings.onboarding_completed);
    assert_eq!(settings.pairing_code.len(), 8);
    assert!(settings
        .pairing_code
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()));
    assert_eq!(generate_pairing_code().len(), 8);
}

#[test]
fn onboarding_flag_and_settings_update() {
    let mut db = Database::in_memory().unwrap();
    let settings = db
        .complete_onboarding(
            &SettingsUpdate {
                timezone_id: "America/Los_Angeles".to_owned(),
                desired_retention: 0.85,
            },
            T0,
        )
        .unwrap();
    assert!(settings.onboarding_completed);
    assert_eq!(settings.timezone_id, "America/Los_Angeles");
    assert!((settings.desired_retention - 0.85).abs() < f64::EPSILON);

    let updated = db
        .update_settings(
            &SettingsUpdate {
                timezone_id: "UTC".to_owned(),
                desired_retention: 0.9,
            },
            T0 + 1,
        )
        .unwrap();
    assert_eq!(updated.timezone_id, "UTC");
    assert!(updated.onboarding_completed);

    let regenerated = db.regenerate_pairing_code(T0 + 2).unwrap();
    assert_ne!(regenerated.pairing_code, settings.pairing_code);
}

#[test]
fn export_import_roundtrip_replaces_learning_data() {
    let mut db = Database::in_memory().unwrap();
    db.complete_onboarding(
        &SettingsUpdate {
            timezone_id: "America/New_York".to_owned(),
            desired_retention: 0.9,
        },
        T0,
    )
    .unwrap();
    let easy = db
        .upsert_problem(&problem("two-sum", Difficulty::Easy), T0)
        .unwrap();
    let hard = db
        .upsert_problem(&problem("median-of-two", Difficulty::Hard), T0 + 1)
        .unwrap();
    db.record_review(
        easy.id,
        ReviewEvent::new("r1", Rating::Medium, T0 + 10).unwrap(),
        &FsrsScheduler::default(),
    )
    .unwrap();
    db.set_problem_status(hard.id, ProblemStatus::Paused, T0 + 11)
        .unwrap();

    let backup = db.export_backup().unwrap();
    assert_eq!(backup.version, BACKUP_VERSION);
    assert_eq!(backup.problems.len(), 2);
    assert_eq!(backup.review_events.len(), 1);

    let mut other = Database::in_memory().unwrap();
    other
        .upsert_problem(&problem("extra-problem", Difficulty::Medium), T0)
        .unwrap();
    let imported = other.import_backup(&backup, T0 + 100).unwrap();
    assert!(imported.onboarding_completed);
    assert_eq!(imported.timezone_id, "America/New_York");

    let problems = other.list_problems().unwrap();
    assert_eq!(problems.len(), 2);
    assert!(problems.iter().any(|item| item.slug == "two-sum"));
    assert!(problems.iter().any(|item| item.slug == "median-of-two"));
    assert!(!problems.iter().any(|item| item.slug == "extra-problem"));

    let two_sum = problems.iter().find(|item| item.slug == "two-sum").unwrap();
    assert_eq!(other.list_review_events(two_sum.id).unwrap().len(), 1);
    assert!(other.get_schedule(two_sum.id).unwrap().is_some());
}

#[test]
fn import_rejects_invalid_backup() {
    let mut db = Database::in_memory().unwrap();
    let invalid = BackupDocument {
        version: 99,
        settings: BackupSettings {
            timezone_id: "America/New_York".to_owned(),
            desired_retention: 0.9,
            onboarding_completed: true,
            pairing_code: "ABCD1234".to_owned(),
        },
        problems: vec![],
        review_events: vec![],
        schedules: None,
    };
    assert!(db.import_backup(&invalid, T0).is_err());

    let dangling = BackupDocument {
        version: BACKUP_VERSION,
        settings: BackupSettings {
            timezone_id: "America/New_York".to_owned(),
            desired_retention: 0.9,
            onboarding_completed: true,
            pairing_code: "ABCD1234".to_owned(),
        },
        problems: vec![BackupProblem {
            slug: "two-sum".to_owned(),
            title: "two sum".to_owned(),
            url: "https://leetcode.com/problems/two-sum/".to_owned(),
            difficulty: Difficulty::Easy,
            status: ProblemStatus::Active,
            added_at: T0,
            updated_at: T0,
        }],
        review_events: vec![BackupReviewEvent {
            problem_slug: "missing".to_owned(),
            idempotency_key: "r1".to_owned(),
            rating: Rating::Medium,
            reviewed_at: T0,
        }],
        schedules: None,
    };
    assert!(db.import_backup(&dangling, T0).is_err());
}

#[test]
fn today_uses_settings_timezone_for_day_window() {
    let mut db = Database::in_memory().unwrap();
    db.complete_onboarding(
        &SettingsUpdate {
            timezone_id: "Asia/Tokyo".to_owned(),
            desired_retention: 0.9,
        },
        T0,
    )
    .unwrap();
    db.upsert_problem(&problem("tokyo-easy", Difficulty::Easy), T0)
        .unwrap();

    let settings = db.get_settings().unwrap();
    let timezone: Tz = settings.timezone_id.parse().unwrap();
    // Pick a UTC instant that is already the next calendar day in Tokyo.
    let utc = Utc.with_ymd_and_hms(2024, 6, 1, 16, 0, 0).unwrap();
    let local_date = timezone
        .from_utc_datetime(&utc.naive_utc())
        .format("%Y-%m-%d")
        .to_string();
    assert_eq!(local_date, "2024-06-02");

    let window = DayWindow::from_local_date(&local_date, &settings.timezone_id).unwrap();
    let assignment = db.generate_daily_assignment(&window).unwrap();
    assert_eq!(assignment.local_date, "2024-06-02");
    assert_eq!(assignment.items.len(), 1);

    // Same civil date with America/New_York would be a different UTC window.
    let ny = DayWindow::from_local_date("2024-06-02", "America/New_York").unwrap();
    assert_ne!(ny.start_utc(), window.start_utc());
}

#[test]
fn hard_problems_never_enter_today_assignment() {
    let mut db = Database::in_memory().unwrap();
    db.upsert_problem(&problem("hard-only", Difficulty::Hard), T0)
        .unwrap();
    let window = DayWindow::from_local_date("2024-06-01", "America/New_York").unwrap();
    let assignment = db.generate_daily_assignment(&window).unwrap();
    assert!(assignment.items.is_empty());
    let listed = db.list_problems().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].difficulty, Difficulty::Hard);
}

#[test]
fn timezone_change_clears_daily_queue() {
    let mut db = Database::in_memory().unwrap();
    db.upsert_problem(&problem("clear-me", Difficulty::Easy), T0)
        .unwrap();
    let window = DayWindow::from_local_date("2024-07-01", "America/New_York").unwrap();
    db.generate_daily_assignment(&window).unwrap();
    assert!(db.load_daily_assignment("2024-07-01").unwrap().is_some());

    db.update_settings(
        &SettingsUpdate {
            timezone_id: "UTC".to_owned(),
            desired_retention: 0.9,
        },
        T0 + DAY,
    )
    .unwrap();
    assert!(db.load_daily_assignment("2024-07-01").unwrap().is_none());
}

#[test]
fn readding_removed_problem_reactivates_it() {
    let mut db = Database::in_memory().unwrap();
    let saved = db
        .upsert_problem(
            &NewProblem::new(
                "two-sum",
                "Two Sum",
                "https://leetcode.com/problems/two-sum/",
                Difficulty::Easy,
            )
            .unwrap(),
            T0,
        )
        .unwrap();
    db.set_problem_status(saved.id, ProblemStatus::Removed, T0 + 1)
        .unwrap();
    assert_eq!(
        db.get_problem(saved.id).unwrap().unwrap().status,
        ProblemStatus::Removed
    );

    let again = db
        .upsert_problem(
            &NewProblem::new(
                "two-sum",
                "Two Sum",
                "https://leetcode.com/problems/two-sum/",
                Difficulty::Easy,
            )
            .unwrap(),
            T0 + 2,
        )
        .unwrap();
    assert_eq!(again.id, saved.id);
    assert_eq!(again.status, ProblemStatus::Active);
}

#[test]
fn delete_problem_removes_history_and_allows_readd() {
    let mut db = Database::in_memory().unwrap();
    let scheduler = FsrsScheduler::new(0.9).unwrap();
    let saved = db
        .upsert_problem(&problem("two-sum", Difficulty::Easy), T0)
        .unwrap();
    let window = DayWindow::new("2024-07-01", T0, T0 + DAY).unwrap();
    let assignment = db.generate_daily_assignment(&window).unwrap();
    assert!(assignment
        .items
        .iter()
        .any(|item| item.problem_id == saved.id));
    db.record_review(
        saved.id,
        ReviewEvent::new("review-1", Rating::Medium, T0 + 1).unwrap(),
        &scheduler,
    )
    .unwrap();
    db.create_pending_completion(saved.id, "accepted:two-sum", T0 + 2, T0 + 2)
        .unwrap();

    db.delete_problem(saved.id).unwrap();

    assert!(db.get_problem(saved.id).unwrap().is_none());
    assert!(db.list_problems().unwrap().is_empty());
    assert!(db.list_review_events(saved.id).unwrap().is_empty());
    assert!(db.get_schedule(saved.id).unwrap().is_none());
    assert!(db.list_pending_completions().unwrap().is_empty());
    let remaining = db.load_daily_assignment("2024-07-01").unwrap().unwrap();
    assert!(!remaining
        .items
        .iter()
        .any(|item| item.problem_id == saved.id));
    assert!(matches!(
        db.delete_problem(saved.id).unwrap_err(),
        ankicode_lib::storage::StorageError::ProblemNotFound(_)
    ));

    let again = db
        .upsert_problem(&problem("two-sum", Difficulty::Easy), T0 + 3)
        .unwrap();
    assert_eq!(again.status, ProblemStatus::Active);
    assert!(db.get_problem(again.id).unwrap().is_some());
    assert!(db.list_review_events(again.id).unwrap().is_empty());
    assert!(db.get_schedule(again.id).unwrap().is_none());
}

#[test]
fn review_streak_counts_consecutive_local_days() {
    let mut db = Database::in_memory().unwrap();
    let scheduler = FsrsScheduler::new(0.9).unwrap();
    let settings = db.get_settings().unwrap();
    assert_eq!(settings.timezone_id, DEFAULT_TIMEZONE);
    db.update_settings(
        &SettingsUpdate {
            timezone_id: "UTC".to_owned(),
            desired_retention: DEFAULT_RETENTION,
        },
        T0,
    )
    .unwrap();
    let saved = db
        .upsert_problem(&problem("two-sum", Difficulty::Easy), T0)
        .unwrap();
    assert_eq!(db.review_streak_days("UTC", T0 + DAY).unwrap(), 0);

    db.record_review(
        saved.id,
        ReviewEvent::new("day-0", Rating::Medium, T0 + 3_600).unwrap(),
        &scheduler,
    )
    .unwrap();
    assert_eq!(db.review_streak_days("UTC", T0 + 3_600).unwrap(), 1);

    db.record_review(
        saved.id,
        ReviewEvent::new("day-1", Rating::Medium, T0 + DAY + 3_600).unwrap(),
        &scheduler,
    )
    .unwrap();
    assert_eq!(
        db.review_streak_days("UTC", T0 + DAY + 3_600).unwrap(),
        2
    );
}

