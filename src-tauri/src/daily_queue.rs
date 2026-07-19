//! Deterministic persisted daily assignment boundary.

use chrono::{DateTime, FixedOffset, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DAILY_BUDGET: u8 = 2;
const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DayWindowError {
    #[error("local date must be ISO YYYY-MM-DD")]
    InvalidDate,
    #[error("day window must have positive bounds and span 23, 24, or 25 hours")]
    InvalidBounds,
    #[error("fixed offset is outside the supported range")]
    InvalidOffset,
    #[error("UTC timestamp is outside the supported range")]
    InvalidTimestamp,
    #[error("invalid IANA timezone: {0}")]
    InvalidTimeZone(String),
    #[error("local midnight is ambiguous in timezone {0}")]
    AmbiguousBoundary(String),
    #[error("local midnight does not exist in timezone {0}")]
    NonexistentBoundary(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayWindow {
    local_date: String,
    start_utc: i64,
    end_utc: i64,
}

impl DayWindow {
    /// Validates an ISO local date and its UTC epoch-second civil-day bounds.
    pub fn new(
        local_date: impl Into<String>,
        start_utc: i64,
        end_utc: i64,
    ) -> Result<Self, DayWindowError> {
        let local_date = local_date.into();
        let parsed = NaiveDate::parse_from_str(&local_date, "%Y-%m-%d")
            .map_err(|_| DayWindowError::InvalidDate)?;
        if parsed.format("%Y-%m-%d").to_string() != local_date {
            return Err(DayWindowError::InvalidDate);
        }
        let duration = end_utc.checked_sub(start_utc);
        // Civil days may be 23h, 23.5h, 24h, 24.5h, or 25h across DST transitions.
        if start_utc <= 0
            || end_utc <= 0
            || !matches!(duration, Some(82_800 | 84_600 | 86_400 | 88_200 | 90_000))
        {
            return Err(DayWindowError::InvalidBounds);
        }
        Ok(Self {
            local_date,
            start_utc,
            end_utc,
        })
    }

    /// Builds a civil day for a fixed UTC offset. Prefer [`Self::from_local_date`]
    /// for IANA zones that observe DST.
    pub fn from_utc_timestamp(timestamp: i64, offset_minutes: i32) -> Result<Self, DayWindowError> {
        let offset_seconds = offset_minutes
            .checked_mul(60)
            .ok_or(DayWindowError::InvalidOffset)?;
        let offset = FixedOffset::east_opt(offset_seconds).ok_or(DayWindowError::InvalidOffset)?;
        let utc = DateTime::<Utc>::from_timestamp(timestamp, 0)
            .ok_or(DayWindowError::InvalidTimestamp)?;
        let local_date = utc.with_timezone(&offset).date_naive();
        let midnight = local_date
            .and_hms_opt(0, 0, 0)
            .ok_or(DayWindowError::InvalidTimestamp)?;
        let start_utc = offset
            .from_local_datetime(&midnight)
            .single()
            .ok_or(DayWindowError::InvalidTimestamp)?
            .timestamp();
        let end_utc = start_utc
            .checked_add(SECONDS_PER_DAY)
            .ok_or(DayWindowError::InvalidTimestamp)?;
        Self::new(
            local_date.format("%Y-%m-%d").to_string(),
            start_utc,
            end_utc,
        )
    }

    /// Computes consecutive local midnights for an IANA timezone.
    pub fn from_local_date(local_date: &str, timezone_id: &str) -> Result<Self, DayWindowError> {
        let date = NaiveDate::parse_from_str(local_date, "%Y-%m-%d")
            .map_err(|_| DayWindowError::InvalidDate)?;
        if date.format("%Y-%m-%d").to_string() != local_date {
            return Err(DayWindowError::InvalidDate);
        }
        let timezone = timezone_id
            .parse::<Tz>()
            .map_err(|_| DayWindowError::InvalidTimeZone(timezone_id.to_owned()))?;
        let next_date = date.succ_opt().ok_or(DayWindowError::InvalidTimestamp)?;
        let start_utc = local_midnight(&timezone, date, timezone_id)?.timestamp();
        let end_utc = local_midnight(&timezone, next_date, timezone_id)?.timestamp();
        Self::new(local_date, start_utc, end_utc)
    }

    pub fn local_date(&self) -> &str {
        &self.local_date
    }

    pub const fn start_utc(&self) -> i64 {
        self.start_utc
    }

    pub const fn end_utc(&self) -> i64 {
        self.end_utc
    }
}

fn local_midnight(
    timezone: &Tz,
    date: NaiveDate,
    timezone_id: &str,
) -> Result<DateTime<Tz>, DayWindowError> {
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or(DayWindowError::InvalidTimestamp)?;
    match timezone.from_local_datetime(&midnight) {
        LocalResult::Single(value) => Ok(value),
        LocalResult::Ambiguous(_, _) => {
            Err(DayWindowError::AmbiguousBoundary(timezone_id.to_owned()))
        }
        LocalResult::None => Err(DayWindowError::NonexistentBoundary(timezone_id.to_owned())),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyAssignmentItem {
    pub problem_id: i64,
    pub position: u8,
    pub cost: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyAssignment {
    pub local_date: String,
    pub items: Vec<DailyAssignmentItem>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AssignmentError {
    #[error("assignment position {actual} must be contiguous at {expected}")]
    NoncontiguousPosition { expected: u8, actual: u8 },
    #[error("assignment cost must be 1 or 2, got {0}")]
    InvalidCost(u8),
    #[error("assignment cost {0} exceeds the daily budget")]
    BudgetExceeded(u32),
}

impl DailyAssignment {
    /// Returns the assignment's cost without narrow-integer overflow.
    pub fn total_cost(&self) -> u32 {
        self.items.iter().map(|item| u32::from(item.cost)).sum()
    }

    /// Validates persisted ordering, legal costs, and the daily budget.
    pub fn validate(&self) -> Result<(), AssignmentError> {
        for (index, item) in self.items.iter().enumerate() {
            let expected =
                u8::try_from(index).map_err(|_| AssignmentError::BudgetExceeded(u32::MAX))?;
            if item.position != expected {
                return Err(AssignmentError::NoncontiguousPosition {
                    expected,
                    actual: item.position,
                });
            }
            if !matches!(item.cost, 1 | 2) {
                return Err(AssignmentError::InvalidCost(item.cost));
            }
        }
        let cost = self.total_cost();
        if cost > u32::from(DAILY_BUDGET) {
            return Err(AssignmentError::BudgetExceeded(cost));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_offset_day_window_handles_local_boundaries() {
        let before_midnight_utc = 1_704_067_199;
        let west = DayWindow::from_utc_timestamp(before_midnight_utc, -300).unwrap();
        assert_eq!(west.local_date(), "2023-12-31");
        assert_eq!(west.end_utc() - west.start_utc(), SECONDS_PER_DAY);
        assert!(west.start_utc() <= before_midnight_utc);
        assert!(before_midnight_utc < west.end_utc());

        let east = DayWindow::from_utc_timestamp(before_midnight_utc, 330).unwrap();
        assert_eq!(east.local_date(), "2024-01-01");
    }

    #[test]
    fn total_cost_does_not_overflow() {
        let assignment = DailyAssignment {
            local_date: "2024-01-01".to_owned(),
            items: (0..300)
                .map(|position| DailyAssignmentItem {
                    problem_id: i64::from(position),
                    position: u8::try_from(position % 256).unwrap(),
                    cost: 2,
                })
                .collect(),
        };
        assert_eq!(assignment.total_cost(), 600);
    }

    #[test]
    fn total_cost_exceeding_u16_is_safe() {
        let assignment = DailyAssignment {
            local_date: "2024-01-01".to_owned(),
            items: (0..40_000)
                .map(|position| DailyAssignmentItem {
                    problem_id: i64::from(position),
                    position: u8::try_from(position % 256).unwrap(),
                    cost: 2,
                })
                .collect(),
        };
        assert_eq!(assignment.total_cost(), 80_000);
    }

    #[test]
    fn civil_window_accepts_23_24_and_25_hour_days_only() {
        assert!(DayWindow::new("2024-03-10", 1, 1 + 23 * 3_600).is_ok());
        assert!(DayWindow::new("2024-03-11", 1, 1 + 24 * 3_600).is_ok());
        assert!(DayWindow::new("2024-11-03", 1, 1 + 25 * 3_600).is_ok());
        assert!(DayWindow::new("2024-01-01", 0, 24 * 3_600).is_err());
        assert!(DayWindow::new("2024-01-01", 1, 1 + 22 * 3_600).is_err());
        assert!(DayWindow::new("2024-01-01", 1, 1 + 26 * 3_600).is_err());
    }

    #[test]
    fn iana_timezone_constructor_handles_dst_boundaries() {
        let spring = DayWindow::from_local_date("2024-03-10", "America/New_York").unwrap();
        assert_eq!(spring.end_utc() - spring.start_utc(), 23 * 3_600);

        let fall = DayWindow::from_local_date("2024-11-03", "America/New_York").unwrap();
        assert_eq!(fall.end_utc() - fall.start_utc(), 25 * 3_600);

        assert!(matches!(
            DayWindow::from_local_date("2024-03-10", "Not/A_Zone"),
            Err(DayWindowError::InvalidTimeZone(_))
        ));
    }

    #[test]
    fn half_hour_dst_windows_are_supported() {
        let spring = DayWindow::from_local_date("2024-10-06", "Australia/Lord_Howe").unwrap();
        assert_eq!(spring.end_utc() - spring.start_utc(), 23 * 3_600 + 1_800);

        let fall = DayWindow::from_local_date("2024-04-07", "Australia/Lord_Howe").unwrap();
        assert_eq!(fall.end_utc() - fall.start_utc(), 24 * 3_600 + 1_800);

        assert!(DayWindow::new("2024-10-06", 1, 1 + 23 * 3_600 + 1_800).is_ok());
        assert!(DayWindow::new("2024-04-07", 1, 1 + 24 * 3_600 + 1_800).is_ok());
        assert!(DayWindow::new("2024-10-06", 1, 1 + 23 * 3_600 + 1).is_err());
    }

    #[test]
    fn assignment_validation_enforces_positions_costs_and_budget() {
        let assignment = DailyAssignment {
            local_date: "2024-01-01".to_owned(),
            items: vec![
                DailyAssignmentItem {
                    problem_id: 1,
                    position: 0,
                    cost: 1,
                },
                DailyAssignmentItem {
                    problem_id: 2,
                    position: 2,
                    cost: 2,
                },
            ],
        };
        assert!(matches!(
            assignment.validate(),
            Err(AssignmentError::NoncontiguousPosition { .. })
        ));

        let illegal_cost = DailyAssignment {
            local_date: "2024-01-01".to_owned(),
            items: vec![DailyAssignmentItem {
                problem_id: 1,
                position: 0,
                cost: 3,
            }],
        };
        assert!(matches!(
            illegal_cost.validate(),
            Err(AssignmentError::InvalidCost(3))
        ));

        let over_budget = DailyAssignment {
            local_date: "2024-01-01".to_owned(),
            items: vec![
                DailyAssignmentItem {
                    problem_id: 1,
                    position: 0,
                    cost: 1,
                },
                DailyAssignmentItem {
                    problem_id: 2,
                    position: 1,
                    cost: 2,
                },
            ],
        };
        assert!(matches!(
            over_budget.validate(),
            Err(AssignmentError::BudgetExceeded(3))
        ));
    }
}
