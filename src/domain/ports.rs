use super::model::{CallbackRecord, Identity};
use std::collections::HashMap;
use std::future::Future;

/// Errors that can occur in the callback store.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StoreError {
    #[error("Identity not allowed: {0}")]
    IdentityNotAllowed(String),
    #[error("Store capacity exceeded")]
    StoreFull,
    #[error("Internal store error: {0}")]
    Internal(String),
}

/// Port for storing and retrieving callback records.
/// This trait defines the interface for different storage implementations.
pub trait CallbackStore: Clone + Send + Sync + 'static {
    /// Records a new callback for the given identity.
    /// Updates the timestamp and increments the counter.
    fn record_callback(
        &self,
        identity: &Identity,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Retrieves the callback record for a specific identity.
    /// Returns None if the identity has never been recorded.
    fn get_record(
        &self,
        identity: &Identity,
    ) -> impl Future<Output = Result<Option<CallbackRecord>, StoreError>> + Send;

    /// Retrieves all callback records.
    /// Returns a vector of (identity, record) pairs.
    fn get_all_records(
        &self,
    ) -> impl Future<Output = Result<Vec<(Identity, CallbackRecord)>, StoreError>> + Send;

    /// Loads a snapshot of data into the store.
    /// Used for restoring state from persistence.
    fn load_snapshot(
        &self,
        data: HashMap<String, CallbackRecord>,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Creates a snapshot of the current state.
    /// Used for persisting state to disk.
    fn snapshot(
        &self,
    ) -> impl Future<Output = Result<HashMap<String, CallbackRecord>, StoreError>> + Send;
}
