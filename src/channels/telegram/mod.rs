//! Telegram channel connector for 0-openclaw.
//!
//! This module implements the `Channel` trait for Telegram using the teloxide library.
//! Supports both private messages and group chats with configurable policies.

#[cfg(feature = "telegram")]
mod implementation;
mod config;

pub use config::{TelegramConfig, DmPolicy, GroupPolicy};

#[cfg(feature = "telegram")]
pub use implementation::TelegramChannel;

#[cfg(not(feature = "telegram"))]
pub use stub::TelegramChannel;

/// Stub implementation when telegram feature is disabled.
#[cfg(not(feature = "telegram"))]
mod stub {
    use async_trait::async_trait;
    use crate::channels::{Channel, ChannelFeature};
    use crate::error::ChannelError;
    use crate::types::{Action, Confidence, IncomingMessage, OutgoingMessage, ProofCarryingAction};
    use super::TelegramConfig;

    /// Stub TelegramChannel when feature is disabled.
    pub struct TelegramChannel {
        config: TelegramConfig,
    }

    impl TelegramChannel {
        /// Create a new stub channel (returns error).
        pub async fn new(config: TelegramConfig) -> Result<Self, ChannelError> {
            tracing::warn!("Telegram support not compiled in. Enable with --features telegram");
            Ok(Self { config })
        }
    }

    #[async_trait]
    impl Channel for TelegramChannel {
        fn name(&self) -> &str {
            "telegram"
        }

        async fn receive(&self) -> Result<IncomingMessage, ChannelError> {
            Err(ChannelError::ConnectionFailed(
                "Telegram feature not enabled. Compile with --features telegram".to_string()
            ))
        }

        async fn send(&self, _message: OutgoingMessage) -> Result<ProofCarryingAction, ChannelError> {
            Err(ChannelError::ConnectionFailed(
                "Telegram feature not enabled. Compile with --features telegram".to_string()
            ))
        }

        fn evaluate_permission(&self, _action: &Action, sender: &str) -> Confidence {
            if self.config.allowlist.contains(&sender.to_string()) {
                Confidence::new(0.95)
            } else {
                Confidence::new(0.3)
            }
        }

        fn allowlist(&self) -> &[String] {
            &self.config.allowlist
        }

        fn supports(&self, _feature: ChannelFeature) -> bool {
            false
        }
    }
}

#[cfg(feature = "telegram")]
mod implementation {
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};
    use teloxide::prelude::*;
    use teloxide::types::ChatId;
    
    use crate::channels::{Channel, ChannelFeature};
    use crate::channels::common::{RateLimiter, RateLimitConfig};
    use crate::error::ChannelError;
    use crate::types::{
        Action, Confidence, ContentHash, IncomingMessage, OutgoingMessage, ProofCarryingAction,
    };
    use super::{TelegramConfig, DmPolicy, GroupPolicy};

    /// Telegram channel implementation using teloxide.
    pub struct TelegramChannel {
        bot: Bot,
        config: TelegramConfig,
        message_rx: Arc<Mutex<mpsc::Receiver<IncomingMessage>>>,
        rate_limiter: RateLimiter,
    }

    impl TelegramChannel {
        /// Create a new Telegram channel with the given configuration.
        pub async fn new(config: TelegramConfig) -> Result<Self, ChannelError> {
            let bot = Bot::new(&config.token);
            let (tx, rx) = mpsc::channel(100);

            let channel = Self {
                bot: bot.clone(),
                config: config.clone(),
                message_rx: Arc::new(Mutex::new(rx)),
                rate_limiter: RateLimiter::new(RateLimitConfig::telegram()),
            };

            // Start the message listener in a background task
            Self::start_listener(bot, tx, config);

            Ok(channel)
        }

        fn start_listener(
            bot: Bot, 
            tx: mpsc::Sender<IncomingMessage>, 
            config: TelegramConfig
        ) {
            tokio::spawn(async move {
                teloxide::repl(bot, move |bot: Bot, msg: Message| {
                    let tx = tx.clone();
                    let config = config.clone();

                    async move {
                        // Check permissions based on policy
                        if !Self::check_permission_static(&msg, &config) {
                            tracing::debug!(
                                "Ignoring message from {} due to policy",
                                msg.from().map(|u| u.id.to_string()).unwrap_or_default()
                            );
                            return Ok(());
                        }

                        // Convert to IncomingMessage
                        let incoming = Self::convert_message(&msg);

                        // Send to channel
                        if tx.send(incoming).await.is_err() {
                            tracing::error!("Failed to send message to channel queue");
                        }

                        Ok(())
                    }
                })
                .await;
            });
        }

        fn check_permission_static(msg: &Message, config: &TelegramConfig) -> bool {
            let sender_id = msg
                .from()
                .map(|u| u.id.to_string())
                .unwrap_or_default();

            if msg.chat.is_private() {
                // DM policy check
                match config.dm_policy {
                    DmPolicy::Open => true,
                    DmPolicy::Allowlist => config.allowlist.contains(&sender_id),
                    DmPolicy::Pairing => {
                        // For pairing, we need to check if the user has a valid pairing code
                        // This would typically check against a pairing store
                        // For now, fall back to allowlist
                        config.allowlist.contains(&sender_id)
                    }
                }
            } else {
                // Group policy check
                match config.group_policy {
                    GroupPolicy::Disabled => false,
                    GroupPolicy::MentionOnly => {
                        // Check if bot was mentioned
                        msg.text()
                            .map(|t| {
                                t.contains(&format!("@{}", config.bot_username))
                                    || msg.reply_to_message().is_some()
                            })
                            .unwrap_or(false)
                    }
                    GroupPolicy::Always => true,
                }
            }
        }

        fn convert_message(msg: &Message) -> IncomingMessage {
            let content = msg
                .text()
                .or(msg.caption())
                .unwrap_or("")
                .to_string();

            let chat_type = if msg.chat.is_private() {
                "private"
            } else if msg.chat.is_group() {
                "group"
            } else if msg.chat.is_supergroup() {
                "supergroup"
            } else {
                "channel"
            };

            IncomingMessage {
                id: ContentHash::from_bytes(
                    format!("telegram:{}:{}", msg.chat.id.0, msg.id.0).as_bytes(),
                ),
                channel_id: "telegram".to_string(),
                sender_id: msg
                    .from()
                    .map(|u| u.id.to_string())
                    .unwrap_or_default(),
                content,
                timestamp: msg.date.timestamp_millis() as u64,
                metadata: serde_json::json!({
                    "chat_id": msg.chat.id.0,
                    "message_id": msg.id.0,
                    "chat_type": chat_type,
                    "username": msg.from().and_then(|u| u.username.clone()),
                    "first_name": msg.from().map(|u| u.first_name.clone()),
                    "reply_to_message_id": msg.reply_to_message().map(|m| m.id.0),
                }),
            }
        }
    }

    #[async_trait]
    impl Channel for TelegramChannel {
        fn name(&self) -> &str {
            "telegram"
        }

        async fn receive(&self) -> Result<IncomingMessage, ChannelError> {
            let mut rx = self.message_rx.lock().await;
            rx.recv()
                .await
                .ok_or(ChannelError::ChannelClosed)
        }

        async fn send(&self, message: OutgoingMessage) -> Result<ProofCarryingAction, ChannelError> {
            // Apply rate limiting
            self.rate_limiter.acquire().await;

            // Parse chat_id from recipient
            let chat_id: i64 = message
                .recipient_id
                .parse()
                .map_err(|e| ChannelError::InvalidMessage(format!("Invalid chat_id: {}", e)))?;

            // Send the message
            self.bot
                .send_message(ChatId(chat_id), &message.content)
                .await
                .map_err(|e| {
                    // Check for rate limiting
                    let error_str = e.to_string();
                    if error_str.contains("429") || error_str.contains("Too Many Requests") {
                        // Extract retry_after if possible
                        ChannelError::RateLimited { retry_after: 1000 }
                    } else {
                        ChannelError::SendFailed(e.to_string())
                    }
                })?;

            // Return a pending PCA (actual proof is generated by Gateway)
            Ok(ProofCarryingAction::pending())
        }

        fn evaluate_permission(&self, _action: &Action, sender: &str) -> Confidence {
            if self.config.allowlist.contains(&sender.to_string()) {
                Confidence::new(0.95)
            } else {
                Confidence::new(0.3)
            }
        }

        fn allowlist(&self) -> &[String] {
            &self.config.allowlist
        }

        fn supports(&self, feature: ChannelFeature) -> bool {
            match feature {
                ChannelFeature::Commands => true,
                ChannelFeature::Groups => true,
                ChannelFeature::Reactions => true,
                ChannelFeature::Threads => true, // Reply threads
                ChannelFeature::Files => true,
                ChannelFeature::Voice => true,
            }
        }
    }
}
