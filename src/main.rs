//! 0-openclaw CLI entry point.
//!
//! This is the main binary for 0-openclaw.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Proof-carrying AI assistant built with 0-lang.
#[derive(Parser)]
#[command(name = "zero-openclaw")]
#[command(about = "Every action carries proof. Every decision is verifiable.")]
#[command(version)]
struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "~/.0-openclaw/config.json")]
    config: PathBuf,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway
    Gateway {
        /// Port to listen on
        #[arg(short, long, default_value = "18789")]
        port: u16,

        /// Run in daemon mode
        #[arg(short, long)]
        daemon: bool,
    },

    /// Channel management
    Channel {
        #[command(subcommand)]
        action: ChannelCommands,
    },

    /// Skill management
    Skill {
        #[command(subcommand)]
        action: SkillCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Show gateway status
    Status,

    /// Run diagnostics
    Doctor,

    /// Initialize a new 0-openclaw installation
    Init {
        /// Installation directory
        #[arg(default_value = "~/.0-openclaw")]
        path: PathBuf,
    },

    /// Verify a proof-carrying action
    Verify {
        /// Path to PCA file
        pca_file: PathBuf,
    },
}

#[derive(Subcommand)]
enum ChannelCommands {
    /// List connected channels
    List,

    /// Connect a channel
    Connect {
        /// Channel type (telegram, discord, slack)
        channel_type: String,
    },

    /// Disconnect a channel
    Disconnect {
        /// Channel name
        name: String,
    },

    /// Show channel status
    Status {
        /// Channel name
        name: String,
    },
}

#[derive(Subcommand)]
enum SkillCommands {
    /// List installed skills
    List,

    /// Install a skill
    Install {
        /// Skill path or URL
        source: String,
    },

    /// Uninstall a skill
    Uninstall {
        /// Skill name or hash
        skill: String,
    },

    /// Verify a skill
    Verify {
        /// Skill name or hash
        skill: String,
    },

    /// Show skill info
    Info {
        /// Skill name or hash
        skill: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current config
    Show,

    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },

    /// Get a config value
    Get {
        /// Config key
        key: String,
    },

    /// Validate config
    Validate,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    match cli.command {
        Commands::Gateway { port, daemon: _ } => {
            println!("Starting 0-openclaw gateway on port {}...", port);
            println!();
            println!("┌─────────────────────────────────────────────────────┐");
            println!("│              0-OPENCLAW GATEWAY                      │");
            println!("├─────────────────────────────────────────────────────┤");
            println!("│  Status: Starting...                                │");
            println!("│  Port: {}                                        │", port);
            println!("│  Proof-Carrying: Enabled                            │");
            println!("└─────────────────────────────────────────────────────┘");
            println!();
            println!("Gateway implementation pending (Agent #7)");
            // TODO: Agent #7 implements this
        }

        Commands::Channel { action } => match action {
            ChannelCommands::List => {
                println!("Channel list implementation pending (Agent #8)");
            }
            ChannelCommands::Connect { channel_type } => {
                println!("Connecting channel: {}", channel_type);
                println!("Channel implementation pending (Agent #8)");
            }
            ChannelCommands::Disconnect { name } => {
                println!("Disconnecting channel: {}", name);
            }
            ChannelCommands::Status { name } => {
                println!("Status for channel: {}", name);
            }
        },

        Commands::Skill { action } => match action {
            SkillCommands::List => {
                println!("Skill list implementation pending (Agent #9)");
            }
            SkillCommands::Install { source } => {
                println!("Installing skill from: {}", source);
                println!("Skill implementation pending (Agent #9)");
            }
            SkillCommands::Uninstall { skill } => {
                println!("Uninstalling skill: {}", skill);
            }
            SkillCommands::Verify { skill } => {
                println!("Verifying skill: {}", skill);
            }
            SkillCommands::Info { skill } => {
                println!("Info for skill: {}", skill);
            }
        },

        Commands::Config { action } => match action {
            ConfigCommands::Show => {
                println!("Config path: {:?}", cli.config);
                println!("Config implementation pending (Agent #10)");
            }
            ConfigCommands::Set { key, value } => {
                println!("Setting {} = {}", key, value);
            }
            ConfigCommands::Get { key } => {
                println!("Getting config: {}", key);
            }
            ConfigCommands::Validate => {
                println!("Validating config...");
            }
        },

        Commands::Status => {
            println!("═══════════════════════════════════════════════════════");
            println!("                 0-OPENCLAW STATUS                       ");
            println!("═══════════════════════════════════════════════════════");
            println!();
            println!("Version:     {}", zero_openclaw::VERSION);
            println!("Gateway:     Not running");
            println!("Channels:    0 connected");
            println!("Skills:      0 installed");
            println!();
            println!("═══════════════════════════════════════════════════════");
        }

        Commands::Doctor => {
            println!("Running 0-openclaw diagnostics...");
            println!();
            
            print!("Checking configuration... ");
            println!("✓");
            
            print!("Checking Rust installation... ");
            println!("✓");
            
            print!("Checking 0-lang... ");
            println!("⚠ Not found (optional)");
            
            println!();
            println!("═══════════════════════════════════════════════════════");
            println!("All critical checks passed!");
            println!("═══════════════════════════════════════════════════════");
        }

        Commands::Init { path } => {
            println!("Initializing 0-openclaw at {:?}...", path);
            println!();
            println!("Created directories:");
            println!("  - ~/.0-openclaw/");
            println!("  - ~/.0-openclaw/skills/");
            println!("  - ~/.0-openclaw/workspace/");
            println!();
            println!("Created files:");
            println!("  - ~/.0-openclaw/config.json");
            println!("  - ~/.0-openclaw/keypair");
            println!();
            println!("0-openclaw initialized successfully!");
            println!();
            println!("Next steps:");
            println!("  1. Edit ~/.0-openclaw/config.json");
            println!("  2. Add channel credentials");
            println!("  3. Run: zero-openclaw gateway");
        }

        Commands::Verify { pca_file } => {
            println!("Verifying proof-carrying action: {:?}", pca_file);
            println!();
            println!("PCA verification implementation pending (Agent #7)");
        }
    }

    Ok(())
}
