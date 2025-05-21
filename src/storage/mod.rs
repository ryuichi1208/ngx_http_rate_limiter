use async_trait::async_trait;
use std::error::Error;

mod redis;
mod memcached;
mod mysql;
mod postgresql;
mod sqlite;
mod memory;

pub use redis::RedisStorage;
pub use memcached::MemcachedStorage;
pub use mysql::MySQLStorage;
pub use postgresql::PostgresStorage;
pub use sqlite::SQLiteStorage;
pub use memory::MemoryStorage;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Invalid value type: {0}")]
    InvalidValueType(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Get the current count value for the key
    async fn get(&self, key: &str) -> Result<u32, StorageError>;

    /// Increment the count value for the key
    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError>;

    /// Delete the value for the key
    async fn delete(&mut self, key: &str) -> Result<(), StorageError>;

    /// Clean up expired keys
    async fn cleanup_expired(&mut self) -> Result<(), StorageError>;
}
