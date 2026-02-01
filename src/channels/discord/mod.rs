//! Discord channel connector for 0-openclaw.
//!
//! This module implements the `Channel` trait for Discord using the serenity library.
//! Supports slash commands, direct messages, and guild messages.

#[cfg(feature = "discord")]
mod implementation;
mod config;

pub use config::DiscordConfig;

#[cfg(feature = "discord")]
pub use implementation::DiscordChannel;

#[cfg(not(feature = "discord"))]
pub use stub::DiscordChannel;

/// Stub implementation when discord feature is disabled.
#[cfg(not(feature = "discord"))]
mod stub {
    use async_trait::async_trait;
    use crate::channels::{Channel, ChannelFeature};
    use crate::error::ChannelError;
    use crate::types::{Action, Confidence, IncomingMessage, OutgoingMessage, ProofCarryingAction};
    use super::DiscordConfig;

    /// Stub DiscordChannel when feature is disabled.
    pub struct DiscordChannel {
        config: DiscordConfig,
    }

    impl DiscordChannel {
        /// Create a new stub channel.
        pub async fn new(config: DiscordConfig) -> Result<Self, ChannelError> {
            tracing::warn!("Discord support not compiled in. Enable with --features discord");
            Ok(Self { config })
        }
    }

    #[async_trait]
    impl Channel for DiscordChannel {
        fn name(&self) -> &str {
            "discord"
        }

        async fn receive(&self) -> Result<IncomingMessage, ChannelError> {
            Err(ChannelError::ConnectionFailed(
                "Discord feature not enabled. Compile with --features discord".to_string()
            ))
        }

        async fn send(&self, _message: OutgoingMessage) -> Result<ProofCarryingAction, ChannelError> {
            Err(ChannelError::ConnectionFailed(
                "Discord feature not enabled. Compile with --features discord".to_string()
            ))
        }

        fn evaluate_permission(&self, _action: &Action, sender: &str) -> Confidence {
            if self.config.dm_allowlist.contains(&sender.to_string()) {
                Confidence::new(0.95)
            } else {
                Confidence::new(0.3)
            }
        }

        fn allowlist(&self) -> &[String] {
            &self.config.dm_allowlist
        }

        fn supports(&self, _feature: ChannelFeature) -> bool {
            false
        }
    }
}

#[cfg(feature = "discord")]
mod implementation {
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex, RwLock};
    use serenity::prelude::*;
    use serenity::model::prelude::*;
    use serenity::model::application::Interaction;
    
    use crate::channels::{Channel, ChannelFeature};
    use crate::channels::common::{RateLimiter, RateLimitConfig};
    use crate::error::ChannelError;
    use crate::types::{
        Action, Confidence, ContentHash, IncomingMessage, OutgoingMessage, ProofCarryingAction,
    };
    use super::DiscordConfig;

    /// Discord channel implementation using serenity.
    pub struct DiscordChannel {
        http: Arc<serenity::http::Http>,
        config: DiscordConfig,
        message_rx: Arc<Mutex<mpsc::Receiver<IncomingMessage>>>,
        rate_limiter: RateLimiter,
    }

    /// Event handler for Discord events.
    struct Handler {
        tx: mpsc::Sender<IncomingMessage>,
        config: DiscordConfig,
    }

    #[async_trait]
    impl EventHandler for Handler {
        async fn message(&self, _ctx: Context, msg: serenity::model::channel::Message) {
            // Skip bot messages
            if msg.author.bot {
                return;
            }

            // Check if we should process this message
            if !self.should_process_message(&msg) {
                return;
            }

            let incoming = self.convert_message(&msg);
            if self.tx.send(incoming).await.is_err() {
                tracing::error!("Failed to send Discord message to channel queue");
            }
        }

        async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
            if let Interaction::Command(command) = interaction {
                // Check permissions for slash commands
                let user_id = command.user.id.to_string();
                if !self.config.dm_allowlist.is_empty() 
                    && !self.config.dm_allowlist.contains(&user_id) 
                {
                    // Respond with permission denied
                    let _ = command
                        .create_response(&ctx.http, serenity::builder::CreateInteractionResponse::Message(
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .content("You don't have permission to use this command.")
                                .ephemeral(true)
                        ))
                        .await;
                    return;
                }

                // Build content from command and options
                let options_str: Vec<String> = command
                    .data
                    .options
                    .iter()
                    .map(|o| format!("{}={:?}", o.name, o.value))
                    .collect();

                let content = format!("/{} {}", command.data.name, options_str.join(" "));

                let incoming = IncomingMessage {
                    id: ContentHash::from_bytes(
                        format!("discord:cmd:{}", command.id.get()).as_bytes(),
                    ),
                    channel_id: "discord".to_string(),
                    sender_id: user_id,
                    content,
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    metadata: serde_json::json!({
                        "type": "slash_command",
                        "command": command.data.name,
                        "interaction_id": command.id.get().to_string(),
                        "channel_id": command.channel_id.get().to_string(),
                        "guild_id": command.guild_id.map(|g| g.get().to_string()),
                    }),
                };

                if self.tx.send(incoming).await.is_err() {
                    tracing::error!("Failed to send Discord command to channel queue");
                }

                // Acknowledge the command
                let _ = command
                    .create_response(&ctx.http, serenity::builder::CreateInteractionResponse::Acknowledge)
                    .await;
            }
        }

        async fn ready(&self, _ctx: Context, ready: Ready) {
            tracing::info!("Discord bot ready as {}", ready.user.name);
        }
    }

    impl Handler {
        fn should_process_message(&self, msg: &serenity::model::channel::Message) -> bool {
            let user_id = msg.author.id.to_string();

            // Check guild allowlist if in a guild
            if let Some(guild_id) = msg.guild_id {
                if !self.config.guild_allowlist.is_empty()
                    && !self.config.guild_allowlist.contains(&guild_id.get())
                {
                    return false;
                }
            }

            // For DMs, check DM allowlist
            if msg.guild_id.is_none() {
                if !self.config.dm_allowlist.is_empty()
                    && !self.config.dm_allowlist.contains(&user_id)
                {
                    return false;
                }
            }

            true
        }

        fn convert_message(&self, msg: &serenity::model::channel::Message) -> IncomingMessage {
            IncomingMessage {
                id: ContentHash::from_bytes(
                    format!("discord:{}:{}", msg.channel_id.get(), msg.id.get()).as_bytes(),
                ),
                channel_id: "discord".to_string(),
                sender_id: msg.author.id.to_string(),
                content: msg.content.clone(),
                timestamp: msg.timestamp.timestamp_millis() as u64,
                metadata: serde_json::json!({
                    "channel_id": msg.channel_id.get().to_string(),
                    "guild_id": msg.guild_id.map(|g| g.get().to_string()),
                    "message_id": msg.id.get().to_string(),
                    "username": msg.author.name.clone(),
                    "discriminator": msg.author.discriminator,
                    "is_dm": msg.guild_id.is_none(),
                }),
            }
        }
    }

    impl DiscordChannel {
        /// Create a new Discord channel with the given configuration.
        pub async fn new(config: DiscordConfig) -> Result<Self, ChannelError> {
            let (tx, rx) = mpsc::channel(100);

            let intents = GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;

            let handler = Handler {
                tx,
                config: config.clone(),
            };

            let mut client = Client::builder(&config.token, intents)
                .event_handler(handler)
                .await
                .map_err(|e| ChannelError::ConnectionFailed(e.to_string()))?;

            let http = client.http.clone();

            // Start the client in a background task
            tokio::spawn(async move {
                if let Err(e) = client.start().await {
                    tracing::error!("Discord client error: {:?}", e);
                }
            });

            Ok(Self {
                http,
                config,
                message_rx: Arc::new(Mutex::new(rx)),
                rate_limiter: RateLimiter::new(RateLimitConfig::discord()),
            })
        }
    }

    #[async_trait]
    impl Channel for DiscordChannel {
        fn name(&self) -> &str {
            "discord"
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

            // Parse channel_id from recipient
            let channel_id: u64 = message
                .recipient_id
                .parse()
                .map_err(|e| ChannelError::InvalidMessage(format!("Invalid channel_id: {}", e)))?;

            // Send the message
            let channel = ChannelId::new(channel_id);
            channel
                .send_message(&self.http, serenity::builder::CreateMessage::new().content(&message.content))
                .await
                .map_err(|e| {
                    let error_str = e.to_string();
                    if error_str.contains("rate limit") {
                        ChannelError::RateLimited { retry_after: 1000 }
                    } else {
                        ChannelError::SendFailed(e.to_string())
                    }
                })?;

            Ok(ProofCarryingAction::pending())
        }

        fn evaluate_permission(&self, _action: &Action, sender: &str) -> Confidence {
            if self.config.dm_allowlist.contains(&sender.to_string()) {
                Confidence::new(0.95)
            } else {
                Confidence::new(0.3)
            }
        }

        fn allowlist(&self) -> &[String] {
            &self.config.dm_allowlist
        }

        fn supports(&self, feature: ChannelFeature) -> bool {
            match feature {
                ChannelFeature::Commands => true,
                ChannelFeature::Groups => true,  // Guilds
                ChannelFeature::Reactions => true,
                ChannelFeature::Threads => true,
                ChannelFeature::Files => true,
                ChannelFeature::Voice => false,  // Not implemented yet
            }
        }
    }
}
