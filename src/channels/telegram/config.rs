//! Telegram channel configuration.

use serde::{Deserialize, Serialize};

/// Configuration for the Telegram channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot API token from @BotFather.
    pub token: String,
    
    /// Bot username (without @).
    #[serde(default)]
    pub bot_username: String,
    
    /// Allowlisted user IDs.
    #[serde(default)]
    pub allowlist: Vec<String>,
    
    /// Policy for direct messages.
    #[serde(default)]
    pub dm_policy: DmPolicy,
    
    /// Policy for group messages.
    #[serde(default)]
    pub group_policy: GroupPolicy,
    
    /// Maximum messages per minute (rate limiting).
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

fn default_rate_limit() -> u32 {
    30
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            bot_username: String::new(),
            allowlist: Vec::new(),
            dm_policy: DmPolicy::default(),
            group_policy: GroupPolicy::default(),
            rate_limit: default_rate_limit(),
        }
    }
}

impl TelegramConfig {
    /// Create a new config with the given token.
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            ..Default::default()
        }
    }

    /// Set the bot username.
    pub fn with_username(mut self, username: &str) -> Self {
        self.bot_username = username.to_string();
        self
    }

    /// Add a user to the allowlist.
    pub fn with_allowlist(mut self, users: Vec<String>) -> Self {
        self.allowlist = users;
        self
    }

    /// Set the DM policy.
    pub fn with_dm_policy(mut self, policy: DmPolicy) -> Self {
        self.dm_policy = policy;
        self
    }

    /// Set the group policy.
    pub fn with_group_policy(mut self, policy: GroupPolicy) -> Self {
        self.group_policy = policy;
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.token.is_empty() {
            return Err("Telegram token is required".to_string());
        }
        // Token format: 123456789:ABCdefGHIjklMNOpqrsTUVwxyz
        if !self.token.contains(':') {
            return Err("Invalid Telegram token format".to_string());
        }
        Ok(())
    }
}

/// Policy for handling direct messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DmPolicy {
    /// Accept DMs from anyone.
    Open,
    
    /// Require a pairing code before accepting DMs.
    Pairing,
    
    /// Only accept DMs from allowlisted users.
    #[default]
    Allowlist,
}

/// Policy for handling group messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GroupPolicy {
    /// Only respond when mentioned.
    #[default]
    MentionOnly,
    
    /// Respond to all messages.
    Always,
    
    /// Ignore all group messages.
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = TelegramConfig::default();
        assert!(config.validate().is_err());

        let config = TelegramConfig::new("123456789:ABCdefGHIjklMNOpqrsTUVwxyz");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = TelegramConfig::new("token:123")
            .with_username("mybot")
            .with_allowlist(vec!["user1".to_string()])
            .with_dm_policy(DmPolicy::Open)
            .with_group_policy(GroupPolicy::Disabled);

        assert_eq!(config.bot_username, "mybot");
        assert_eq!(config.dm_policy, DmPolicy::Open);
        assert_eq!(config.group_policy, GroupPolicy::Disabled);
    }

    #[test]
    fn test_config_serde() {
        let json = r#"{
            "token": "123:abc",
            "bot_username": "testbot",
            "allowlist": ["user1", "user2"],
            "dm_policy": "open",
            "group_policy": "mentiononly"
        }"#;

        let config: TelegramConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.token, "123:abc");
        assert_eq!(config.dm_policy, DmPolicy::Open);
    }
}
