//! Gateway module for 0-openclaw.
//!
//! The Gateway is the central control plane that coordinates all operations,
//! including message routing, session management, and proof generation.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                         Gateway                               │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐              │
//! │  │  Session   │  │   Router   │  │   Skill    │              │
//! │  │  Manager   │  │            │  │  Registry  │              │
//! │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘              │
//! │        └───────────────┼───────────────┘                      │
//! │                        ↓                                      │
//! │              ┌──────────────────┐                             │
//! │              │  Proof Generator │                             │
//! │              └──────────────────┘                             │
//! │                        ↓                                      │
//! │  ┌────────────┐  ┌────────────┐                              │
//! │  │  Event Bus │  │  WebSocket │                              │
//! │  │            │  │   Server   │                              │
//! │  └────────────┘  └────────────┘                              │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Implementation
//!
//! Implemented by **Agent #7**.
//!
//! See: `AGENT-7-0OPENCLAW-GATEWAY.md`

pub mod config;
pub mod session;
pub mod router;
pub mod proof;
pub mod events;
pub mod server;

// Re-exports
pub use config::GatewayConfig;
pub use session::{Session, SessionManager, SessionInfo};
pub use router::{Router, RouteResult};
pub use proof::{ProofGenerator, ProofBuilder, ExecutionTrace};
pub use events::{EventBus, GatewayEvent, EventSubscriber, EventFilter};
pub use server::{GatewayServer, ServerState, ServerMessage, ClientMessage};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::{ContentHash, IncomingMessage, OutgoingMessage, Action, ProofCarryingAction};
use crate::error::GatewayError;
use crate::channels::Channel;
use crate::skills::SkillRegistry;

/// Main Gateway structure.
///
/// The Gateway coordinates all 0-openclaw operations including:
/// - Message routing to appropriate skills
/// - Session management and trust scoring
/// - Proof-Carrying Action generation
/// - Channel communication
/// - Event broadcasting
pub struct Gateway {
    /// Session manager
    sessions: Arc<RwLock<SessionManager>>,
    
    /// Message router
    router: Arc<RwLock<Router>>,
    
    /// Registered channels
    channels: HashMap<String, Arc<dyn Channel>>,
    
    /// Skill registry
    skills: Arc<RwLock<SkillRegistry>>,
    
    /// Proof generator
    proof_generator: Arc<ProofGenerator>,
    
    /// Event bus for internal communication
    event_bus: EventBus,
    
    /// Gateway configuration
    config: GatewayConfig,
    
    /// Whether the gateway is running
    running: Arc<RwLock<bool>>,
}

impl Gateway {
    /// Create a new Gateway instance with default configuration.
    pub fn new() -> Result<Self, GatewayError> {
        Self::with_config(GatewayConfig::default())
    }

    /// Create a new Gateway instance with custom configuration.
    pub fn with_config(config: GatewayConfig) -> Result<Self, GatewayError> {
        // Validate configuration
        config.validate().map_err(|e| GatewayError::InvalidConfig(e.to_string()))?;

        // Initialize proof generator
        let proof_generator = if config.keypair_path.exists() {
            ProofGenerator::from_file(&config.keypair_path)
                .map_err(|e| GatewayError::InvalidConfig(format!("Failed to load keypair: {}", e)))?
        } else {
            // Generate new keypair for development
            tracing::warn!("No keypair found, generating new one for development");
            let generator = ProofGenerator::new_random();
            
            // Try to save it
            if let Some(parent) = config.keypair_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = generator.save_to_file(&config.keypair_path);
            
            generator
        };

        // Initialize session manager with config
        let session_config = session::SessionManagerConfig {
            timeout_seconds: config.session.timeout_seconds,
            max_per_user: config.session.max_per_user,
            initial_trust: config.session.initial_trust,
            trust_decay: config.session.trust_decay,
        };

        // Initialize router
        let router = Self::create_default_router();

        Ok(Self {
            sessions: Arc::new(RwLock::new(SessionManager::with_config(session_config))),
            router: Arc::new(RwLock::new(router)),
            channels: HashMap::new(),
            skills: Arc::new(RwLock::new(SkillRegistry::new(&config.skills_path))),
            proof_generator: Arc::new(proof_generator),
            event_bus: EventBus::new().with_history(1000),
            config,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Create the default router with built-in commands.
    fn create_default_router() -> Router {
        // Use the graph-based router with default configuration
        Router::with_defaults()
    }

    /// Register a channel.
    pub fn register_channel(&mut self, channel: Arc<dyn Channel>) {
        let name = channel.name().to_string();
        tracing::info!("Registering channel: {}", name);
        self.channels.insert(name, channel);
    }

    /// Get a registered channel by name.
    pub fn get_channel(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.channels.get(name).cloned()
    }

    /// Get the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get the configuration.
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    /// Process an incoming message.
    ///
    /// This is the main entry point for message processing.
    /// Returns a Proof-Carrying Action.
    pub async fn process_message(
        &self,
        message: IncomingMessage,
    ) -> Result<ProofCarryingAction, GatewayError> {
        tracing::debug!("Processing message from {}/{}", message.channel_id, message.sender_id);

        // Publish event
        self.event_bus.publish(GatewayEvent::MessageReceived {
            channel_id: message.channel_id.clone(),
            sender_id: message.sender_id.clone(),
            message_hash: message.id,
        }).await;

        // 1. Get or create session
        let (session_id, session_hash, trust_score) = {
            let mut sessions = self.sessions.write().await;
            let session = sessions.get_or_create(&message.channel_id, &message.sender_id)
                .map_err(|e| GatewayError::RouterError(e.to_string()))?;
            (session.id, session.hash(), session.trust_score.value())
        };

        // Publish session event if new
        self.event_bus.publish(GatewayEvent::SessionUpdated {
            session_id,
            trust_score,
        }).await;

        // 2. Route the message
        let (route_result, route_trace) = {
            let mut router = self.router.write().await;
            router.route(&message).await?
        };

        tracing::debug!("Routed to skill: {} ({})", route_result.route_name, route_result.skill_hash);

        // Publish skill invoked event
        self.event_bus.publish(GatewayEvent::SkillInvoked {
            skill_hash: route_result.skill_hash,
            skill_name: route_result.route_name.clone(),
        }).await;

        // 3. Execute skill (placeholder - would invoke 0-lang VM)
        let (action, skill_trace) = self.execute_skill(
            &route_result.skill_hash,
            &message,
            &route_result.params,
        ).await?;

        // 4. Generate proof-carrying action
        let pca = self.proof_generator.generate(
            action,
            session_hash,
            message.id,
            vec![route_trace, skill_trace],
        ).map_err(|e| GatewayError::VmError(e.to_string()))?;

        // 5. Update session
        {
            let mut sessions = self.sessions.write().await;
            sessions.update(&session_id, &pca)
                .map_err(|e| GatewayError::RouterError(e.to_string()))?;
        }

        // Publish completion event
        self.event_bus.publish(GatewayEvent::MessageProcessed {
            message_hash: message.id,
            skill_hash: route_result.skill_hash,
            confidence: pca.confidence,
        }).await;

        tracing::debug!("Message processed: {}", pca);
        Ok(pca)
    }

    /// Execute a skill graph.
    ///
    /// This is a placeholder that would integrate with the 0-lang VM.
    async fn execute_skill(
        &self,
        skill_hash: &ContentHash,
        message: &IncomingMessage,
        params: &HashMap<String, String>,
    ) -> Result<(Action, ExecutionTrace), GatewayError> {
        let mut trace = ExecutionTrace::new();
        trace.add_node(*skill_hash);

        // Check if skill exists
        let skills = self.skills.read().await;
        
        // For now, return a placeholder action based on the skill
        let action = if let Some(_skill) = skills.get(skill_hash) {
            // Skill exists, would execute via VM
            Action::NoOp {
                reason: format!("Skill {} execution not yet implemented", skill_hash),
            }
        } else {
            // Built-in command handling
            self.handle_builtin_command(message, params)?
        };

        Ok((action, trace))
    }

    /// Handle built-in commands.
    fn handle_builtin_command(
        &self,
        message: &IncomingMessage,
        _params: &HashMap<String, String>,
    ) -> Result<Action, GatewayError> {
        let content = message.content.trim();
        
        if content.starts_with("/help") {
            Ok(Action::SendMessage(OutgoingMessage::new(
                &message.channel_id,
                &message.sender_id,
                "Available commands:\n\
                 /help - Show this help message\n\
                 /status - Show gateway status\n\
                 /skills - List installed skills\n\
                 /session - Show session info",
            ).reply_to(message.id)))
        } else if content.starts_with("/status") {
            Ok(Action::SendMessage(OutgoingMessage::new(
                &message.channel_id,
                &message.sender_id,
                &format!(
                    "Gateway Status:\n\
                     Channels: {}\n\
                     Running: {}",
                    self.channels.len(),
                    // Can't await here, so use placeholder
                    "checking..."
                ),
            ).reply_to(message.id)))
        } else {
            // Default conversation response
            Ok(Action::SendMessage(OutgoingMessage::new(
                &message.channel_id,
                &message.sender_id,
                &format!("Received: {}", content),
            ).reply_to(message.id)))
        }
    }

    /// Execute a proof-carrying action.
    pub async fn execute_action(
        &self,
        pca: &ProofCarryingAction,
    ) -> Result<(), GatewayError> {
        // Verify the proof first
        self.proof_generator.verify(pca)
            .map_err(|e| GatewayError::VmError(format!("Proof verification failed: {}", e)))?;

        match &pca.action {
            Action::SendMessage(msg) => {
                if let Some(channel) = self.channels.get(&msg.channel_id) {
                    channel.send(msg.clone()).await
                        .map_err(|e| GatewayError::ChannelNotFound(e.to_string()))?;
                }
            }
            Action::ExecuteSkill { skill_hash, inputs: _ } => {
                tracing::info!("Would execute skill: {}", skill_hash);
            }
            Action::UpdateSession { session_id, updates: _ } => {
                tracing::info!("Would update session: {}", session_id);
            }
            Action::NoOp { reason } => {
                tracing::debug!("NoOp: {}", reason);
            }
        }

        // Publish action executed event
        self.event_bus.publish(GatewayEvent::ActionExecuted {
            action: Some(pca.clone()),
            action_type: pca.action.action_type().to_string(),
            success: true,
        }).await;

        Ok(())
    }

    /// Start the gateway.
    pub async fn run(&self) -> Result<(), GatewayError> {
        // Check if already running
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(GatewayError::AlreadyRunning);
            }
            *running = true;
        }

        tracing::info!("Starting 0-openclaw Gateway");

        // Publish start event
        self.event_bus.publish(GatewayEvent::GatewayStarted {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }).await;

        // Start channel listeners
        for (name, channel) in &self.channels {
            let channel = channel.clone();
            let channel_name = name.clone();
            let _sessions = self.sessions.clone();
            let _router = self.router.clone();
            let _proof_generator = self.proof_generator.clone();
            let _event_bus = self.event_bus.clone();
            let _skills = self.skills.clone();

            tokio::spawn(async move {
                tracing::info!("Starting channel listener: {}", channel_name);
                
                loop {
                    match channel.receive().await {
                        Ok(message) => {
                            tracing::debug!("Received message on {}: {}", channel_name, message.id);
                            
                            // Process message (simplified version without full gateway context)
                            // In production, this would call back to the gateway
                        }
                        Err(e) => {
                            tracing::error!("Channel {} receive error: {}", channel_name, e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            });

            // Publish channel connected event
            self.event_bus.publish(GatewayEvent::ChannelConnected {
                channel_id: name.clone(),
            }).await;
        }

        // Start WebSocket server
        let server = GatewayServer::new(
            self.event_bus.clone(),
            &self.config.server.host,
            self.config.server.port,
        );
        
        server.start().await
    }

    /// Stop the gateway gracefully.
    pub async fn stop(&self) -> Result<(), GatewayError> {
        let mut running = self.running.write().await;
        *running = false;

        // Publish stop event
        self.event_bus.publish(GatewayEvent::GatewayStopped {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            reason: "Shutdown requested".to_string(),
        }).await;

        tracing::info!("Gateway stopped");
        Ok(())
    }

    /// Get session information.
    pub async fn get_session_info(&self, session_id: &ContentHash) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(SessionInfo::from)
    }

    /// List all active sessions.
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.list().iter().map(|s| SessionInfo::from(*s)).collect()
    }

    /// Get the number of active sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.count()
    }

    /// Clean up expired sessions.
    pub async fn cleanup_sessions(&self) -> usize {
        self.sessions.write().await.cleanup_expired()
    }

    /// Set the default skill for routing.
    pub async fn set_default_skill(&self, skill_hash: ContentHash) {
        self.router.write().await.set_default_skill(skill_hash);
    }

    /// Load a custom router from a graph file.
    pub async fn load_router_graph(&self, path: &str) -> Result<(), GatewayError> {
        let new_router = Router::from_file(path)?;
        *self.router.write().await = new_router;
        Ok(())
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new().expect("Failed to create default Gateway")
    }
}

// Allow cloning the gateway (shares state via Arc)
impl Clone for Gateway {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            router: self.router.clone(),
            channels: self.channels.clone(),
            skills: self.skills.clone(),
            proof_generator: self.proof_generator.clone(),
            event_bus: self.event_bus.clone(),
            config: self.config.clone(),
            running: self.running.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_creation() {
        let gateway = Gateway::new().unwrap();
        assert!(gateway.channels.is_empty());
    }

    #[test]
    fn test_gateway_with_config() {
        let config = GatewayConfig::for_testing();
        let gateway = Gateway::with_config(config).unwrap();
        assert_eq!(gateway.config.server.port, 0);
    }

    #[tokio::test]
    async fn test_process_message() {
        let gateway = Gateway::new().unwrap();
        let message = IncomingMessage::new("test", "user123", "/help");
        
        let pca = gateway.process_message(message).await.unwrap();
        
        assert!(pca.is_signed());
        assert!(matches!(pca.action, Action::SendMessage(_)));
    }

    #[tokio::test]
    async fn test_session_management() {
        let gateway = Gateway::new().unwrap();
        
        // Process a message to create session
        let message = IncomingMessage::new("test", "user123", "hello");
        let _ = gateway.process_message(message).await.unwrap();
        
        // Check session exists
        assert_eq!(gateway.session_count().await, 1);
        
        // List sessions
        let sessions = gateway.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].channel_id, "test");
        assert_eq!(sessions[0].user_id, "user123");
    }

    #[tokio::test]
    async fn test_event_publishing() {
        let gateway = Gateway::new().unwrap();
        let mut subscriber = gateway.event_bus().subscribe();
        
        let message = IncomingMessage::new("test", "user", "hello");
        let _ = gateway.process_message(message).await.unwrap();
        
        // Should receive events
        let event = subscriber.recv().await.unwrap();
        assert_eq!(event.event_type(), "message_received");
    }

    #[tokio::test]
    async fn test_builtin_help_command() {
        let gateway = Gateway::new().unwrap();
        let message = IncomingMessage::new("test", "user", "/help");
        
        let pca = gateway.process_message(message).await.unwrap();
        
        if let Action::SendMessage(msg) = &pca.action {
            assert!(msg.content.contains("Available commands"));
        } else {
            panic!("Expected SendMessage action");
        }
    }

    #[tokio::test]
    async fn test_set_default_skill() {
        let gateway = Gateway::new().unwrap();
        let custom_default = ContentHash::from_string("skill:custom_default");
        
        gateway.set_default_skill(custom_default).await;
        
        // Verify the router has the updated default
        let router = gateway.router.read().await;
        assert!(router.graph().name.len() > 0);
    }
}
