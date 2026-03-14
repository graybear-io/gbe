use std::sync::Arc;

use bytes::Bytes;
use gbe_state_store::StateStore;

use crate::error::SentinelError;

/// Attempts a CAS claim on a task in the state store.
///
/// Flow: `compare_and_swap(key, "state", "pending", "claimed")`
///   - Success → set worker, `updated_at`, `timeout_at`
///   - Failure → return `ClaimFailed` error
///
/// # Errors
///
/// Returns `SentinelError::ClaimFailed` if the CAS fails, or a store error on I/O failure.
///
/// # Panics
///
/// Panics if the system clock is before the Unix epoch.
pub async fn claim_task(
    store: &Arc<dyn StateStore>,
    state_key: &str,
    host_id: &str,
    vm_cid: u32,
    timeout_at: u64,
) -> Result<(), SentinelError> {
    let claimed = store
        .compare_and_swap(
            state_key,
            "state",
            Bytes::from("pending"),
            Bytes::from("claimed"),
        )
        .await?;

    if !claimed {
        return Err(SentinelError::ClaimFailed {
            task_id: state_key.to_string(),
            reason: "CAS failed — task already claimed".to_string(),
        });
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();

    let worker = format!("{host_id}:{vm_cid}");

    store
        .set_fields(
            state_key,
            std::collections::HashMap::from([
                ("worker".to_string(), Bytes::from(worker)),
                ("updated_at".to_string(), Bytes::from(now.clone())),
                (
                    "timeout_at".to_string(),
                    Bytes::from(timeout_at.to_string()),
                ),
            ]),
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use gbe_state_store::{Record, ScanFilter, StateStoreError};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Duration;

    /// In-memory StateStore for testing claim logic.
    struct MockStore {
        cas_result: Mutex<bool>,
        fields: Mutex<HashMap<String, HashMap<String, Bytes>>>,
    }

    impl MockStore {
        fn new(cas_succeeds: bool) -> Self {
            Self {
                cas_result: Mutex::new(cas_succeeds),
                fields: Mutex::new(HashMap::new()),
            }
        }

        fn get_stored_fields(&self, key: &str) -> HashMap<String, String> {
            self.fields
                .lock()
                .unwrap()
                .get(key)
                .map(|f| {
                    f.iter()
                        .map(|(k, v)| (k.clone(), String::from_utf8_lossy(v).to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }
    }

    #[async_trait]
    impl gbe_state_store::StateStore for MockStore {
        async fn get(&self, _key: &str) -> Result<Option<Record>, StateStoreError> {
            Ok(None)
        }
        async fn put(
            &self,
            _key: &str,
            _record: Record,
            _ttl: Option<Duration>,
        ) -> Result<(), StateStoreError> {
            Ok(())
        }
        async fn delete(&self, _key: &str) -> Result<(), StateStoreError> {
            Ok(())
        }
        async fn get_field(
            &self,
            _key: &str,
            _field: &str,
        ) -> Result<Option<Bytes>, StateStoreError> {
            Ok(None)
        }
        async fn set_field(
            &self,
            _key: &str,
            _field: &str,
            _value: Bytes,
        ) -> Result<(), StateStoreError> {
            Ok(())
        }
        async fn set_fields(
            &self,
            key: &str,
            fields: HashMap<String, Bytes>,
        ) -> Result<(), StateStoreError> {
            self.fields.lock().unwrap().insert(key.to_string(), fields);
            Ok(())
        }
        async fn compare_and_swap(
            &self,
            _key: &str,
            _field: &str,
            _expected: Bytes,
            _new: Bytes,
        ) -> Result<bool, StateStoreError> {
            Ok(*self.cas_result.lock().unwrap())
        }
        async fn scan(
            &self,
            _prefix: &str,
            _filter: Option<ScanFilter>,
        ) -> Result<Vec<(String, Record)>, StateStoreError> {
            Ok(vec![])
        }
        async fn ping(&self) -> Result<bool, StateStoreError> {
            Ok(true)
        }
        async fn close(&self) -> Result<(), StateStoreError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn claim_succeeds_sets_fields() {
        let mock = Arc::new(MockStore::new(true));
        let store: Arc<dyn gbe_state_store::StateStore> = Arc::clone(&mock) as _;
        let result = claim_task(&store, "job:1:task:a", "host-01", 3, 9999).await;
        assert!(result.is_ok());

        let fields = mock.get_stored_fields("job:1:task:a");
        assert_eq!(fields["worker"], "host-01:3");
        assert_eq!(fields["timeout_at"], "9999");
        assert!(fields.contains_key("updated_at"));
    }

    #[tokio::test]
    async fn claim_fails_on_cas_conflict() {
        let store: Arc<dyn gbe_state_store::StateStore> = Arc::new(MockStore::new(false));
        let result = claim_task(&store, "job:1:task:a", "host-01", 3, 9999).await;
        let err = result.unwrap_err();
        assert!(matches!(err, SentinelError::ClaimFailed { .. }));
        assert!(err.to_string().contains("already claimed"));
    }

    #[tokio::test]
    async fn claim_worker_format() {
        let mock = Arc::new(MockStore::new(true));
        let store: Arc<dyn gbe_state_store::StateStore> = Arc::clone(&mock) as _;
        claim_task(&store, "k", "node-x", 42, 0).await.unwrap();
        let fields = mock.get_stored_fields("k");
        assert_eq!(fields["worker"], "node-x:42");
    }
}
