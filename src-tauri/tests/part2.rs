use ankicode_lib::{
    daily_queue::DayWindow,
    learning::{FsrsScheduler, LearningError, Rating, ReviewEvent},
    problems::{Difficulty, NewProblem, ProblemStatus},
    storage::{Database, StorageError},
};
use rusqlite::{params, Connection};
use tempfile::tempdir;

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

fn review(rating: Rating, reviewed_at: i64) -> ReviewEvent {
    ReviewEvent::new(
        format!("{}-{reviewed_at}", rating.as_db_str()),
        rating,
        reviewed_at,
    )
    .unwrap()
}

#[test]
fn rating_has_stable_numeric_and_string_encodings() {
    let cases = [
        (Rating::Again, 1, "again"),
        (Rating::Hard, 2, "hard"),
        (Rating::Medium, 3, "medium"),
        (Rating::Easy, 4, "easy"),
    ];
    for (rating, number, text) in cases {
        assert_eq!(rating.fsrs_value(), number);
        assert_eq!(rating.as_db_str(), text);
        assert_eq!(Rating::try_from(number).unwrap(), rating);
        assert_eq!(text.parse::<Rating>().unwrap(), rating);
    }
    assert!(Rating::try_from(0).is_err());
    assert!("great".parse::<Rating>().is_err());
}

#[test]
fn scheduler_validates_retention() {
    assert_eq!(FsrsScheduler::default().desired_retention(), 0.9);
    assert!(FsrsScheduler::new(0.8).is_ok());
    assert!(FsrsScheduler::new(0.0).is_err());
    assert!(FsrsScheduler::new(1.0).is_err());
    assert!(FsrsScheduler::new(f32::NAN).is_err());
}

#[test]
fn event_conversion_rejects_reverse_time_and_uses_whole_utc_days() {
    let scheduler = FsrsScheduler::default();
    let item = scheduler
        .to_fsrs_item(&[
            review(Rating::Medium, T0),
            review(Rating::Easy, T0 + DAY + DAY / 2),
        ])
        .unwrap();
    assert_eq!(item.reviews[0].delta_t, 0);
    assert_eq!(item.reviews[1].delta_t, 1);

    let error = scheduler
        .to_fsrs_item(&[review(Rating::Medium, T0), review(Rating::Medium, T0 - 1)])
        .unwrap_err();
    assert!(matches!(error, LearningError::OutOfOrder { .. }));
}

#[test]
fn projection_handles_empty_initial_and_repeated_histories_deterministically() {
    let scheduler = FsrsScheduler::default();
    assert_eq!(scheduler.project(&[]).unwrap(), None);

    let first = scheduler
        .project(&[review(Rating::Medium, T0)])
        .unwrap()
        .unwrap();
    assert!(first.stability.is_finite());
    assert!(first.difficulty.is_finite());
    assert!(first.due_at >= T0 + DAY);

    let history = [
        review(Rating::Medium, T0),
        review(Rating::Hard, T0 + 3 * DAY),
        review(Rating::Easy, T0 + 8 * DAY),
    ];
    assert_eq!(
        scheduler.project(&history).unwrap(),
        scheduler.project(&history).unwrap()
    );
}

#[test]
fn fixed_offset_day_window_handles_local_boundaries() {
    let before_midnight_utc = 1_704_067_199;
    let west = DayWindow::from_utc_timestamp(before_midnight_utc, -300).unwrap();
    assert_eq!(west.local_date(), "2023-12-31");
    assert_eq!(west.end_utc() - west.start_utc(), DAY);
    assert!(west.start_utc() <= before_midnight_utc);
    assert!(before_midnight_utc < west.end_utc());

    let east = DayWindow::from_utc_timestamp(before_midnight_utc, 330).unwrap();
    assert_eq!(east.local_date(), "2024-01-01");
}

#[test]
fn problem_validation_rejects_empty_and_noncanonical_values() {
    assert!(NewProblem::new(
        "",
        "Title",
        "https://leetcode.com/problems/x/",
        Difficulty::Easy
    )
    .is_err());
    assert!(NewProblem::new(
        "x",
        " ",
        "https://leetcode.com/problems/x/",
        Difficulty::Easy
    )
    .is_err());
    assert!(NewProblem::new(
        "x",
        "X",
        "http://leetcode.com/problems/x/",
        Difficulty::Easy
    )
    .is_err());
    assert!(NewProblem::new(
        "x",
        "X",
        "https://example.com/problems/x/",
        Difficulty::Easy
    )
    .is_err());
    assert!(NewProblem::new(
        "x",
        "X",
        "https://leetcode.com/problems/y/",
        Difficulty::Easy
    )
    .is_err());
}

#[test]
fn migrations_are_idempotent_and_constraints_are_enforced() {
    let mut db = Database::in_memory().unwrap();
    let count = db.schema_migration_count().unwrap();
    db.run_migrations().unwrap();
    assert_eq!(db.schema_migration_count().unwrap(), count);
    assert!(count > 0);

    let saved = db
        .upsert_problem(&problem("two-sum", Difficulty::Easy), T0)
        .unwrap();
    assert!(db
        .record_review(999_999, review(Rating::Medium, T0), &FsrsScheduler::default())
        .is_err());
    assert!(db.get_problem(saved.id).unwrap().is_some());
}

#[test]
fn metadata_upsert_preserves_lifecycle_and_history() {
    let mut db = Database::in_memory().unwrap();
    let original = db
        .upsert_problem(&problem("two-sum", Difficulty::Easy), T0)
        .unwrap();
    db.set_problem_status(original.id, ProblemStatus::Paused, T0 + 1)
        .unwrap();
    db.record_review(
        original.id,
        review(Rating::Medium, T0 + 2),
        &FsrsScheduler::default(),
    )
    .unwrap();

    let renamed = NewProblem::new(
        "two-sum",
        "Two Sum Updated",
        "https://leetcode.com/problems/two-sum/",
        Difficulty::Medium,
    )
    .unwrap();
    let updated = db.upsert_problem(&renamed, T0 + 3).unwrap();
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.status, ProblemStatus::Paused);
    assert_eq!(updated.title, "Two Sum Updated");
    assert_eq!(db.list_review_events(original.id).unwrap().len(), 1);
}

#[test]
fn recording_and_rebuilding_projection_are_transactional() {
    let mut db = Database::in_memory().unwrap();
    let saved = db
        .upsert_problem(&problem("two-sum", Difficulty::Easy), T0)
        .unwrap();
    let scheduler = FsrsScheduler::default();
    let state = db
        .record_review(saved.id, review(Rating::Medium, T0 + DAY), &scheduler)
        .unwrap();
    assert_eq!(db.get_schedule(saved.id).unwrap(), Some(state.clone()));

    let failed = db.record_review(saved.id, review(Rating::Again, T0), &scheduler);
    assert!(failed.is_err());
    assert_eq!(db.list_review_events(saved.id).unwrap().len(), 1);

    let rebuilt = db
        .rebuild_projection(saved.id, &scheduler)
        .unwrap()
        .unwrap();
    assert_eq!(rebuilt, state);
}

#[test]
fn queue_respects_weighted_budget_and_excludes_hard() {
    let mut db = Database::in_memory().unwrap();
    let easy1 = db
        .upsert_problem(&problem("easy-one", Difficulty::Easy), T0)
        .unwrap();
    let easy2 = db
        .upsert_problem(&problem("easy-two", Difficulty::Easy), T0 + 1)
        .unwrap();
    db.upsert_problem(&problem("medium", Difficulty::Medium), T0 + 2)
        .unwrap();
    db.upsert_problem(&problem("hard", Difficulty::Hard), T0 + 3)
        .unwrap();
    let window = DayWindow::new("2024-01-01", T0, T0 + DAY).unwrap();

    let assignment = db.generate_daily_assignment(&window).unwrap();
    assert_eq!(assignment.total_cost(), 2);
    assert_eq!(assignment.items.len(), 2);
    assert_eq!(assignment.items[0].problem_id, easy1.id);
    assert_eq!(assignment.items[1].problem_id, easy2.id);
    assert!(assignment.items.iter().all(|item| item.cost == 1));
}

#[test]
fn due_items_precede_new_items_and_ties_are_stable() {
    let mut db = Database::in_memory().unwrap();
    let due1 = db
        .upsert_problem(&problem("due-one", Difficulty::Easy), T0)
        .unwrap();
    let due2 = db
        .upsert_problem(&problem("due-two", Difficulty::Easy), T0)
        .unwrap();
    db.record_review(
        due1.id,
        review(Rating::Again, T0),
        &FsrsScheduler::default(),
    )
    .unwrap();
    db.record_review(
        due2.id,
        review(Rating::Again, T0),
        &FsrsScheduler::default(),
    )
    .unwrap();
    db.upsert_problem(&problem("new", Difficulty::Easy), T0 - 100)
        .unwrap();
    let window = DayWindow::new("2024-02-01", T0 + 30 * DAY, T0 + 31 * DAY).unwrap();

    let assignment = db.generate_daily_assignment(&window).unwrap();
    assert_eq!(
        assignment
            .items
            .iter()
            .map(|item| item.problem_id)
            .collect::<Vec<_>>(),
        vec![due1.id, due2.id]
    );
}

#[test]
fn medium_candidate_can_consume_entire_budget() {
    let mut db = Database::in_memory().unwrap();
    let medium = db
        .upsert_problem(&problem("medium", Difficulty::Medium), T0)
        .unwrap();
    db.upsert_problem(&problem("easy", Difficulty::Easy), T0 + 1)
        .unwrap();
    let assignment = db
        .generate_daily_assignment(&DayWindow::new("2024-03-01", T0, T0 + DAY).unwrap())
        .unwrap();
    assert_eq!(assignment.total_cost(), 2);
    assert_eq!(assignment.items.len(), 1);
    assert_eq!(assignment.items[0].problem_id, medium.id);
}

#[test]
fn empty_and_nonempty_assignments_are_persisted_without_refill() {
    let mut db = Database::in_memory().unwrap();
    let empty_window = DayWindow::new("2024-04-01", T0, T0 + DAY).unwrap();
    assert!(db
        .generate_daily_assignment(&empty_window)
        .unwrap()
        .items
        .is_empty());
    db.upsert_problem(&problem("later", Difficulty::Easy), T0)
        .unwrap();
    assert!(db
        .generate_daily_assignment(&empty_window)
        .unwrap()
        .items
        .is_empty());

    let next = DayWindow::new("2024-04-02", T0 + DAY, T0 + 2 * DAY).unwrap();
    let fixed = db.generate_daily_assignment(&next).unwrap();
    assert_eq!(fixed.items.len(), 1);
    db.upsert_problem(&problem("another", Difficulty::Easy), T0 + 1)
        .unwrap();
    assert_eq!(db.generate_daily_assignment(&next).unwrap(), fixed);
}

#[test]
fn pausing_or_archiving_only_affects_future_assignments() {
    let mut db = Database::in_memory().unwrap();
    let first = db
        .upsert_problem(&problem("first", Difficulty::Easy), T0)
        .unwrap();
    let second = db
        .upsert_problem(&problem("second", Difficulty::Easy), T0 + 1)
        .unwrap();
    let today = DayWindow::new("2024-05-01", T0, T0 + DAY).unwrap();
    let fixed = db.generate_daily_assignment(&today).unwrap();

    db.set_problem_status(first.id, ProblemStatus::Paused, T0 + 2)
        .unwrap();
    db.set_problem_status(second.id, ProblemStatus::Archived, T0 + 2)
        .unwrap();
    assert_eq!(db.generate_daily_assignment(&today).unwrap(), fixed);

    let tomorrow = DayWindow::new("2024-05-02", T0 + DAY, T0 + 2 * DAY).unwrap();
    assert!(db
        .generate_daily_assignment(&tomorrow)
        .unwrap()
        .items
        .is_empty());
}

#[test]
fn review_retries_are_idempotent_and_payload_conflicts_are_typed() {
    let mut db = Database::in_memory().unwrap();
    let saved = db
        .upsert_problem(&problem("idempotent", Difficulty::Easy), T0)
        .unwrap();
    let scheduler = FsrsScheduler::default();
    let event = ReviewEvent::new("stable-key", Rating::Medium, T0).unwrap();
    let first = db
        .record_review(saved.id, event.clone(), &scheduler)
        .unwrap();
    let retry = db
        .record_review(saved.id, event.clone(), &scheduler)
        .unwrap();
    assert_eq!(retry, first);
    assert_eq!(db.list_review_events(saved.id).unwrap().len(), 1);

    let conflict = db
        .record_review(
            saved.id,
            ReviewEvent::new("stable-key", Rating::Easy, T0).unwrap(),
            &scheduler,
        )
        .unwrap_err();
    assert!(matches!(
        conflict,
        StorageError::ReviewIdempotencyConflict { .. }
    ));
    assert_eq!(db.list_review_events(saved.id).unwrap().len(), 1);

    let later = db
        .record_review(
            saved.id,
            ReviewEvent::new("later-key", Rating::Hard, T0 + DAY).unwrap(),
            &scheduler,
        )
        .unwrap();
    assert_ne!(later, first);
    let delayed_retry = db.record_review(saved.id, event, &scheduler).unwrap();
    assert_eq!(delayed_retry, first);
    assert_eq!(db.get_schedule(saved.id).unwrap(), Some(later));
    assert_eq!(db.list_review_events(saved.id).unwrap().len(), 2);
}

#[test]
fn chronological_append_is_enforced_and_same_timestamp_order_is_preserved() {
    let mut db = Database::in_memory().unwrap();
    let saved = db
        .upsert_problem(&problem("ordered", Difficulty::Easy), T0)
        .unwrap();
    let scheduler = FsrsScheduler::default();
    db.record_review(
        saved.id,
        ReviewEvent::new("first", Rating::Medium, T0).unwrap(),
        &scheduler,
    )
    .unwrap();
    db.record_review(
        saved.id,
        ReviewEvent::new("second", Rating::Easy, T0).unwrap(),
        &scheduler,
    )
    .unwrap();
    let events = db.list_review_events(saved.id).unwrap();
    assert_eq!(events[0].idempotency_key(), "first");
    assert_eq!(events[1].idempotency_key(), "second");

    let error = db
        .record_review(
            saved.id,
            ReviewEvent::new("older", Rating::Again, T0 - 1).unwrap(),
            &scheduler,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        StorageError::Learning(LearningError::OutOfOrder { .. })
    ));
    assert_eq!(db.list_review_events(saved.id).unwrap().len(), 2);
}

#[test]
fn persisted_day_rejects_different_utc_bounds() {
    let mut db = Database::in_memory().unwrap();
    let original = DayWindow::new("2024-03-10", T0, T0 + 23 * 3_600).unwrap();
    db.generate_daily_assignment(&original).unwrap();
    let mismatched = DayWindow::new("2024-03-10", T0 + 1, T0 + 1 + 23 * 3_600).unwrap();
    let error = db.generate_daily_assignment(&mismatched).unwrap_err();
    assert!(matches!(error, StorageError::DayWindowMismatch { .. }));
}

#[test]
fn removed_is_distinct_from_archived_and_only_affects_future_days() {
    let mut db = Database::in_memory().unwrap();
    let archived = db
        .upsert_problem(&problem("archived", Difficulty::Easy), T0)
        .unwrap();
    let removed = db
        .upsert_problem(&problem("removed", Difficulty::Easy), T0 + 1)
        .unwrap();
    let today = DayWindow::new("2024-06-01", T0, T0 + DAY).unwrap();
    let fixed = db.generate_daily_assignment(&today).unwrap();
    db.record_review(
        removed.id,
        ReviewEvent::new("removed-history", Rating::Medium, T0 + 1).unwrap(),
        &FsrsScheduler::default(),
    )
    .unwrap();
    db.set_problem_status(archived.id, ProblemStatus::Archived, T0 + 2)
        .unwrap();
    db.set_problem_status(removed.id, ProblemStatus::Removed, T0 + 2)
        .unwrap();
    assert_eq!(db.generate_daily_assignment(&today).unwrap(), fixed);
    assert_eq!(
        db.get_problem(archived.id).unwrap().unwrap().status,
        ProblemStatus::Archived
    );
    assert_eq!(
        db.get_problem(removed.id).unwrap().unwrap().status,
        ProblemStatus::Removed
    );
    assert_eq!(db.list_review_events(removed.id).unwrap().len(), 1);
    assert!(db.get_schedule(removed.id).unwrap().is_some());

    let tomorrow = DayWindow::new("2024-06-02", T0 + DAY, T0 + 2 * DAY).unwrap();
    assert!(db
        .generate_daily_assignment(&tomorrow)
        .unwrap()
        .items
        .is_empty());
}

#[test]
fn file_database_reopens_reviews_schedules_and_assignments() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("part2.sqlite");
    let (problem_id, expected_schedule, expected_assignment) = {
        let mut db = Database::open(&path).unwrap();
        db.generate_daily_assignment(
            &DayWindow::new("2024-07-02", T0 + 101 * DAY, T0 + 102 * DAY).unwrap(),
        )
        .unwrap();
        let saved = db
            .upsert_problem(&problem("persistent", Difficulty::Easy), T0)
            .unwrap();
        let schedule = db
            .record_review(
                saved.id,
                ReviewEvent::new("persistent-review", Rating::Medium, T0).unwrap(),
                &FsrsScheduler::default(),
            )
            .unwrap();
        let assignment = db
            .generate_daily_assignment(
                &DayWindow::new("2024-07-01", T0 + 100 * DAY, T0 + 101 * DAY).unwrap(),
            )
            .unwrap();
        (saved.id, schedule, assignment)
    };

    let mut reopened = Database::open(&path).unwrap();
    reopened.run_migrations().unwrap();
    assert_eq!(reopened.schema_migration_count().unwrap(), 4);
    assert_eq!(
        reopened.list_review_events(problem_id).unwrap()[0].idempotency_key(),
        "persistent-review"
    );
    assert_eq!(
        reopened.get_schedule(problem_id).unwrap(),
        Some(expected_schedule)
    );
    assert_eq!(
        reopened.load_daily_assignment("2024-07-01").unwrap(),
        Some(expected_assignment)
    );
    assert!(reopened
        .load_daily_assignment("2024-07-02")
        .unwrap()
        .unwrap()
        .items
        .is_empty());
}

#[test]
fn migration_checksum_drift_is_rejected() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("drift.sqlite");
    drop(Database::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE schema_migrations SET checksum = 'tampered' WHERE version = '0001_part2'",
            [],
        )
        .unwrap();
    let error = match Database::open(&path) {
        Ok(_) => panic!("checksum drift was accepted"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        StorageError::MigrationChecksumMismatch { .. }
    ));
}

#[test]
fn file_schema_rejects_mutation_and_assignment_corruption() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("constraints.sqlite");
    let (easy_id, medium_id) = {
        let mut db = Database::open(&path).unwrap();
        let easy = db
            .upsert_problem(&problem("easy-direct", Difficulty::Easy), T0)
            .unwrap();
        let medium = db
            .upsert_problem(&problem("medium-direct", Difficulty::Medium), T0 + 1)
            .unwrap();
        db.generate_daily_assignment(
            &DayWindow::new("2024-08-01", T0 + 200 * DAY, T0 + 201 * DAY).unwrap(),
        )
        .unwrap();
        db.record_review(
            easy.id,
            ReviewEvent::new("immutable-review", Rating::Medium, T0).unwrap(),
            &FsrsScheduler::default(),
        )
        .unwrap();
        (easy.id, medium.id)
    };

    let connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    assert!(connection
        .execute("UPDATE review_events SET rating = 4", [])
        .is_err());
    assert!(connection.execute("DELETE FROM review_events", []).is_err());
    assert!(connection
        .execute("UPDATE daily_assignments SET cost = 2", [])
        .is_err());
    assert!(connection
        .execute("DELETE FROM daily_assignments", [])
        .is_err());
    assert!(connection
        .execute("UPDATE daily_queue_generations SET day_start_utc = 1", [])
        .is_err());
    assert!(connection
        .execute("DELETE FROM daily_queue_generations", [])
        .is_err());

    connection
        .execute(
            "INSERT INTO daily_queue_generations VALUES ('2024-08-02', ?1, ?2)",
            params![T0 + 201 * DAY, T0 + 202 * DAY],
        )
        .unwrap();
    assert!(connection
        .execute(
            "INSERT INTO daily_assignments VALUES ('2024-08-02', ?1, 0, 1)",
            [medium_id],
        )
        .is_err());
    assert!(connection
        .execute(
            "INSERT INTO daily_assignments VALUES ('2024-08-02', ?1, 1, 1)",
            [easy_id],
        )
        .is_err());

    connection
        .execute(
            "INSERT INTO daily_queue_generations VALUES ('2024-08-03', ?1, ?2)",
            params![T0 + 202 * DAY, T0 + 203 * DAY],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO daily_assignments VALUES ('2024-08-03', ?1, 0, 2)",
            [medium_id],
        )
        .unwrap();
    assert!(connection
        .execute(
            "INSERT INTO daily_assignments VALUES ('2024-08-03', ?1, 1, 1)",
            [easy_id],
        )
        .is_err());
}
