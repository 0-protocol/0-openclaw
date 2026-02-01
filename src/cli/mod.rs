//! CLI module for 0-openclaw.
//!
//! This module provides utilities for the command-line interface.
//!
//! ## Implementation Status
//!
//! This module is implemented by **Agent #10**.
//!
//! See: `AGENT-10-0OPENCLAW-CLI-INTEGRATION.md`

use std::path::{Path, PathBuf};
use crate::error::ConfigError;

// Submodules to be implemented by Agent #10
// pub mod commands;
// pub mod config;

/// Expand tilde (~) in paths.
pub fn expand_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(&path_str[2..]);
        }
    }
    path.to_path_buf()
}

/// Configuration for 0-openclaw.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Gateway settings
    pub gateway: GatewayConfig,
    /// Channel configurations
    pub channels: Vec<ChannelConfig>,
    /// Skill paths
    pub skills: Vec<String>,
}

/// Gateway configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewayConfig {
    /// Port to listen on
    pub port: u16,
    /// Address to bind to
    pub bind: String,
    /// Path to keypair file
    pub keypair_path: String,
}

/// Channel configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelConfig {
    /// Channel type (telegram, discord, slack)
    #[serde(rename = "type")]
    pub channel_type: String,
    /// Whether the channel is enabled
    pub enabled: bool,
    /// Channel token (if applicable)
    pub token: Option<String>,
    /// Allowlist of user IDs
    pub allowlist: Vec<String>,
}

impl Config {
    /// Load configuration from a file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let expanded = expand_path(path);
        let content = std::fs::read_to_string(&expanded)
            .map_err(|_| ConfigError::FileNotFound(expanded.display().to_string()))?;
        
        serde_json::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Save configuration to a file.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let expanded = expand_path(path);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        std::fs::write(&expanded, content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        Ok(())
    }

    /// Create a default configuration.
    pub fn default_config() -> Self {
        Self {
            gateway: GatewayConfig {
                port: 18789,
                bind: "127.0.0.1".to_string(),
                keypair_path: "~/.0-openclaw/keypair".to_string(),
            },
            channels: Vec::new(),
            skills: vec!["graphs/skills/echo.0".to_string()],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::default_config()
    }
}
