use async_trait::async_trait;
use rusqlite::{Connection, Result as SqliteResult, params};
use std::path::Path;
use crate::storage::{StorageBackend, StorageError};

pub struct SQLiteStorage {
    conn: Connection,
}

impl SQLiteStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create table if not exists
        Self::create_table(&conn)?;

        Ok(Self { conn })
    }

    pub fn new_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Self::create_table(&conn)?;

        Ok(Self { conn })
    }

    fn create_table(conn: &Connection) -> Result<(), StorageError> {
        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS rate_limits (
                key_name TEXT PRIMARY KEY,
                count INTEGER NOT NULL DEFAULT 0,
                expire_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_expire_at ON rate_limits(expire_at);
            "
        ).map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn get_current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

#[async_trait]
impl StorageBackend for SQLiteStorage {
    async fn get(&self, key: &str) -> Result<u32, StorageError> {
        let current_time = Self::get_current_timestamp();

        let result: Option<u32> = self.conn
            .query_row(
                "SELECT count FROM rate_limits WHERE key_name = ? AND expire_at > ?",
                params![key, current_time],
                |row| row.get(0)
            )
            .optional()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(result.unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError> {
        let current_time = Self::get_current_timestamp();
        let expire_at = current_time + expire as i64;

        let tx = self.conn.transaction()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        tx.execute(
            r"
            INSERT INTO rate_limits (key_name, count, expire_at)
            VALUES (?1, 1, ?2)
            ON CONFLICT(key_name) DO UPDATE SET
                count = CASE
                    WHEN expire_at > ?3 THEN count + 1
                    ELSE 1
                END,
                expire_at = ?2
            ",
            params![key, expire_at, current_time]
        ).map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        tx.commit()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.conn
            .execute(
                "DELETE FROM rate_limits WHERE key_name = ?",
                params![key]
            )
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired(&mut self) -> Result<(), StorageError> {
        let current_time = Self::get_current_timestamp();

        self.conn
            .execute(
                "DELETE FROM rate_limits WHERE expire_at <= ?",
                params![current_time]
            )
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_sqlite_storage() {
        let mut storage = SQLiteStorage::new_in_memory().unwrap();

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
