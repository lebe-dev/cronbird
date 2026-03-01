use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind the HTTP server to.
    pub listen_addr: SocketAddr,

    /// Predefined allowed identities.
    /// If empty and allow_dynamic is false, no callbacks will be accepted.
    pub identities: Vec<String>,

    /// Whether to accept unknown identities dynamically.
    pub allow_dynamic: bool,

    /// Optional Bearer token for authentication.
    /// If None, authentication is disabled.
    pub auth_token: Option<String>,

    /// Whether to require authentication for metrics endpoints.
    /// Only effective when auth_token is also set.
    pub protect_metrics: bool,

    /// Path to the state persistence file.
    pub persist_path: PathBuf,

    /// Interval in seconds between persistence writes.
    pub persist_interval: u64,

    /// Log level for tracing.
    pub log_level: String,
}

impl Config {
    /// Loads configuration from environment variables.
    /// Uses default values for any missing variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let listen_addr = std::env::var("CRONBIRD_LISTEN")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse()
            .map_err(|e| ConfigError::InvalidListenAddr(format!("{}", e)))?;

        let identities_str = std::env::var("CRONBIRD_IDENTITIES").unwrap_or_default();
        let identities: Vec<String> = identities_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let allow_dynamic = std::env::var("CRONBIRD_ALLOW_DYNAMIC")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let auth_token = std::env::var("CRONBIRD_AUTH_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        let protect_metrics = std::env::var("CRONBIRD_PROTECT_METRICS")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let persist_path = std::env::var("CRONBIRD_PERSIST_PATH")
            .unwrap_or_else(|_| "./cronbird-state.json".to_string())
            .into();

        let persist_interval = std::env::var("CRONBIRD_PERSIST_INTERVAL")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);

        let log_level = std::env::var("CRONBIRD_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            listen_addr,
            identities,
            allow_dynamic,
            auth_token,
            protect_metrics,
            persist_path,
            persist_interval,
            log_level,
        })
    }

    /// Returns the allowed identities as a HashSet, or None if dynamic mode is enabled.
    pub fn allowed_identities(&self) -> Option<HashSet<String>> {
        if self.allow_dynamic {
            None
        } else {
            Some(self.identities.iter().cloned().collect())
        }
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.allow_dynamic && self.identities.is_empty() {
            return Err(ConfigError::NoIdentitiesConfigured);
        }

        if self.persist_interval == 0 {
            return Err(ConfigError::InvalidPersistInterval);
        }

        Ok(())
    }
}

/// Errors that can occur during configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid listen address: {0}")]
    InvalidListenAddr(String),

    #[error("No identities configured and ALLOW_DYNAMIC is false")]
    NoIdentitiesConfigured,

    #[error("Persist interval must be greater than 0")]
    InvalidPersistInterval,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_identities_dynamic_mode() {
        let config = Config {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            identities: vec!["job1".to_string(), "job2".to_string()],
            allow_dynamic: true,
            auth_token: None,
            protect_metrics: false,
            persist_path: PathBuf::from("./state.json"),
            persist_interval: 60,
            log_level: "info".to_string(),
        };

        assert!(config.allowed_identities().is_none());
    }

    #[test]
    fn test_allowed_identities_static_mode() {
        let config = Config {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            identities: vec!["job1".to_string(), "job2".to_string()],
            allow_dynamic: false,
            auth_token: None,
            protect_metrics: false,
            persist_path: PathBuf::from("./state.json"),
            persist_interval: 60,
            log_level: "info".to_string(),
        };

        let allowed = config.allowed_identities().unwrap();
        assert_eq!(allowed.len(), 2);
        assert!(allowed.contains("job1"));
        assert!(allowed.contains("job2"));
    }

    #[test]
    fn test_validate_no_identities_no_dynamic() {
        let config = Config {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            identities: vec![],
            allow_dynamic: false,
            auth_token: None,
            protect_metrics: false,
            persist_path: PathBuf::from("./state.json"),
            persist_interval: 60,
            log_level: "info".to_string(),
        };

        assert!(matches!(
            config.validate(),
            Err(ConfigError::NoIdentitiesConfigured)
        ));
    }

    #[test]
    fn test_validate_zero_persist_interval() {
        let config = Config {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            identities: vec!["job1".to_string()],
            allow_dynamic: false,
            auth_token: None,
            protect_metrics: false,
            persist_path: PathBuf::from("./state.json"),
            persist_interval: 0,
            log_level: "info".to_string(),
        };

        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidPersistInterval)
        ));
    }

    #[test]
    fn test_validate_empty_identities_with_dynamic_allowed() {
        let config = Config {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            identities: vec![],
            allow_dynamic: true,
            auth_token: None,
            protect_metrics: false,
            persist_path: PathBuf::from("./state.json"),
            persist_interval: 60,
            log_level: "info".to_string(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_protect_metrics_default_false() {
        let config = Config {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            identities: vec!["job1".to_string()],
            allow_dynamic: false,
            auth_token: None,
            protect_metrics: false,
            persist_path: PathBuf::from("./state.json"),
            persist_interval: 60,
            log_level: "info".to_string(),
        };

        assert!(!config.protect_metrics);
    }

    #[test]
    fn test_protect_metrics_enabled() {
        let config = Config {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            identities: vec!["job1".to_string()],
            allow_dynamic: false,
            auth_token: Some("secret".to_string()),
            protect_metrics: true,
            persist_path: PathBuf::from("./state.json"),
            persist_interval: 60,
            log_level: "info".to_string(),
        };

        assert!(config.protect_metrics);
        assert!(config.auth_token.is_some());
    }
}
