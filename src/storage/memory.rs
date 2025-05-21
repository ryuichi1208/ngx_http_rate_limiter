use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::storage::{StorageBackend, StorageError};

#[derive(Debug)]
struct RateLimit {
    count: u32,
    expire_at: u64,
}

pub struct MemoryStorage {
    store: Mutex<HashMap<String, RateLimit>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    fn get_current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn get(&self, key: &str) -> Result<u32, StorageError> {
        let store = self.store.lock().map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        let current_time = Self::get_current_timestamp();

        if let Some(rate_limit) = store.get(key) {
            if rate_limit.expire_at > current_time {
                return Ok(rate_limit.count);
            }
        }

        Ok(0)
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError> {
        let mut store = self.store.lock().map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        let current_time = Self::get_current_timestamp();
        let expire_at = current_time + expire as u64;

        match store.get_mut(key) {
            Some(rate_limit) if rate_limit.expire_at > current_time => {
                rate_limit.count += 1;
                rate_limit.expire_at = expire_at;
            }
            _ => {
                store.insert(key.to_string(), RateLimit {
                    count: 1,
                    expire_at,
                });
            }
        }

        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        let mut store = self.store.lock().map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        store.remove(key);
        Ok(())
    }

    async fn cleanup_expired(&mut self) -> Result<(), StorageError> {
        let mut store = self.store.lock().map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        let current_time = Self::get_current_timestamp();
        store.retain(|_, rate_limit| rate_limit.expire_at > current_time);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_memory_storage() {
        let mut storage = MemoryStorage::new();

        // Test increment and get
        storage.increment("test_key", 2).await.unwrap();
        assert_eq!(storage.get("test_key").await.unwrap(), 1);

        storage.increment("test_key", 2).await.unwrap();
        assert_eq!(storage.get("test_key").await.unwrap(), 2);

        // Test expiration
        storage.increment("expire_key", 1).await.unwrap();
        thread::sleep(Duration::from_secs(2));
        assert_eq!(storage.get("expire_key").await.unwrap(), 0);

        // Test cleanup
        storage.cleanup_expired().await.unwrap();
        assert_eq!(storage.get("expire_key").await.unwrap(), 0);
    }
}
