use crate::error::ApiError;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

/// Bounded global and account limits, including unknown names.
/// Also configure a per-IP limit at the TLS reverse proxy.
pub fn check(username: &str) -> Result<(), ApiError> {
    type Attempts = HashMap<String, (Instant, u32)>;
    static ATTEMPTS: OnceLock<Mutex<Attempts>> = OnceLock::new();
    let mut attempts = ATTEMPTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| ApiError::Internal("Login limiter unavailable".into()))?;
    let now = Instant::now();
    attempts.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
    for (key, limit) in [(String::new(), 120), (format!("user:{username}"), 10)] {
        let entry = attempts.entry(key).or_insert((now, 0));
        if entry.1 >= limit {
            return Err(ApiError::TooManyRequests);
        }
        entry.1 += 1;
    }
    Ok(())
}
