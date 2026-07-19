//! Problem identity, metadata, and lifecycle boundary.

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
        }
    }

    pub const fn queue_cost(self) -> Option<u8> {
        match self {
            Self::Easy => Some(1),
            Self::Medium => Some(2),
            Self::Hard => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProblemStatus {
    Active,
    Paused,
    Archived,
    Removed,
}

impl ProblemStatus {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Archived => "archived",
            Self::Removed => "removed",
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProblemError {
    #[error("{0} must not be empty")]
    EmptyField(&'static str),
    #[error("slug contains unsupported characters")]
    InvalidSlug,
    #[error("URL must be the canonical LeetCode URL for the slug")]
    InvalidUrl,
    #[error("invalid {kind} database value: {value}")]
    InvalidDatabaseValue { kind: &'static str, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewProblem {
    pub slug: String,
    pub title: String,
    pub url: String,
    pub difficulty: Difficulty,
}

impl NewProblem {
    pub fn new(
        slug: impl Into<String>,
        title: impl Into<String>,
        url: impl Into<String>,
        difficulty: Difficulty,
    ) -> Result<Self, ProblemError> {
        let value = Self {
            slug: slug.into(),
            title: title.into(),
            url: url.into(),
            difficulty,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), ProblemError> {
        if self.slug.trim().is_empty() {
            return Err(ProblemError::EmptyField("slug"));
        }
        if self.title.trim().is_empty() {
            return Err(ProblemError::EmptyField("title"));
        }
        if !self
            .slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(ProblemError::InvalidSlug);
        }
        let canonical = format!("https://leetcode.com/problems/{}/", self.slug);
        if self.url != canonical {
            return Err(ProblemError::InvalidUrl);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub url: String,
    pub difficulty: Difficulty,
    pub status: ProblemStatus,
    /// First-added instant in UTC epoch seconds.
    pub added_at: i64,
    /// Lifecycle metadata update instant in UTC epoch seconds.
    pub updated_at: i64,
}

macro_rules! impl_db_text {
    ($type:ty, $kind:literal, {$($variant:ident => $text:literal),+ $(,)?}) => {
        impl FromStr for $type {
            type Err = ProblemError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(ProblemError::InvalidDatabaseValue {
                        kind: $kind,
                        value: value.to_owned(),
                    }),
                }
            }
        }

        impl fmt::Display for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_db_str())
            }
        }
    };
}

impl_db_text!(Difficulty, "difficulty", {
    Easy => "easy",
    Medium => "medium",
    Hard => "hard",
});
impl_db_text!(ProblemStatus, "problem status", {
    Active => "active",
    Paused => "paused",
    Archived => "archived",
    Removed => "removed",
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_rejects_empty_and_noncanonical_values() {
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
}
