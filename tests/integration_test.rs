use ngx_http_rate_limiter::storage::{
    StorageBackend,
    RedisStorage,
    MemcachedStorage,
    MySQLStorage,
    PostgresStorage,
    SQLiteStorage,
    MemoryStorage,
};
use std::env;

async fn test_storage_backend<T: StorageBackend>(mut storage: T) {
    // Basic increment and get
    storage.increment("test_key", 60).await.unwrap();
    assert_eq!(storage.get("test_key").await.unwrap(), 1);

    storage.increment("test_key", 60).await.unwrap();
    assert_eq!(storage.get("test_key").await.unwrap(), 2);

    // Delete
    storage.delete("test_key").await.unwrap();
    assert_eq!(storage.get("test_key").await.unwrap(), 0);

    // Cleanup expired
    storage.cleanup_expired().await.unwrap();
}

#[tokio::test]
async fn test_redis_storage() {
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let storage = RedisStorage::new(&redis_url).unwrap();
    test_storage_backend(storage).await;
}

#[tokio::test]
async fn test_memcached_storage() {
    let memcached_url = env::var("MEMCACHED_URL").unwrap_or_else(|_| "memcache://127.0.0.1:11211".to_string());
    let storage = MemcachedStorage::new(&memcached_url).unwrap();
    test_storage_backend(storage).await;
}

#[tokio::test]
async fn test_mysql_storage() {
    let mysql_url = env::var("MYSQL_URL").unwrap_or_else(|_| "mysql://root:password@localhost:3306/ratelimit".to_string());
    let storage = MySQLStorage::new(&mysql_url).unwrap();
    test_storage_backend(storage).await;
}

#[tokio::test]
async fn test_postgres_storage() {
    let postgres_url = env::var("POSTGRES_URL").unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/ratelimit".to_string());
    let storage = PostgresStorage::new(&postgres_url).await.unwrap();
    test_storage_backend(storage).await;
}

#[tokio::test]
async fn test_sqlite_storage() {
    let storage = SQLiteStorage::new_in_memory().unwrap();
    test_storage_backend(storage).await;
}

#[tokio::test]
async fn test_memory_storage() {
    let storage = MemoryStorage::new();
    test_storage_backend(storage).await;
}
