use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<u32, Box<dyn Error>>;
    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), Box<dyn Error>>;
}

pub struct RedisStorage {
    client: redis::Client,
}

impl RedisStorage {
    pub fn new() -> Self {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        Self { client }
    }
}

#[async_trait]
impl StorageBackend for RedisStorage {
    async fn get(&self, key: &str) -> Result<u32, Box<dyn Error>> {
        let mut conn = self.client.get_async_connection().await?;
        let count: Option<u32> = redis::cmd("GET").arg(key).query_async(&mut conn).await?;
        Ok(count.unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), Box<dyn Error>> {
        let mut conn = self.client.get_async_connection().await?;
        redis::cmd("INCR").arg(key).query_async(&mut conn).await?;
        redis::cmd("EXPIRE").arg(key).arg(expire).query_async(&mut conn).await?;
        Ok(())
    }
}

pub struct MemcachedStorage {
    client: memcached::Client,
}

impl MemcachedStorage {
    pub fn new() -> Self {
        let client = memcached::Client::connect("memcache://127.0.0.1:11211").unwrap();
        Self { client }
    }
}

#[async_trait]
impl StorageBackend for MemcachedStorage {
    async fn get(&self, key: &str) -> Result<u32, Box<dyn Error>> {
        let value = self.client.get(key)?;
        Ok(value.unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), Box<dyn Error>> {
        self.client.increment(key, 1, 0, expire as u32)?;
        Ok(())
    }
}

pub struct MySQLStorage {
    pool: mysql::Pool,
}

impl MySQLStorage {
    pub fn new() -> Self {
        let pool = mysql::Pool::new("mysql://user:password@localhost:3306/ratelimit").unwrap();
        Self { pool }
    }
}

#[async_trait]
impl StorageBackend for MySQLStorage {
    async fn get(&self, key: &str) -> Result<u32, Box<dyn Error>> {
        let mut conn = self.pool.get_conn()?;
        let result: Option<u32> = conn
            .query_first("SELECT count FROM rate_limits WHERE key = ?")?;
        Ok(result.unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), Box<dyn Error>> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            "INSERT INTO rate_limits (key, count, expire_at)
             VALUES (?, 1, NOW() + INTERVAL ? SECOND)
             ON DUPLICATE KEY UPDATE
             count = count + 1,
             expire_at = NOW() + INTERVAL ? SECOND",
            (key, expire, expire),
        )?;
        Ok(())
    }
}

pub struct PostgresStorage {
    client: tokio_postgres::Client,
}

impl PostgresStorage {
    pub fn new() -> Self {
        let (client, connection) = tokio_postgres::connect(
            "host=localhost user=postgres dbname=ratelimit",
            tokio_postgres::NoTls,
        ).unwrap();
        tokio::spawn(connection);
        Self { client }
    }
}

#[async_trait]
impl StorageBackend for PostgresStorage {
    async fn get(&self, key: &str) -> Result<u32, Box<dyn Error>> {
        let row = self.client
            .query_opt(
                "SELECT count FROM rate_limits WHERE key = $1",
                &[&key],
            )
            .await?;
        Ok(row.map(|r| r.get(0)).unwrap_or(0))
    }

    async fn increment(&mut self, key: &str, expire: u32) -> Result<(), Box<dyn Error>> {
        self.client
            .execute(
                "INSERT INTO rate_limits (key, count, expire_at)
                 VALUES ($1, 1, NOW() + INTERVAL '$2 seconds')
                 ON CONFLICT (key) DO UPDATE
                 SET count = rate_limits.count + 1,
                     expire_at = NOW() + INTERVAL '$2 seconds'",
                &[&key, &(expire as i32)],
            )
            .await?;
        Ok(())
    }
}
