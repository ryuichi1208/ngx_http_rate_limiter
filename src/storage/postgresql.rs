use async_trait::async_trait;
use tokio_postgres::{Client, NoTls, Error as PgError};
use crate::storage::{StorageBackend, StorageError};

pub struct PostgresStorage {
    client: Client,
}

impl PostgresStorage {
    pub async fn new(connection_str: &str) -> Result<Self, StorageError> {
        let (client, connection) = tokio_postgres::connect(connection_str, NoTls)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        // Handle connection in background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        // Create table
        Self::create_table(&client).await?;

        Ok(Self { client })
    }

    async fn create_table(client: &Client) -> Result<(), StorageError> {
        client.batch_execute(
            r"
            CREATE TABLE IF NOT EXISTS rate_limits (
                key_name VARCHAR(255) PRIMARY KEY,
                count INTEGER NOT NULL DEFAULT 0,
                expire_at TIMESTAMP WITH TIME ZONE NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_expire_at ON rate_limits(expire_at);
            "
        ).await.map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for PostgresStorage {
    async fn get(&self, key: &str) -> Result<u32, StorageError> {
        let row = self.client
            .query_opt(
                "SELECT count FROM rate_limits WHERE key_name = $1 AND expire_at > NOW()",
                &[&key]
            )
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| r.get::<_, i32>(0) as u32).unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), StorageError> {
        self.client
            .execute(
                r"
                INSERT INTO rate_limits (key_name, count, expire_at)
                VALUES ($1, 1, NOW() + ($2 || ' seconds')::INTERVAL)
                ON CONFLICT (key_name) DO UPDATE
                SET count = CASE
                    WHEN rate_limits.expire_at > NOW()
                    THEN rate_limits.count + 1
                    ELSE 1
                    END,
                    expire_at = NOW() + ($2 || ' seconds')::INTERVAL
                ",
                &[&key, &(expire as i32)]
            )
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.client
            .execute(
                "DELETE FROM rate_limits WHERE key_name = $1",
                &[&key]
            )
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired(&mut self) -> Result<(), StorageError> {
        self.client
            .execute(
                "DELETE FROM rate_limits WHERE expire_at <= NOW()",
                &[]
            )
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
