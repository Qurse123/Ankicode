//! Review events and deterministic FSRS projection boundary.

use fsrs::{FSRSItem, FSRSReview, FSRS};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Again,
    Hard,
    /// FSRS grade 3 (Anki “Good”); labeled Medium in the product UI.
    #[serde(alias = "good")]
    Medium,
    Easy,
}

impl Rating {
    pub const fn fsrs_value(self) -> u32 {
        match self {
            Self::Again => 1,
            Self::Hard => 2,
            Self::Medium => 3,
            Self::Easy => 4,
        }
    }

    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Again => "again",
            Self::Hard => "hard",
            Self::Medium => "medium",
            Self::Easy => "easy",
        }
    }
}

impl TryFrom<u32> for Rating {
    type Error = LearningError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Again),
            2 => Ok(Self::Hard),
            3 => Ok(Self::Medium),
            4 => Ok(Self::Easy),
            _ => Err(LearningError::InvalidRating(value.to_string())),
        }
    }
}

impl FromStr for Rating {
    type Err = LearningError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "again" => Ok(Self::Again),
            "hard" => Ok(Self::Hard),
            // Accept legacy backup/export spelling.
            "medium" | "good" => Ok(Self::Medium),
            "easy" => Ok(Self::Easy),
            _ => Err(LearningError::InvalidRating(value.to_owned())),
        }
    }
}

impl fmt::Display for Rating {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_db_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewEvent {
    idempotency_key: String,
    rating: Rating,
    reviewed_at: i64,
}

impl ReviewEvent {
    /// Creates a review identified by a stable, non-empty retry key.
    pub fn new(
        idempotency_key: impl Into<String>,
        rating: Rating,
        reviewed_at: i64,
    ) -> Result<Self, LearningError> {
        let event = Self {
            idempotency_key: idempotency_key.into(),
            rating,
            reviewed_at,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> Result<(), LearningError> {
        if self.idempotency_key.trim().is_empty() {
            return Err(LearningError::InvalidIdempotencyKey);
        }
        Ok(())
    }

    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    pub const fn rating(&self) -> Rating {
        self.rating
    }

    /// UTC epoch seconds at which the review occurred.
    pub const fn reviewed_at(&self) -> i64 {
        self.reviewed_at
    }
}

impl<'de> Deserialize<'de> for ReviewEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawReviewEvent {
            idempotency_key: String,
            rating: Rating,
            reviewed_at: i64,
        }

        let raw = RawReviewEvent::deserialize(deserializer)?;
        Self::new(raw.idempotency_key, raw.rating, raw.reviewed_at).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleState {
    pub stability: f32,
    pub difficulty: f32,
    /// Next due instant in UTC epoch seconds.
    pub due_at: i64,
    /// Most recent review instant in UTC epoch seconds.
    pub last_review_at: i64,
}

#[derive(Debug, Error)]
pub enum LearningError {
    #[error("desired retention must be finite and strictly between 0 and 1")]
    InvalidRetention,
    #[error("invalid rating: {0}")]
    InvalidRating(String),
    #[error("review idempotency key must not be empty")]
    InvalidIdempotencyKey,
    #[error("review at {current} precedes review at {previous}")]
    OutOfOrder { previous: i64, current: i64 },
    #[error("elapsed day count exceeds FSRS limits")]
    ElapsedDaysOverflow,
    #[error("FSRS projection failed: {0}")]
    Fsrs(#[from] fsrs::FSRSError),
    #[error("FSRS returned a non-finite interval")]
    NonFiniteInterval,
    #[error("due timestamp overflow")]
    DueTimestampOverflow,
}

pub struct FsrsScheduler {
    model: FSRS,
    desired_retention: f32,
}

impl Default for FsrsScheduler {
    fn default() -> Self {
        Self {
            model: FSRS::default(),
            desired_retention: 0.9,
        }
    }
}

impl FsrsScheduler {
    /// Creates a scheduler with retention strictly between zero and one.
    pub fn new(desired_retention: f32) -> Result<Self, LearningError> {
        if !desired_retention.is_finite() || desired_retention <= 0.0 || desired_retention >= 1.0 {
            return Err(LearningError::InvalidRetention);
        }
        Ok(Self {
            model: FSRS::default(),
            desired_retention,
        })
    }

    pub const fn desired_retention(&self) -> f32 {
        self.desired_retention
    }

    /// Converts append-ordered events to FSRS whole-UTC-day deltas.
    pub fn to_fsrs_item(&self, events: &[ReviewEvent]) -> Result<FSRSItem, LearningError> {
        let mut previous = None;
        let mut reviews = Vec::with_capacity(events.len());
        for event in events {
            let delta_t = if let Some(previous_at) = previous {
                if event.reviewed_at < previous_at {
                    return Err(LearningError::OutOfOrder {
                        previous: previous_at,
                        current: event.reviewed_at,
                    });
                }
                let elapsed = event
                    .reviewed_at
                    .checked_sub(previous_at)
                    .ok_or(LearningError::ElapsedDaysOverflow)?;
                u32::try_from(elapsed / SECONDS_PER_DAY)
                    .map_err(|_| LearningError::ElapsedDaysOverflow)?
            } else {
                0
            };
            reviews.push(FSRSReview {
                rating: event.rating.fsrs_value(),
                delta_t,
            });
            previous = Some(event.reviewed_at);
        }
        Ok(FSRSItem { reviews })
    }

    /// Replays immutable events and returns memory plus a UTC due timestamp.
    pub fn project(&self, events: &[ReviewEvent]) -> Result<Option<ScheduleState>, LearningError> {
        let Some(last) = events.last() else {
            return Ok(None);
        };
        let item = self.to_fsrs_item(events)?;
        let memory = self.model.memory_state(item, None)?;
        let interval = self.model.next_interval(
            Some(memory.stability),
            self.desired_retention,
            last.rating.fsrs_value(),
        );
        if !interval.is_finite() {
            return Err(LearningError::NonFiniteInterval);
        }
        let rounded_days = interval.round().max(1.0);
        if rounded_days > i64::MAX as f32 {
            return Err(LearningError::DueTimestampOverflow);
        }
        let seconds = (rounded_days as i64)
            .checked_mul(SECONDS_PER_DAY)
            .ok_or(LearningError::DueTimestampOverflow)?;
        let due_at = last
            .reviewed_at
            .checked_add(seconds)
            .ok_or(LearningError::DueTimestampOverflow)?;
        Ok(Some(ScheduleState {
            stability: memory.stability,
            difficulty: memory.difficulty,
            due_at,
            last_review_at: last.reviewed_at,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;
    const T0: i64 = 1_700_000_000;

    fn review(rating: Rating, reviewed_at: i64) -> ReviewEvent {
        ReviewEvent {
            idempotency_key: format!("{}-{reviewed_at}", rating.as_db_str()),
            rating,
            reviewed_at,
        }
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
        assert_eq!("good".parse::<Rating>().unwrap(), Rating::Medium);
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
    fn elapsed_timestamp_overflow_is_typed() {
        let scheduler = FsrsScheduler::default();
        let error = scheduler
            .to_fsrs_item(&[
                review(Rating::Medium, i64::MIN),
                review(Rating::Easy, i64::MAX),
            ])
            .unwrap_err();
        assert!(matches!(error, LearningError::ElapsedDaysOverflow));
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
    fn fsrs_6_6_1_projection_vector_is_stable() {
        let state = FsrsScheduler::default()
            .project(&[
                review(Rating::Medium, T0),
                review(Rating::Hard, T0 + 3 * DAY),
                review(Rating::Easy, T0 + 8 * DAY),
            ])
            .unwrap()
            .unwrap();
        let due_days = (state.due_at - state.last_review_at) / DAY;
        assert!((state.stability - 34.406_27).abs() < 0.0001);
        assert!((state.difficulty - 2.984_736_2).abs() < 0.0001);
        assert_eq!(due_days, 34);
    }

    #[test]
    fn review_event_requires_nonempty_idempotency_key() {
        assert!(ReviewEvent::new("", Rating::Medium, T0).is_err());
        assert!(ReviewEvent::new("   ", Rating::Medium, T0).is_err());
        let event = ReviewEvent::new("review-1", Rating::Medium, T0).unwrap();
        assert_eq!(event.idempotency_key(), "review-1");
    }

    #[test]
    fn deserialization_cannot_bypass_review_key_validation() {
        let json = format!(r#"{{"idempotency_key":" ","rating":"medium","reviewed_at":{T0}}}"#);
        assert!(serde_json::from_str::<ReviewEvent>(&json).is_err());
    }
}
