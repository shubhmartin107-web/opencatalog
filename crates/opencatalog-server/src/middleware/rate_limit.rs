use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

pub struct RateLimiter {
    inner: parking_lot::Mutex<HashMap<String, (u64, Instant)>>,
    max_requests: u64,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_per_minute: u64) -> Self {
        Self {
            inner: parking_lot::Mutex::new(HashMap::new()),
            max_requests: max_per_minute,
            window_secs: 60,
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.inner.lock();
        let entry = map.get(key).copied();
        if let Some((count, window_start)) = entry {
            if now.duration_since(window_start).as_secs() >= self.window_secs {
                map.insert(key.to_string(), (1, now));
                true
            } else if count >= self.max_requests {
                false
            } else {
                map.insert(key.to_string(), (count + 1, window_start));
                true
            }
        } else {
            map.insert(key.to_string(), (1, now));
            true
        }
    }
}

pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let limiter = req
        .extensions()
        .get::<Arc<RateLimiter>>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            req.headers()
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
        })
        .unwrap_or("unknown")
        .to_string();

    if !limiter.check(&ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(req).await)
}