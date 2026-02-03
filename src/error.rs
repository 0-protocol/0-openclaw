//! Error types for 0-openclaw.
//!
//! This module defines all error types used throughout the system.

use thiserror::Error;
use crate::types::ContentHash;

/// Main error type for 0-openclaw operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Gateway errors
    #[error("Gateway error: {0}")]
    Gateway(#[from] GatewayError),

    /// Channel errors
    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),

    /// Skill errors
    #[error("Skill error: {0}")]
    Skill(#[from] SkillError),

    /// Session errors
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    /// Proof errors
    #[error("Proof error: {0}")]
    Proof(#[from] ProofError),

    /// Configuration errors
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Result type alias for 0-openclaw.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors related to the Gateway.
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Gateway not initialized")]
    NotInitialized,

    #[error("Gateway already running")]
    AlreadyRunning,

    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("Skill not found: {hash}")]
    SkillNotFound { hash: ContentHash },

    #[error("Router graph error: {0}")]
    RouterError(String),

    #[error("VM execution error: {0}")]
    VmError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Config error: {0}")]
    ConfigError(String),
}

impl From<SessionError> for GatewayError {
    fn from(err: SessionError) -> Self {
        GatewayError::SessionError(err.to_string())
    }
}

/// Errors related to Channels.
#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Rate limited, retry after {retry_after}ms")]
    RateLimited { retry_after: u64 },

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

/// Errors related to Skills.
#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Skill verification failed: {0}")]
    VerificationFailed(String),

    #[error("Skill execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid skill graph: {0}")]
    InvalidGraph(String),

    #[error("Skill composition error: {0}")]
    CompositionError(String),

    #[error("Skill already installed: {0}")]
    AlreadyInstalled(String),

    #[error("Unsafe operation detected: {op} - {reason}")]
    UnsafeOperation { op: String, reason: String },
}

/// Errors related to Sessions.
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found")]
    NotFound,

    #[error("Session expired")]
    Expired,

    #[error("Invalid session state: {0}")]
    InvalidState(String),

    #[error("Session update failed: {0}")]
    UpdateFailed(String),
}

/// Errors related to Proofs.
#[derive(Error, Debug)]
pub enum ProofError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid execution trace: {0}")]
    InvalidTrace(String),

    #[error("Confidence below threshold: {confidence} < {threshold}")]
    ConfidenceBelowThreshold { confidence: f32, threshold: f32 },

    #[error("Missing keypair")]
    MissingKeypair,

    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

/// Errors related to Configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    FileNotFound(String),

    #[error("Invalid config value for '{key}': {reason}")]
    InvalidValue { key: String, reason: String },

    #[error("Missing required config: {0}")]
    MissingRequired(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}
