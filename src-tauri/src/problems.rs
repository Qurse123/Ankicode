//! Problem identity, metadata, and lifecycle boundary.

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

const LEETCODE_PROBLEM_PREFIX: &str = "https://leetcode.com/problems/";

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

    /// Builds a problem from a pasted LeetCode URL, optional title, and difficulty.
    pub fn from_url(
        raw_url: &str,
        title: Option<&str>,
        difficulty: Difficulty,
    ) -> Result<Self, ProblemError> {
        let (slug, url) = parse_leetcode_problem_url(raw_url)?;
        let title = title
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| title_from_slug(&slug));
        Self::new(slug, title, url, difficulty)
    }

    pub fn validate(&self) -> Result<(), ProblemError> {
        if self.slug.trim().is_empty() {
            return Err(ProblemError::EmptyField("slug"));
        }
        if self.title.trim().is_empty() {
            return Err(ProblemError::EmptyField("title"));
        }
        if !is_valid_slug(&self.slug) {
            return Err(ProblemError::InvalidSlug);
        }
        let canonical = canonical_problem_url(&self.slug);
        if self.url != canonical {
            return Err(ProblemError::InvalidUrl);
        }
        Ok(())
    }
}

/// Parses a pasted LeetCode problem URL into `(slug, canonical_url)`.
pub fn parse_leetcode_problem_url(raw_url: &str) -> Result<(String, String), ProblemError> {
    let trimmed = raw_url.trim();
    let without_query = trimmed.split(['?', '#']).next().unwrap_or(trimmed);
    let rest = without_query
        .strip_prefix(LEETCODE_PROBLEM_PREFIX)
        .or_else(|| without_query.strip_prefix("http://leetcode.com/problems/"))
        .ok_or(ProblemError::InvalidUrl)?;
    let slug = rest
        .trim_matches('/')
        .split('/')
        .next()
        .unwrap_or("")
        .trim();
    if slug.is_empty() || !is_valid_slug(slug) {
        return Err(ProblemError::InvalidSlug);
    }
    Ok((slug.to_owned(), canonical_problem_url(slug)))
}

pub fn canonical_problem_url(slug: &str) -> String {
    format!("{LEETCODE_PROBLEM_PREFIX}{slug}/")
}

pub fn title_from_slug(slug: &str) -> String {
    slug.replace('-', " ")
}

fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
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

    #[test]
    fn url_parser_normalizes_canonical_form() {
        let (slug, url) =
            parse_leetcode_problem_url("https://leetcode.com/problems/two-sum/?tab=description")
                .unwrap();
        assert_eq!(slug, "two-sum");
        assert_eq!(url, "https://leetcode.com/problems/two-sum/");
        let problem = NewProblem::from_url(
            "https://leetcode.com/problems/two-sum",
            None,
            Difficulty::Easy,
        )
        .unwrap();
        assert_eq!(problem.title, "two sum");
        assert!(parse_leetcode_problem_url("https://example.com/problems/x/").is_err());
    }
}
