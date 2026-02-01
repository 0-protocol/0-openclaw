//! Slack channel configuration.

use serde::{Deserialize, Serialize};

/// Configuration for the Slack channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot OAuth token (xoxb-...).
    pub bot_token: String,
    
    /// App-level token for Socket Mode (xapp-...).
    #[serde(default)]
    pub app_token: String,
    
    /// Signing secret for verifying requests.
    #[serde(default)]
    pub signing_secret: String,
    
    /// Allowlisted workspace IDs.
    #[serde(default)]
    pub workspace_allowlist: Vec<String>,
    
    /// Allowlisted channel IDs.
    #[serde(default)]
    pub channel_allowlist: Vec<String>,
    
    /// Whether to use Socket Mode (vs Events API HTTP).
    #[serde(default)]
    pub use_socket_mode: bool,
    
    /// Port for Events API HTTP server.
    #[serde(default = "default_port")]
    pub events_port: u16,
}

fn default_port() -> u16 {
    3000
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            app_token: String::new(),
            signing_secret: String::new(),
            workspace_allowlist: Vec::new(),
            channel_allowlist: Vec::new(),
            use_socket_mode: false,
            events_port: default_port(),
        }
    }
}

impl SlackConfig {
    /// Create a new config with the given bot token.
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            ..Default::default()
        }
    }

    /// Set the app token for Socket Mode.
    pub fn with_app_token(mut self, token: &str) -> Self {
        self.app_token = token.to_string();
        self.use_socket_mode = true;
        self
    }

    /// Set the signing secret.
    pub fn with_signing_secret(mut self, secret: &str) -> Self {
        self.signing_secret = secret.to_string();
        self
    }

    /// Set the workspace allowlist.
    pub fn with_workspace_allowlist(mut self, workspaces: Vec<String>) -> Self {
        self.workspace_allowlist = workspaces;
        self
    }

    /// Set the channel allowlist.
    pub fn with_channel_allowlist(mut self, channels: Vec<String>) -> Self {
        self.channel_allowlist = channels;
        self
    }

    /// Enable Socket Mode.
    pub fn with_socket_mode(mut self, enabled: bool) -> Self {
        self.use_socket_mode = enabled;
        self
    }

    /// Set the Events API port.
    pub fn with_events_port(mut self, port: u16) -> Self {
        self.events_port = port;
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.bot_token.is_empty() {
            return Err("Slack bot token is required".to_string());
        }
        
        // Bot tokens should start with xoxb-
        if !self.bot_token.starts_with("xoxb-") && !self.bot_token.starts_with("test_") {
            return Err("Invalid Slack bot token format (should start with xoxb-)".to_string());
        }

        if self.use_socket_mode && self.app_token.is_empty() {
            return Err("App token is required for Socket Mode".to_string());
        }

        if self.use_socket_mode && !self.app_token.starts_with("xapp-") && !self.app_token.starts_with("test_") {
            return Err("Invalid Slack app token format (should start with xapp-)".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = SlackConfig::default();
        assert!(config.validate().is_err());

        let config = SlackConfig::new("xoxb-test-token");
        assert!(config.validate().is_ok());

        let config = SlackConfig::new("invalid-token");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_socket_mode_validation() {
        let config = SlackConfig::new("xoxb-test")
            .with_socket_mode(true);
        assert!(config.validate().is_err()); // Missing app token

        let config = SlackConfig::new("xoxb-test")
            .with_app_token("xapp-test");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = SlackConfig::new("xoxb-token")
            .with_signing_secret("secret123")
            .with_workspace_allowlist(vec!["T123".to_string()])
            .with_channel_allowlist(vec!["C456".to_string()])
            .with_events_port(8080);

        assert_eq!(config.signing_secret, "secret123");
        assert_eq!(config.workspace_allowlist.len(), 1);
        assert_eq!(config.events_port, 8080);
    }

    #[test]
    fn test_config_serde() {
        let json = r#"{
            "bot_token": "xoxb-123",
            "app_token": "xapp-456",
            "signing_secret": "secret",
            "workspace_allowlist": ["T111"],
            "channel_allowlist": ["C222", "C333"],
            "use_socket_mode": true,
            "events_port": 4000
        }"#;

        let config: SlackConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.bot_token, "xoxb-123");
        assert!(config.use_socket_mode);
        assert_eq!(config.channel_allowlist.len(), 2);
    }
}
