use crate::domain::{CallbackRecord, CallbackStore, Identity, StoreError};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory implementation of CallbackStore.
/// Uses Arc<RwLock<HashMap>> for concurrent access.
#[derive(Debug, Clone)]
pub struct InMemoryStore {
    data: Arc<RwLock<HashMap<String, CallbackRecord>>>,
    allowed_identities: Option<Arc<HashSet<String>>>,
}

impl InMemoryStore {
    /// Creates a new InMemoryStore with the given allowed identities.
    /// If `allowed_identities` is None, all identities are allowed (dynamic mode).
    pub fn new(allowed_identities: Option<HashSet<String>>) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            allowed_identities: allowed_identities.map(Arc::new),
        }
    }

    /// Checks if the given identity is allowed.
    fn is_allowed(&self, identity: &Identity) -> bool {
        match &self.allowed_identities {
            None => true, // ALLOW_DYNAMIC=true
            Some(allowed) => allowed.contains(identity.as_str()),
        }
    }

    /// Gets the current UTC timestamp in seconds.
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

impl CallbackStore for InMemoryStore {
    async fn record_callback(&self, identity: &Identity) -> Result<(), StoreError> {
        if !self.is_allowed(identity) {
            tracing::warn!("Identity not allowed: {}", identity);
            return Err(StoreError::IdentityNotAllowed(identity.to_string()));
        }

        let timestamp = Self::current_timestamp();
        tracing::debug!(
            "Recording callback for identity: {} at {}",
            identity,
            timestamp
        );

        let mut data = self.data.write().await;

        data.entry(identity.to_string())
            .and_modify(|record| {
                record.record_callback(timestamp);
                tracing::debug!(
                    "Updated existing record for {}: count={}",
                    identity,
                    record.callback_count
                );
            })
            .or_insert_with(|| {
                tracing::info!("Creating new record for identity: {}", identity);
                CallbackRecord::new(timestamp)
            });

        Ok(())
    }

    async fn get_record(&self, identity: &Identity) -> Result<Option<CallbackRecord>, StoreError> {
        let data = self.data.read().await;
        Ok(data.get(identity.as_str()).cloned())
    }

    async fn get_all_records(&self) -> Result<Vec<(Identity, CallbackRecord)>, StoreError> {
        tracing::debug!("Retrieving all records from memory store");
        let data = self.data.read().await;
        let mut records = Vec::with_capacity(data.len());

        for (id_str, record) in data.iter() {
            match Identity::new(id_str.clone()) {
                Ok(identity) => records.push((identity, record.clone())),
                Err(e) => {
                    tracing::error!("Corrupted identity in store: {}: {}", id_str, e);
                    continue;
                }
            }
        }

        Ok(records)
    }

    async fn load_snapshot(
        &self,
        snapshot: HashMap<String, CallbackRecord>,
    ) -> Result<(), StoreError> {
        let count = snapshot.len();
        tracing::info!("Loading snapshot with {} records into memory store", count);
        let mut data = self.data.write().await;
        *data = snapshot;
        tracing::debug!("Memory store snapshot load complete");
        Ok(())
    }

    async fn snapshot(&self) -> Result<HashMap<String, CallbackRecord>, StoreError> {
        let data = self.data.read().await;
        tracing::debug!("Creating memory store snapshot ({} records)", data.len());
        Ok(data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_callback_dynamic_mode() {
        let store = InMemoryStore::new(None);
        let id = Identity::new("test-job").unwrap();

        // First callback
        store.record_callback(&id).await.unwrap();

        let record = store.get_record(&id).await.unwrap().unwrap();
        assert_eq!(record.callback_count, 1);

        // Second callback
        store.record_callback(&id).await.unwrap();

        let record = store.get_record(&id).await.unwrap().unwrap();
        assert_eq!(record.callback_count, 2);
    }

    #[tokio::test]
    async fn test_record_callback_with_allow_list() {
        let mut allowed = HashSet::new();
        allowed.insert("allowed-job".to_string());

        let store = InMemoryStore::new(Some(allowed));

        let allowed_id = Identity::new("allowed-job").unwrap();
        let forbidden_id = Identity::new("forbidden-job").unwrap();

        // Allowed identity should work
        assert!(store.record_callback(&allowed_id).await.is_ok());

        // Forbidden identity should fail
        let result = store.record_callback(&forbidden_id).await;
        assert!(matches!(result, Err(StoreError::IdentityNotAllowed(_))));
    }

    #[tokio::test]
    async fn test_get_all_records() {
        let store = InMemoryStore::new(None);

        let id1 = Identity::new("job1").unwrap();
        let id2 = Identity::new("job2").unwrap();

        store.record_callback(&id1).await.unwrap();
        store.record_callback(&id2).await.unwrap();

        let records = store.get_all_records().await.unwrap();
        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn test_snapshot_and_load() {
        let store = InMemoryStore::new(None);
        let id = Identity::new("test-job").unwrap();

        store.record_callback(&id).await.unwrap();

        // Create snapshot
        let snapshot = store.snapshot().await.unwrap();
        assert_eq!(snapshot.len(), 1);

        // Create new store and load snapshot
        let new_store = InMemoryStore::new(None);
        new_store.load_snapshot(snapshot).await.unwrap();

        let record = new_store.get_record(&id).await.unwrap().unwrap();
        assert_eq!(record.callback_count, 1);
    }
}
