//! Gateway configuration.
//!
//! This module provides configuration management for the Gateway,
//! including server settings, paths, and runtime options.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::ConfigError;

/// Gateway configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Path to the main router graph
    #[serde(default = "default_router_graph_path")]
    pub router_graph_path: PathBuf,

    /// Path to the keypair for signing proofs
    #[serde(default = "default_keypair_path")]
    pub keypair_path: PathBuf,

    /// Path to the skills directory
    #[serde(default = "default_skills_path")]
    pub skills_path: PathBuf,

    /// Path to the graphs directory
    #[serde(default = "default_graphs_path")]
    pub graphs_path: PathBuf,

    /// Session configuration
    #[serde(default)]
    pub session: SessionConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Enable CORS
    #[serde(default = "default_true")]
    pub cors_enabled: bool,

    /// Maximum WebSocket connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

/// Session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout in seconds
    #[serde(default = "default_session_timeout")]
    pub timeout_seconds: u64,

    /// Maximum sessions per user
    #[serde(default = "default_max_sessions_per_user")]
    pub max_per_user: usize,

    /// Initial trust score for new sessions
    #[serde(default = "default_initial_trust")]
    pub initial_trust: f32,

    /// Trust score decay rate (per interaction without positive feedback)
    #[serde(default = "default_trust_decay")]
    pub trust_decay: f32,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable JSON logging format
    #[serde(default)]
    pub json_format: bool,

    /// Log file path (None for stdout only)
    pub file_path: Option<PathBuf>,
}

// Default value functions
fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_true() -> bool {
    true
}

fn default_max_connections() -> usize {
    1000
}

fn default_router_graph_path() -> PathBuf {
    PathBuf::from("graphs/core/router.0")
}

fn default_keypair_path() -> PathBuf {
    PathBuf::from("config/keypair.bin")
}

fn default_skills_path() -> PathBuf {
    PathBuf::from("graphs/skills")
}

fn default_graphs_path() -> PathBuf {
    PathBuf::from("graphs")
}

fn default_session_timeout() -> u64 {
    3600 // 1 hour
}

fn default_max_sessions_per_user() -> usize {
    10
}

fn default_initial_trust() -> f32 {
    0.5
}

fn default_trust_decay() -> f32 {
    0.01
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            router_graph_path: default_router_graph_path(),
            keypair_path: default_keypair_path(),
            skills_path: default_skills_path(),
            graphs_path: default_graphs_path(),
            session: SessionConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_enabled: default_true(),
            max_connections: default_max_connections(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: default_session_timeout(),
            max_per_user: default_max_sessions_per_user(),
            initial_trust: default_initial_trust(),
            trust_decay: default_trust_decay(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json_format: false,
            file_path: None,
        }
    }
}

impl GatewayConfig {
    /// Create a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a JSON file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }

        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        serde_json::from_str(&contents)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Save configuration to a JSON file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        std::fs::write(path, contents)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Note: port 0 is valid - it means "let the OS assign a port"

        // Validate trust values
        if self.session.initial_trust < 0.0 || self.session.initial_trust > 1.0 {
            return Err(ConfigError::InvalidValue {
                key: "session.initial_trust".to_string(),
                reason: "Initial trust must be between 0.0 and 1.0".to_string(),
            });
        }

        if self.session.trust_decay < 0.0 || self.session.trust_decay > 1.0 {
            return Err(ConfigError::InvalidValue {
                key: "session.trust_decay".to_string(),
                reason: "Trust decay must be between 0.0 and 1.0".to_string(),
            });
        }

        Ok(())
    }

    /// Get the server address string.
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Create a configuration for testing.
    pub fn for_testing() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0, // OS will assign a port
                cors_enabled: true,
                max_connections: 10,
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GatewayConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_config_validation() {
        let mut config = GatewayConfig::default();
        assert!(config.validate().is_ok());

        config.session.initial_trust = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_addr() {
        let config = GatewayConfig::default();
        assert_eq!(config.server_addr(), "127.0.0.1:8080");
    }

    #[test]
    fn test_config_serialization() {
        let config = GatewayConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: GatewayConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.server.port, restored.server.port);
    }
}
