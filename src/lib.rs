use nginx_module::{
    http::{HTTPModule, HTTPContext, Status},
    bindings,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

mod storage;
use storage::{
    StorageBackend,
    MemcachedStorage,
    RedisStorage,
    MySQLStorage,
    PostgresStorage,
};

#[derive(Debug)]
pub struct RateLimiter {
    storage: Arc<Mutex<Box<dyn StorageBackend>>>,
    requests_per_second: u32,
    window_size: u32,
}

impl RateLimiter {
    pub fn new(backend_type: &str, requests_per_second: u32, window_size: u32) -> Self {
        let storage: Box<dyn StorageBackend> = match backend_type {
            "memcached" => Box::new(MemcachedStorage::new()),
            "redis" => Box::new(RedisStorage::new()),
            "mysql" => Box::new(MySQLStorage::new()),
            "postgresql" => Box::new(PostgresStorage::new()),
            _ => Box::new(RedisStorage::new()), // デフォルトはRedis
        };

        RateLimiter {
            storage: Arc::new(Mutex::new(storage)),
            requests_per_second,
            window_size,
        }
    }

    async fn is_rate_limited(&self, key: &str) -> bool {
        let mut storage = self.storage.lock().await;
        let current_count = storage.get(key).await.unwrap_or(0);

        if current_count >= self.requests_per_second {
            true
        } else {
            storage.increment(key, self.window_size).await.unwrap_or(());
            false
        }
    }
}

#[async_trait]
impl HTTPModule for RateLimiter {
    async fn handle(&self, ctx: &mut HTTPContext) -> Status {
        let ip = ctx.remote_addr().to_string();

        if self.is_rate_limited(&ip).await {
            ctx.set_status(429);
            Status::Declined
        } else {
            Status::Ok
        }
    }
}

#[no_mangle]
pub extern "C" fn ngx_http_rate_limiter_module() -> *mut bindings::ngx_module_t {
    let rate_limiter = RateLimiter::new("redis", 100, 60);
    nginx_module::create_http_module!(rate_limiter)
}
