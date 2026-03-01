use crate::domain::{CallbackRecord, CallbackStore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::interval;

/// Errors that can occur during file persistence.
#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Store error: {0}")]
    Store(String),
}

/// Periodic persister that saves store snapshots to a JSON file.
pub struct FilePersister {
    path: PathBuf,
    interval_seconds: u64,
}

impl FilePersister {
    /// Creates a new FilePersister.
    pub fn new(path: impl Into<PathBuf>, interval_seconds: u64) -> Self {
        Self {
            path: path.into(),
            interval_seconds,
        }
    }

    /// Loads the initial state from the persist file.
    /// Returns an empty HashMap if the file doesn't exist.
    pub async fn load_initial_state(
        &self,
    ) -> Result<HashMap<String, CallbackRecord>, PersistError> {
        if !self.path.exists() {
            tracing::info!(
                "No persist file found at {}, starting with empty state",
                self.path.display()
            );
            return Ok(HashMap::new());
        }

        tracing::info!("Loading state from {}", self.path.display());
        tracing::debug!("Reading persist file: {}", self.path.display());
        let contents = tokio::fs::read_to_string(&self.path).await?;

        tracing::debug!("Deserializing state from JSON");
        let data: HashMap<String, CallbackRecord> = serde_json::from_str(&contents)?;

        tracing::info!(
            "Successfully loaded {} records from persist file",
            data.len()
        );
        Ok(data)
    }

    /// Saves the current state to disk using atomic write.
    /// Writes to a temporary file first, then renames to prevent corruption.
    /// Uses unique temp file name (with PID) to prevent collisions when multiple
    /// instances write to the same directory.
    async fn save_snapshot(
        &self,
        data: &HashMap<String, CallbackRecord>,
    ) -> Result<(), PersistError> {
        tracing::debug!("Serializing state for snapshot");
        let json = serde_json::to_string_pretty(data)?;

        let tmp_path = PathBuf::from(format!(
            "{}.tmp.{}",
            self.path.display(),
            std::process::id()
        ));
        tracing::debug!("Writing to temporary file: {}", tmp_path.display());
        tokio::fs::write(&tmp_path, json).await?;

        tracing::debug!("Performing atomic rename to {}", self.path.display());
        tokio::fs::rename(&tmp_path, &self.path).await?;

        tracing::info!("Successfully persisted {} records to disk", data.len());
        Ok(())
    }

    /// Starts the periodic persist loop.
    /// This runs indefinitely until cancelled.
    pub async fn run_persist_loop<S: CallbackStore>(&self, store: S) {
        tracing::info!(
            "Starting background persistence loop (interval: {}s, path: {})",
            self.interval_seconds,
            self.path.display()
        );
        let mut tick = interval(Duration::from_secs(self.interval_seconds));

        loop {
            tick.tick().await;
            tracing::debug!("Persistence loop tick: creating snapshot");

            match store.snapshot().await {
                Ok(snapshot) => {
                    if let Err(e) = self.save_snapshot(&snapshot).await {
                        tracing::error!("Failed to persist state: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create snapshot: {}", e);
                }
            }
        }
    }

    /// Performs a final persist operation.
    /// Should be called during graceful shutdown.
    pub async fn final_persist<S: CallbackStore>(&self, store: S) -> Result<(), PersistError> {
        tracing::info!("Performing final persist before shutdown");

        let snapshot = store
            .snapshot()
            .await
            .map_err(|e| PersistError::Store(e.to_string()))?;

        self.save_snapshot(&snapshot).await?;

        tracing::info!("Final persist completed");
        Ok(())
    }

    /// Gets the path to the persist file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InMemoryStore;
    use crate::domain::Identity;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_empty_state() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");

        let persister = FilePersister::new(path, 60);
        let state = persister.load_initial_state().await.unwrap();

        assert!(state.is_empty());
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");

        let persister = FilePersister::new(&path, 60);
        let store = InMemoryStore::new(None);

        // Record some callbacks
        let id1 = Identity::new("job1").unwrap();
        let id2 = Identity::new("job2").unwrap();
        store.record_callback(&id1).await.unwrap();
        store.record_callback(&id2).await.unwrap();

        // Save snapshot
        let snapshot = store.snapshot().await.unwrap();
        persister.save_snapshot(&snapshot).await.unwrap();

        // Load into new store
        let loaded = persister.load_initial_state().await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains_key("job1"));
        assert!(loaded.contains_key("job2"));
    }

    #[tokio::test]
    async fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");

        let persister = FilePersister::new(&path, 60);
        let mut data = HashMap::new();
        data.insert("test".to_string(), CallbackRecord::new(1234567890));

        persister.save_snapshot(&data).await.unwrap();

        // Verify main file exists
        assert!(path.exists());

        // Verify tmp file doesn't exist (format: {path}.tmp.{pid})
        let tmp_path = PathBuf::from(format!("{}.tmp.{}", path.display(), std::process::id()));
        assert!(!tmp_path.exists());
    }
}
