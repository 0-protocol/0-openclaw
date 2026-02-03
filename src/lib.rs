//! # 0-openclaw
//!
//! Proof-carrying AI assistant built with 0-lang.
//!
//! ## Overview
//!
//! 0-openclaw is the first AI assistant where every action carries cryptographic
//! proof of the decision path. Instead of trusting that code does what it claims,
//! you can verify it.
//!
//! ## Core Concepts
//!
//! - **Proof-Carrying Actions**: Every action includes execution trace and signature
//! - **Content-Addressed Logic**: Same hash = same behavior, always
//! - **Confidence Scores**: Probabilistic trust instead of boolean gates
//! - **Composable Skills**: Verified, shareable graph modules
//!
//! ## Example
//!
//! ```rust,ignore
//! use zero_openclaw::{Gateway, GatewayConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = GatewayConfig::load("config.json")?;
//!     let gateway = Gateway::new(config)?;
//!     gateway.run().await?;
//!     Ok(())
//! }
//! ```

pub mod types;
pub mod error;
pub mod runtime;
pub mod gateway;
pub mod channels;
pub mod skills;
pub mod cli;

// Re-export commonly used types
pub use types::{
    ContentHash,
    Confidence,
    IncomingMessage,
    OutgoingMessage,
    ProofCarryingAction,
    Action,
};
pub use error::{Error, Result};
pub use runtime::{GraphInterpreter, Graph, Value, ExecutionResult};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
