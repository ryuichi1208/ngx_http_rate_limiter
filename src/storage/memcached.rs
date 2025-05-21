use async_trait::async_trait;
use memcached::Client;
use crate::storage::{StorageBackend, StorageError};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MemcachedStorage {
    client: Client,
}

impl MemcachedStorage {
    pub fn new(memcached_url: &str) -> Result<Self, StorageError> {
        let client = Client::connect(memcached_url)
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;
        Ok(Self { client })
    }

    fn get_current_timestamp() -> u32 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32
    }
}

#[async_trait]
impl StorageBackend for MemcachedStorage {
    async fn get(&self, key: &str) -> Result<u32, StorageError> {
        let value: Option<u32> = self.client
            .get(key)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(value.unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError> {
        // Since Memcached's increment fails if the key doesn't exist,
        // we need to combine add (set only if key doesn't exist) and increment (increase existing value)
        let expire_time = Self::get_current_timestamp() + expire;

        // Set initial value if key doesn't exist
        let _ = self.client.add(key, 0u32, expire_time);

        // Increment the value
        self.client
            .increment(key, 1, 0, expire_time)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete(key)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn cleanup_expired(&mut self) -> Result<(), StorageError> {
        // Memcached automatically removes expired keys,
        // so no special implementation is needed
        Ok(())
    }
}
