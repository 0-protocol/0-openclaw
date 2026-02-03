//! Channel connectors for 0-openclaw.
//!
//! Channels connect 0-openclaw to messaging platforms like Telegram, Discord, and Slack.
//! Each channel implements the `Channel` trait for standardized message handling.
//!
//! ## Architecture
//!
//! Channel code is kept minimal - acting as thin API adapters. Message processing logic
//! is defined in 0-lang graphs:
//!
//! ```text
//! ┌─────────────────┐     ┌────────────────────┐
//! │  Channel API    │────▶│  0-lang Graph      │
//! │  (Rust adapter) │     │  (Processing)      │
//! │  ~100 lines     │     │  graphs/channels/  │
//! └─────────────────┘     └────────────────────┘
//! ```
//!
//! - `graphs/channels/telegram.0` - Telegram message processing
//! - `graphs/channels/discord.0` - Discord message processing  
//! - `graphs/channels/slack.0` - Slack message processing
//!
//! ## Supported Channels
//!
//! | Channel  | Status      | Feature Flag | Graph File          |
//! |----------|-------------|--------------|---------------------|
//! | Telegram | Implemented | `telegram`   | `telegram.0`        |
//! | Discord  | Implemented | `discord`    | `discord.0`         |
//! | Slack    | Implemented | -            | `slack.0`           |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zero_openclaw::channels::{Channel, telegram::TelegramChannel, TelegramConfig};
//!
//! let config = TelegramConfig::new("your_bot_token")
//!     .with_dm_policy(DmPolicy::Allowlist)
//!     .with_allowlist(vec!["123456789".to_string()]);
//!
//! let channel = TelegramChannel::new(config).await?;
//!
//! // Receive messages
//! loop {
//!     let msg = channel.receive().await?;
//!     println!("Received: {}", msg.content);
//! }
//! ```
//!
//! ## Implementation Status
//!
//! This module is implemented by **Agent #8**.
//!
//! See: `AGENT-8-0OPENCLAW-CHANNELS.md`

use async_trait::async_trait;
use crate::types::{Action, Confidence, IncomingMessage, OutgoingMessage, ProofCarryingAction};
use crate::error::ChannelError;

// Submodules
pub mod common;
pub mod telegram;
pub mod discord;
pub mod slack;

// Re-export commonly used types
pub use telegram::{TelegramChannel, TelegramConfig, DmPolicy, GroupPolicy};
pub use discord::{DiscordChannel, DiscordConfig};
pub use slack::{SlackChannel, SlackConfig, SlackEvent};
pub use common::{RateLimiter, RateLimitConfig, RetryPolicy};

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

    /// Add users to the allowlist.
    pub fn with_allowlist(mut self, users: Vec<String>) -> Self {
        self.allowlist = users;
        self
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

/// Registry for managing multiple channels.
pub struct ChannelRegistry {
    channels: std::collections::HashMap<String, Box<dyn Channel>>,
}

impl ChannelRegistry {
    /// Create a new empty channel registry.
    pub fn new() -> Self {
        Self {
            channels: std::collections::HashMap::new(),
        }
    }

    /// Register a channel.
    pub fn register<C: Channel + 'static>(&mut self, channel: C) {
        self.channels.insert(channel.name().to_string(), Box::new(channel));
    }

    /// Get a channel by name.
    pub fn get(&self, name: &str) -> Option<&dyn Channel> {
        self.channels.get(name).map(|c| c.as_ref())
    }

    /// List all registered channel names.
    pub fn list(&self) -> Vec<&str> {
        self.channels.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a channel is registered.
    pub fn has(&self, name: &str) -> bool {
        self.channels.contains_key(name)
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
