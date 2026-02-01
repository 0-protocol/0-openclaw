//! Retry logic for channel operations.
//!
//! Implements exponential backoff with jitter for resilient channel operations.

use std::future::Future;
use std::time::Duration;
use rand::Rng;
use crate::error::ChannelError;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay before first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: f64,
    /// Whether to add jitter to delays.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy.
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Create a policy with no retries.
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Create an aggressive retry policy for critical operations.
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a conservative retry policy for rate-limited APIs.
    pub fn conservative() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(120),
            backoff_multiplier: 3.0,
            jitter: true,
        }
    }

    /// Calculate the delay for a given attempt number.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay.as_millis() as f64 
            * self.backoff_multiplier.powi(attempt as i32);
        let capped_delay = base_delay.min(self.max_delay.as_millis() as f64);
        
        let final_delay = if self.jitter {
            let jitter_factor = rand::thread_rng().gen_range(0.5..1.5);
            capped_delay * jitter_factor
        } else {
            capped_delay
        };

        Duration::from_millis(final_delay as u64)
    }
}

/// Result of a retry operation.
#[derive(Debug)]
pub enum RetryResult<T> {
    /// Operation succeeded.
    Success(T),
    /// Operation failed after all retries.
    Failed {
        last_error: ChannelError,
        attempts: u32,
    },
    /// Operation was rate limited.
    RateLimited {
        retry_after: Duration,
    },
}

/// Execute an operation with retry logic.
///
/// The operation will be retried according to the policy if it returns
/// a retryable error. Rate limit errors are handled specially.
pub async fn with_retry<F, Fut, T>(
    policy: &RetryPolicy,
    mut operation: F,
) -> RetryResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ChannelError>>,
{
    let mut attempts = 0;

    loop {
        match operation().await {
            Ok(result) => return RetryResult::Success(result),
            Err(e) => {
                // Check if we should retry
                if !should_retry(&e) {
                    return RetryResult::Failed {
                        last_error: e,
                        attempts,
                    };
                }

                // Handle rate limiting specially
                if let ChannelError::RateLimited { retry_after } = &e {
                    return RetryResult::RateLimited {
                        retry_after: Duration::from_millis(*retry_after),
                    };
                }

                attempts += 1;
                if attempts > policy.max_retries {
                    return RetryResult::Failed {
                        last_error: e,
                        attempts,
                    };
                }

                // Wait before retrying
                let delay = policy.delay_for_attempt(attempts - 1);
                tracing::debug!(
                    "Retry attempt {} after {:?}: {:?}",
                    attempts,
                    delay,
                    e
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Determine if an error is retryable.
fn should_retry(error: &ChannelError) -> bool {
    match error {
        ChannelError::ConnectionFailed(_) => true,
        ChannelError::SendFailed(_) => true,
        ChannelError::ReceiveFailed(_) => true,
        ChannelError::RateLimited { .. } => true,
        ChannelError::PermissionDenied(_) => false,
        ChannelError::InvalidMessage(_) => false,
        ChannelError::ChannelClosed => false,
        ChannelError::AuthenticationFailed(_) => false,
    }
}

/// Execute an operation with automatic rate limit handling.
///
/// Unlike `with_retry`, this will automatically wait and retry when
/// rate limited, up to a maximum total wait time.
pub async fn with_rate_limit_retry<F, Fut, T>(
    policy: &RetryPolicy,
    max_rate_limit_wait: Duration,
    operation: F,
) -> RetryResult<T>
where
    F: FnMut() -> Fut + Clone,
    Fut: Future<Output = Result<T, ChannelError>>,
{
    let mut total_wait = Duration::ZERO;
    let op = operation;

    loop {
        match with_retry(policy, op.clone()).await {
            RetryResult::Success(result) => return RetryResult::Success(result),
            RetryResult::Failed { last_error, attempts } => {
                return RetryResult::Failed { last_error, attempts };
            }
            RetryResult::RateLimited { retry_after } => {
                total_wait += retry_after;
                if total_wait > max_rate_limit_wait {
                    return RetryResult::Failed {
                        last_error: ChannelError::RateLimited { 
                            retry_after: retry_after.as_millis() as u64 
                        },
                        attempts: 0,
                    };
                }
                tracing::info!("Rate limited, waiting {:?}", retry_after);
                tokio::time::sleep(retry_after).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: false,
        };

        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(400));
    }

    #[test]
    fn test_delay_capping() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 10.0,
            jitter: false,
        };

        // Should be capped at max_delay
        assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_successful_retry() {
        let policy = RetryPolicy::new(3);
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(&policy, || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count < 2 {
                    Err(ChannelError::ConnectionFailed("test".to_string()))
                } else {
                    Ok("success")
                }
            }
        }).await;

        match result {
            RetryResult::Success(s) => assert_eq!(s, "success"),
            _ => panic!("Expected success"),
        }
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}
