use serde::{Deserialize, Serialize};
use std::fmt;

/// Identity for a cron job callback.
/// Validated to contain only alphanumeric characters, dashes, and underscores.
/// Maximum length: 128 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identity(String);

impl Identity {
    /// Creates a new Identity if the input is valid.
    /// Valid identities contain only [a-zA-Z0-9_-] and are 1-128 characters long.
    pub fn new(s: impl Into<String>) -> Result<Self, IdentityError> {
        let s = s.into();

        if s.is_empty() {
            return Err(IdentityError::Empty);
        }

        if s.len() > 128 {
            return Err(IdentityError::TooLong(s.len()));
        }

        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(IdentityError::InvalidCharacters);
        }

        Ok(Self(s))
    }

    /// Returns the identity as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the Identity and returns the inner String.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Identity {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Errors that can occur when creating an Identity.
#[derive(Debug, Clone, thiserror::Error)]
pub enum IdentityError {
    #[error("Identity cannot be empty")]
    Empty,
    #[error("Identity is too long: {0} characters (max 128)")]
    TooLong(usize),
    #[error("Identity contains invalid characters (only a-zA-Z0-9_- allowed)")]
    InvalidCharacters,
}

/// Record of callback data for a specific identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackRecord {
    /// UTC Unix timestamp of the last callback.
    pub last_callback_ts: i64,
    /// Total number of callbacks received for this identity.
    pub callback_count: u64,
}

impl CallbackRecord {
    /// Creates a new CallbackRecord with the given timestamp and count 1.
    pub fn new(timestamp: i64) -> Self {
        Self {
            last_callback_ts: timestamp,
            callback_count: 1,
        }
    }

    /// Updates the record with a new callback timestamp.
    pub fn record_callback(&mut self, timestamp: i64) {
        self.last_callback_ts = timestamp;
        self.callback_count = self.callback_count.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_valid() {
        assert!(Identity::new("clickhouse-backup-prod").is_ok());
        assert!(Identity::new("pg_backup_staging").is_ok());
        assert!(Identity::new("backup123").is_ok());
        assert!(Identity::new("a").is_ok());
    }

    #[test]
    fn test_identity_empty() {
        assert!(matches!(Identity::new(""), Err(IdentityError::Empty)));
    }

    #[test]
    fn test_identity_too_long() {
        let long_name = "a".repeat(129);
        assert!(matches!(
            Identity::new(long_name),
            Err(IdentityError::TooLong(_))
        ));
    }

    #[test]
    fn test_identity_invalid_chars() {
        assert!(matches!(
            Identity::new("backup/prod"),
            Err(IdentityError::InvalidCharacters)
        ));
        assert!(matches!(
            Identity::new("backup prod"),
            Err(IdentityError::InvalidCharacters)
        ));
        assert!(matches!(
            Identity::new("backup@prod"),
            Err(IdentityError::InvalidCharacters)
        ));
    }

    #[test]
    fn test_callback_record_new() {
        let record = CallbackRecord::new(1234567890);
        assert_eq!(record.last_callback_ts, 1234567890);
        assert_eq!(record.callback_count, 1);
    }

    #[test]
    fn test_callback_record_update() {
        let mut record = CallbackRecord::new(1000);
        record.record_callback(2000);
        assert_eq!(record.last_callback_ts, 2000);
        assert_eq!(record.callback_count, 2);

        record.record_callback(3000);
        assert_eq!(record.last_callback_ts, 3000);
        assert_eq!(record.callback_count, 3);
    }
}
