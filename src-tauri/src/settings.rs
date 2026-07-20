//! Application settings and pairing-code helpers.

use chrono_tz::Tz;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_TIMEZONE: &str = "America/New_York";
pub const DEFAULT_RETENTION: f64 = 0.9;
pub const PAIRING_CODE_LEN: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub timezone_id: String,
    pub desired_retention: f64,
    pub onboarding_completed: bool,
    pub pairing_code: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsUpdate {
    pub timezone_id: String,
    pub desired_retention: f64,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SettingsError {
    #[error("invalid IANA timezone: {0}")]
    InvalidTimeZone(String),
    #[error("desired retention must be finite and strictly between 0 and 1")]
    InvalidRetention,
    #[error("pairing code must be non-empty")]
    EmptyPairingCode,
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), SettingsError> {
        validate_timezone(&self.timezone_id)?;
        validate_retention(self.desired_retention)?;
        if self.pairing_code.trim().is_empty() {
            return Err(SettingsError::EmptyPairingCode);
        }
        Ok(())
    }
}

impl SettingsUpdate {
    pub fn validate(&self) -> Result<(), SettingsError> {
        validate_timezone(&self.timezone_id)?;
        validate_retention(self.desired_retention)?;
        Ok(())
    }
}

pub fn validate_timezone(timezone_id: &str) -> Result<(), SettingsError> {
    timezone_id
        .parse::<Tz>()
        .map(|_| ())
        .map_err(|_| SettingsError::InvalidTimeZone(timezone_id.to_owned()))
}

pub fn validate_retention(desired_retention: f64) -> Result<(), SettingsError> {
    if !desired_retention.is_finite() || desired_retention <= 0.0 || desired_retention >= 1.0 {
        return Err(SettingsError::InvalidRetention);
    }
    Ok(())
}

/// Generates an 8-character uppercase alphanumeric pairing code using OS entropy.
pub fn generate_pairing_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..PAIRING_CODE_LEN)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_code_is_eight_uppercase_alnum() {
        let code = generate_pairing_code();
        assert_eq!(code.len(), 8);
        assert!(code
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()));
    }

    #[test]
    fn retention_and_timezone_validation() {
        assert!(validate_retention(0.9).is_ok());
        assert!(validate_retention(0.0).is_err());
        assert!(validate_timezone("America/New_York").is_ok());
        assert!(validate_timezone("Not/A_Zone").is_err());
    }
}
