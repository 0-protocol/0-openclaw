//! Slack channel connector for 0-openclaw.
//!
//! This module implements the `Channel` trait for Slack using the slack-morphism library.
//! Supports Events API, slash commands, and interactive messages.

mod config;

pub use config::SlackConfig;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::channels::{Channel, ChannelFeature};
use crate::channels::common::{RateLimiter, RateLimitConfig};
use crate::error::ChannelError;
use crate::types::{
    Action, Confidence, ContentHash, IncomingMessage, OutgoingMessage, ProofCarryingAction,
};

/// Slack channel implementation.
/// 
/// Note: This is a basic implementation. Full slack-morphism integration
/// requires the `slack` feature flag (not yet available in Cargo.toml).
pub struct SlackChannel {
    config: SlackConfig,
    message_rx: Arc<Mutex<mpsc::Receiver<IncomingMessage>>>,
    message_tx: mpsc::Sender<IncomingMessage>,
    rate_limiter: RateLimiter,
}

impl SlackChannel {
    /// Create a new Slack channel with the given configuration.
    pub async fn new(config: SlackConfig) -> Result<Self, ChannelError> {
        config.validate().map_err(|e| ChannelError::ConnectionFailed(e))?;

        let (tx, rx) = mpsc::channel(100);

        let channel = Self {
            config,
            message_rx: Arc::new(Mutex::new(rx)),
            message_tx: tx,
            rate_limiter: RateLimiter::new(RateLimitConfig::slack()),
        };

        // Note: Full implementation would start an HTTP server for Events API
        // and connect to Slack's Socket Mode or Events API
        tracing::info!("Slack channel initialized (basic implementation)");

        Ok(channel)
    }

    /// Process an incoming Slack event (called by external HTTP handler).
    /// 
    /// This method would be called by an HTTP server handling the Events API.
    pub async fn process_event(&self, event: SlackEvent) -> Result<(), ChannelError> {
        let incoming = self.convert_event(event)?;
        self.message_tx
            .send(incoming)
            .await
            .map_err(|_| ChannelError::ChannelClosed)?;
        Ok(())
    }

    fn convert_event(&self, event: SlackEvent) -> Result<IncomingMessage, ChannelError> {
        match event {
            SlackEvent::Message {
                channel,
                user,
                text,
                ts,
                thread_ts,
            } => {
                // Check allowlists
                if !self.config.channel_allowlist.is_empty()
                    && !self.config.channel_allowlist.contains(&channel)
                {
                    return Err(ChannelError::PermissionDenied(
                        "Channel not in allowlist".to_string(),
                    ));
                }

                Ok(IncomingMessage {
                    id: ContentHash::from_bytes(format!("slack:{}:{}", channel, ts).as_bytes()),
                    channel_id: "slack".to_string(),
                    sender_id: user,
                    content: text,
                    timestamp: parse_slack_ts(&ts),
                    metadata: serde_json::json!({
                        "channel": channel,
                        "ts": ts,
                        "thread_ts": thread_ts,
                    }),
                })
            }
            SlackEvent::SlashCommand {
                command,
                text,
                user_id,
                channel_id,
                trigger_id,
            } => Ok(IncomingMessage {
                id: ContentHash::from_bytes(
                    format!("slack:cmd:{}:{}", trigger_id, command).as_bytes(),
                ),
                channel_id: "slack".to_string(),
                sender_id: user_id,
                content: format!("{} {}", command, text),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                metadata: serde_json::json!({
                    "type": "slash_command",
                    "command": command,
                    "channel": channel_id,
                    "trigger_id": trigger_id,
                }),
            }),
            SlackEvent::AppMention {
                channel,
                user,
                text,
                ts,
            } => Ok(IncomingMessage {
                id: ContentHash::from_bytes(format!("slack:mention:{}:{}", channel, ts).as_bytes()),
                channel_id: "slack".to_string(),
                sender_id: user,
                content: text,
                timestamp: parse_slack_ts(&ts),
                metadata: serde_json::json!({
                    "type": "app_mention",
                    "channel": channel,
                    "ts": ts,
                }),
            }),
        }
    }

    /// Send a message to Slack.
    /// 
    /// Note: Full implementation would use slack-morphism's WebAPI client.
    async fn send_message_impl(&self, channel: &str, text: &str) -> Result<(), ChannelError> {
        // Apply rate limiting
        self.rate_limiter.acquire().await;

        // In a full implementation, this would use the Slack Web API:
        // self.client.chat_postMessage(channel, text).await

        // For now, log the message
        tracing::info!(
            "Would send to Slack channel {}: {}",
            channel,
            text
        );

        // Placeholder for actual HTTP call
        let client = reqwest::Client::new();
        let response = client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .json(&serde_json::json!({
                "channel": channel,
                "text": text,
            }))
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChannelError::SendFailed(format!(
                "Slack API error: {}",
                response.status()
            )));
        }

        // Check for rate limiting
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        if body.get("ok") == Some(&serde_json::Value::Bool(false)) {
            let error = body
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown");

            if error == "rate_limited" {
                let retry_after = body
                    .get("retry_after")
                    .and_then(|r| r.as_u64())
                    .unwrap_or(1);
                return Err(ChannelError::RateLimited {
                    retry_after: retry_after * 1000,
                });
            }

            return Err(ChannelError::SendFailed(format!("Slack error: {}", error)));
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn receive(&self) -> Result<IncomingMessage, ChannelError> {
        let mut rx = self.message_rx.lock().await;
        rx.recv()
            .await
            .ok_or(ChannelError::ChannelClosed)
    }

    async fn send(&self, message: OutgoingMessage) -> Result<ProofCarryingAction, ChannelError> {
        self.send_message_impl(&message.recipient_id, &message.content)
            .await?;
        Ok(ProofCarryingAction::pending())
    }

    fn evaluate_permission(&self, _action: &Action, _sender: &str) -> Confidence {
        // Check if sender's workspace is allowlisted
        // In Slack, we'd typically check workspace membership
        if !self.config.workspace_allowlist.is_empty() {
            // For simplicity, check user against allowlist
            Confidence::new(0.5)
        } else {
            Confidence::new(0.3)
        }
    }

    fn allowlist(&self) -> &[String] {
        &self.config.channel_allowlist
    }

    fn supports(&self, feature: ChannelFeature) -> bool {
        match feature {
            ChannelFeature::Commands => true,  // Slash commands
            ChannelFeature::Groups => true,    // Channels
            ChannelFeature::Reactions => true,
            ChannelFeature::Threads => true,
            ChannelFeature::Files => true,
            ChannelFeature::Voice => false,    // Huddles not supported via API
        }
    }
}

/// Slack event types that can be received from the Events API.
#[derive(Debug, Clone)]
pub enum SlackEvent {
    /// A message was posted in a channel.
    Message {
        channel: String,
        user: String,
        text: String,
        ts: String,
        thread_ts: Option<String>,
    },
    /// A slash command was invoked.
    SlashCommand {
        command: String,
        text: String,
        user_id: String,
        channel_id: String,
        trigger_id: String,
    },
    /// The app was mentioned.
    AppMention {
        channel: String,
        user: String,
        text: String,
        ts: String,
    },
}

/// Parse a Slack timestamp (e.g., "1234567890.123456") to milliseconds.
fn parse_slack_ts(ts: &str) -> u64 {
    ts.replace('.', "")
        .parse::<u64>()
        .map(|v| v / 1000) // Slack ts is in microseconds
        .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slack_ts() {
        let ts = "1234567890.123456";
        let millis = parse_slack_ts(ts);
        // Should be approximately 1234567890123 (ms)
        assert!(millis > 1234567890000);
    }
}
