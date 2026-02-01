//! Rate limiting utilities for channel connectors.
//!
//! Implements token bucket rate limiting to prevent exceeding platform API limits.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Configuration for rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed in the window.
    pub max_requests: u32,
    /// Time window for rate limiting.
    pub window: Duration,
    /// Burst capacity (allows temporary spikes).
    pub burst_capacity: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 30,
            window: Duration::from_secs(1),
            burst_capacity: 5,
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit config.
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            burst_capacity: max_requests / 6,
        }
    }

    /// Create a Telegram-appropriate rate limit config.
    /// Telegram allows ~30 messages/second to different chats.
    pub fn telegram() -> Self {
        Self {
            max_requests: 30,
            window: Duration::from_secs(1),
            burst_capacity: 5,
        }
    }

    /// Create a Discord-appropriate rate limit config.
    /// Discord has complex rate limits, this is a safe default.
    pub fn discord() -> Self {
        Self {
            max_requests: 50,
            window: Duration::from_secs(1),
            burst_capacity: 10,
        }
    }

    /// Create a Slack-appropriate rate limit config.
    /// Slack's Web API allows ~1 request/second for most methods.
    pub fn slack() -> Self {
        Self {
            max_requests: 1,
            window: Duration::from_secs(1),
            burst_capacity: 3,
        }
    }
}

/// Token bucket rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
}

#[derive(Debug)]
struct RateLimiterState {
    tokens: f64,
    last_update: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        let initial_tokens = (config.max_requests + config.burst_capacity) as f64;
        Self {
            config,
            state: Arc::new(Mutex::new(RateLimiterState {
                tokens: initial_tokens,
                last_update: Instant::now(),
            })),
        }
    }

    /// Create a rate limiter with default configuration.
    pub fn default_limiter() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Try to acquire a permit to make a request.
    /// Returns `Ok(())` if allowed, or `Err(wait_time)` if rate limited.
    pub async fn try_acquire(&self) -> Result<(), Duration> {
        let mut state = self.state.lock().await;
        
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_update);
        let refill_rate = self.config.max_requests as f64 / self.config.window.as_secs_f64();
        let new_tokens = elapsed.as_secs_f64() * refill_rate;
        
        let max_tokens = (self.config.max_requests + self.config.burst_capacity) as f64;
        state.tokens = (state.tokens + new_tokens).min(max_tokens);
        state.last_update = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate wait time until we have a token
            let wait_time = (1.0 - state.tokens) / refill_rate;
            Err(Duration::from_secs_f64(wait_time))
        }
    }

    /// Acquire a permit, waiting if necessary.
    pub async fn acquire(&self) {
        loop {
            match self.try_acquire().await {
                Ok(()) => return,
                Err(wait_time) => {
                    tokio::time::sleep(wait_time).await;
                }
            }
        }
    }

    /// Get the current number of available tokens.
    pub async fn available_tokens(&self) -> u32 {
        let state = self.state.lock().await;
        state.tokens.floor() as u32
    }

    /// Reset the rate limiter to full capacity.
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.tokens = (self.config.max_requests + self.config.burst_capacity) as f64;
        state.last_update = Instant::now();
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_initial_requests() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 10,
            window: Duration::from_secs(1),
            burst_capacity: 5,
        });

        // Should allow initial burst
        for _ in 0..15 {
            assert!(limiter.try_acquire().await.is_ok());
        }

        // Next one should be rate limited
        assert!(limiter.try_acquire().await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_refills() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(1),
            burst_capacity: 0,
        });

        // Consume all tokens
        for _ in 0..100 {
            let _ = limiter.try_acquire().await;
        }

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should have some tokens now
        assert!(limiter.try_acquire().await.is_ok());
    }
}
