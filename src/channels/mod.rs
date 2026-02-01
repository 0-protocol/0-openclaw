//! Channel connectors for 0-openclaw.
//!
//! Channels connect 0-openclaw to messaging platforms like Telegram, Discord, and Slack.
//! Each channel implements the `Channel` trait for standardized message handling.
//!
//! ## Supported Channels
//!
//! | Channel  | Status      | Feature Flag |
//! |----------|-------------|--------------|
//! | Telegram | Pending     | `telegram`   |
//! | Discord  | Pending     | `discord`    |
//! | Slack    | Pending     | -            |
//!
//! ## Implementation Status
//!
//! This module is implemented by **Agent #8**.
//!
//! See: `AGENT-8-0OPENCLAW-CHANNELS.md`

use async_trait::async_trait;
use crate::types::{Action, Confidence, IncomingMessage, OutgoingMessage, ProofCarryingAction};
use crate::error::ChannelError;

/// Channel features that may or may not be supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelFeature {
    /// Slash/bot commands
    Commands,
    /// Group chats
    Groups,
    /// Message reactions
    Reactions,
    /// Threaded replies
    Threads,
    /// File attachments
    Files,
    /// Voice messages
    Voice,
}

/// Trait that all channel connectors must implement.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Get the channel identifier (e.g., "telegram", "discord").
    fn name(&self) -> &str;

    /// Receive the next message from the channel.
    ///
    /// This is a blocking call that waits for the next message.
    async fn receive(&self) -> Result<IncomingMessage, ChannelError>;

    /// Send a message to the channel.
    ///
    /// Returns a Proof-Carrying Action indicating the result.
    async fn send(&self, message: OutgoingMessage) -> Result<ProofCarryingAction, ChannelError>;

    /// Evaluate permission for an action.
    ///
    /// Returns a confidence score based on the sender and action.
    fn evaluate_permission(&self, action: &Action, sender: &str) -> Confidence;

    /// Get the channel's allowlist.
    fn allowlist(&self) -> &[String];

    /// Check if the channel supports a feature.
    fn supports(&self, feature: ChannelFeature) -> bool;
}

// Submodules to be implemented by Agent #8
// pub mod telegram;
// pub mod discord;
// pub mod slack;

/// Placeholder channel for testing.
pub struct TestChannel {
    name: String,
    allowlist: Vec<String>,
}

impl TestChannel {
    /// Create a new test channel.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            allowlist: Vec::new(),
        }
    }
}

#[async_trait]
impl Channel for TestChannel {
    fn name(&self) -> &str {
        &self.name
    }

    async fn receive(&self) -> Result<IncomingMessage, ChannelError> {
        Err(ChannelError::ChannelClosed)
    }

    async fn send(&self, _message: OutgoingMessage) -> Result<ProofCarryingAction, ChannelError> {
        Ok(ProofCarryingAction::pending())
    }

    fn evaluate_permission(&self, _action: &Action, _sender: &str) -> Confidence {
        Confidence::neutral()
    }

    fn allowlist(&self) -> &[String] {
        &self.allowlist
    }

    fn supports(&self, _feature: ChannelFeature) -> bool {
        false
    }
}
