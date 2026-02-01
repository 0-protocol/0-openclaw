//! Discord channel configuration.

use serde::{Deserialize, Serialize};

/// Configuration for the Discord channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot token from Discord Developer Portal.
    pub token: String,
    
    /// Application ID.
    #[serde(default)]
    pub application_id: u64,
    
    /// Allowlisted user IDs for DMs.
    #[serde(default)]
    pub dm_allowlist: Vec<String>,
    
    /// Allowlisted guild (server) IDs.
    #[serde(default)]
    pub guild_allowlist: Vec<u64>,
    
    /// Whether to register slash commands on startup.
    #[serde(default)]
    pub register_commands: bool,
    
    /// Command prefix for text commands (optional).
    #[serde(default)]
    pub command_prefix: Option<String>,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            application_id: 0,
            dm_allowlist: Vec::new(),
            guild_allowlist: Vec::new(),
            register_commands: true,
            command_prefix: None,
        }
    }
}

impl DiscordConfig {
    /// Create a new config with the given token.
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            ..Default::default()
        }
    }

    /// Set the application ID.
    pub fn with_application_id(mut self, id: u64) -> Self {
        self.application_id = id;
        self
    }

    /// Set the DM allowlist.
    pub fn with_dm_allowlist(mut self, users: Vec<String>) -> Self {
        self.dm_allowlist = users;
        self
    }

    /// Set the guild allowlist.
    pub fn with_guild_allowlist(mut self, guilds: Vec<u64>) -> Self {
        self.guild_allowlist = guilds;
        self
    }

    /// Enable or disable slash command registration.
    pub fn with_register_commands(mut self, register: bool) -> Self {
        self.register_commands = register;
        self
    }

    /// Set a command prefix for text commands.
    pub fn with_command_prefix(mut self, prefix: &str) -> Self {
        self.command_prefix = Some(prefix.to_string());
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.token.is_empty() {
            return Err("Discord token is required".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = DiscordConfig::default();
        assert!(config.validate().is_err());

        let config = DiscordConfig::new("test_token");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = DiscordConfig::new("token")
            .with_application_id(123456789)
            .with_dm_allowlist(vec!["user1".to_string()])
            .with_guild_allowlist(vec![111, 222])
            .with_register_commands(true)
            .with_command_prefix("!");

        assert_eq!(config.application_id, 123456789);
        assert_eq!(config.dm_allowlist.len(), 1);
        assert_eq!(config.guild_allowlist.len(), 2);
        assert_eq!(config.command_prefix, Some("!".to_string()));
    }

    #[test]
    fn test_config_serde() {
        let json = r#"{
            "token": "my_token",
            "application_id": 123,
            "dm_allowlist": ["user1"],
            "guild_allowlist": [456, 789],
            "register_commands": false
        }"#;

        let config: DiscordConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.token, "my_token");
        assert_eq!(config.application_id, 123);
        assert!(!config.register_commands);
    }
}
