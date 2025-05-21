use async_trait::async_trait;
use std::error::Error;

mod redis;
mod memcached;
mod mysql;
mod postgresql;

pub use redis::RedisStorage;
pub use memcached::MemcachedStorage;
pub use mysql::MySQLStorage;
pub use postgresql::PostgresStorage;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("接続エラー: {0}")]
    ConnectionError(String),
    #[error("キーが見つかりません: {0}")]
    KeyNotFound(String),
    #[error("値の型が不正です: {0}")]
    InvalidValueType(String),
    #[error("データベースエラー: {0}")]
    DatabaseError(String),
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// キーに対応する現在のカウント値を取得
    async fn get(&self, key: &str) -> Result<u32, StorageError>;

    /// キーに対応するカウント値をインクリメント
    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError>;

    /// キーに対応する値を削除
    async fn delete(&mut self, key: &str) -> Result<(), StorageError>;

    /// 有効期限切れのキーを削除
    async fn cleanup_expired(&mut self) -> Result<(), StorageError>;
}
