use async_trait::async_trait;
use redis::{Client, AsyncCommands};
use crate::storage::{StorageBackend, StorageError};

pub struct RedisStorage {
    client: Client,
}

impl RedisStorage {
    pub fn new(redis_url: &str) -> Result<Self, StorageError> {
        let client = Client::open(redis_url)
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl StorageBackend for RedisStorage {
    async fn get(&self, key: &str) -> Result<u32, StorageError> {
        let mut conn = self.client
            .get_async_connection()
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        let count: Option<u32> = conn.get(key)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError> {
        let mut conn = self.client
            .get_async_connection()
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        // Execute increment and expiration setting using multi command
        let mut pipe = redis::pipe();
        pipe.atomic()
            .incr(key, 1_u32)
            .expire(key, expire as usize);

        pipe.query_async(&mut conn)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        let mut conn = self.client
            .get_async_connection()
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        conn.del(key)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired(&mut self) -> Result<(), StorageError> {
        // Redis automatically removes expired keys,
        // so no special implementation is needed
        Ok(())
    }
}
