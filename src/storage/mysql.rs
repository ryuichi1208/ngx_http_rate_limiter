use async_trait::async_trait;
use mysql::{Pool, PooledConn, Opts, OptsBuilder};
use crate::storage::{StorageBackend, StorageError};

pub struct MySQLStorage {
    pool: Pool,
}

impl MySQLStorage {
    pub fn new(url: &str) -> Result<Self, StorageError> {
        let opts = Opts::from_url(url)
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        let builder = OptsBuilder::from_opts(opts);
        let pool = Pool::new(builder)
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        // Create table if it doesn't exist
        let mut conn = pool.get_conn()
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Self::create_table(&mut conn)?;

        Ok(Self { pool })
    }

    fn create_table(conn: &mut PooledConn) -> Result<(), StorageError> {
        conn.query_drop(
            r"CREATE TABLE IF NOT EXISTS rate_limits (
                key_name VARCHAR(255) PRIMARY KEY,
                count INT UNSIGNED NOT NULL DEFAULT 0,
                expire_at TIMESTAMP NOT NULL
            )"
        ).map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create index for expiration time
        conn.query_drop(
            "CREATE INDEX IF NOT EXISTS idx_expire_at ON rate_limits(expire_at)"
        ).map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for MySQLStorage {
    async fn get(&self, key: &str) -> Result<u32, StorageError> {
        let mut conn = self.pool
            .get_conn()
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        let result: Option<u32> = conn
            .query_first(
                "SELECT count FROM rate_limits WHERE key_name = ? AND expire_at > NOW()",
            )
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(result.unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError> {
        let mut conn = self.pool
            .get_conn()
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        conn.exec_drop(
            r"INSERT INTO rate_limits (key_name, count, expire_at)
              VALUES (?, 1, NOW() + INTERVAL ? SECOND)
              ON DUPLICATE KEY UPDATE
                count = IF(expire_at > NOW(), count + 1, 1),
                expire_at = NOW() + INTERVAL ? SECOND",
            (key, expire, expire)
        ).map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        let mut conn = self.pool
            .get_conn()
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        conn.exec_drop(
            "DELETE FROM rate_limits WHERE key_name = ?",
            (key,)
        ).map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired(&mut self) -> Result<(), StorageError> {
        let mut conn = self.pool
            .get_conn()
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        conn.query_drop(
            "DELETE FROM rate_limits WHERE expire_at <= NOW()"
        ).map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
