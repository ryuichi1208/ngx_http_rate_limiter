# Nginx Rate Limiter Module (Rust)

This Nginx module provides rate limiting functionality implemented in Rust. It supports multiple backend storage options (Memcached, Redis, MySQL, PostgreSQL).

## Features

- IP address-based rate limiting
- Multiple storage backend support:
  - Redis
  - Memcached
  - MySQL
  - PostgreSQL
- Configurable rate limits and window sizes

## Requirements

- Rust 1.70 or higher
- Nginx 1.20 or higher
- One of the following storage backends:
  - Redis
  - Memcached
  - MySQL
  - PostgreSQL

## Installation

1. Clone the repository:

```bash
git clone https://github.com/yourusername/ngx_http_rate_limiter
cd ngx_http_rate_limiter
```

2. Build:

```bash
cargo build --release
```

3. Add the following to your Nginx configuration file:

```nginx
load_module /path/to/libngx_http_rate_limiter.so;

http {
    rate_limit_storage redis;  # redis, memcached, mysql, postgresql
    rate_limit_requests 100;   # requests per minute
    rate_limit_window 60;      # window size in seconds
}
```

## Database Setup

### MySQL

```sql
CREATE DATABASE ratelimit;
USE ratelimit;

CREATE TABLE rate_limits (
    key VARCHAR(255) PRIMARY KEY,
    count INT NOT NULL DEFAULT 0,
    expire_at TIMESTAMP NOT NULL
);
```

### PostgreSQL

```sql
CREATE DATABASE ratelimit;
\c ratelimit

CREATE TABLE rate_limits (
    key VARCHAR(255) PRIMARY KEY,
    count INT NOT NULL DEFAULT 0,
    expire_at TIMESTAMP NOT NULL
);
```

## Configuration Options

- `rate_limit_storage`: Storage backend selection (redis/memcached/mysql/postgresql)
- `rate_limit_requests`: Number of allowed requests within the specified period
- `rate_limit_window`: Rate limit window size in seconds

## License

MIT
