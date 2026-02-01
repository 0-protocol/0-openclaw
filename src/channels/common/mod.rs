//! Common utilities for channel connectors.
//!
//! This module provides shared functionality used across all channel implementations,
//! including rate limiting, retry logic, and message conversion utilities.

pub mod rate_limit;
pub mod retry;

pub use rate_limit::{RateLimiter, RateLimitConfig};
pub use retry::{RetryPolicy, RetryResult, with_retry};
