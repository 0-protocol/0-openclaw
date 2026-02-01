//! Event bus for internal communication in 0-openclaw.
//!
//! The event bus provides a publish-subscribe mechanism for
//! loosely coupled communication between gateway components.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use serde::{Serialize, Deserialize};

use crate::types::{ContentHash, Confidence, ProofCarryingAction};

/// Event types that can be published on the event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GatewayEvent {
    /// A message was received from a channel
    MessageReceived {
        channel_id: String,
        sender_id: String,
        message_hash: ContentHash,
    },

    /// A message was processed successfully
    MessageProcessed {
        message_hash: ContentHash,
        skill_hash: ContentHash,
        confidence: Confidence,
    },

    /// An action was executed
    ActionExecuted {
        #[serde(skip)]
        action: Option<ProofCarryingAction>,
        action_type: String,
        success: bool,
    },

    /// A session was created
    SessionCreated {
        session_id: ContentHash,
        channel_id: String,
        user_id: String,
    },

    /// A session was updated
    SessionUpdated {
        session_id: ContentHash,
        trust_score: f32,
    },

    /// A session expired
    SessionExpired {
        session_id: ContentHash,
    },

    /// A skill was invoked
    SkillInvoked {
        skill_hash: ContentHash,
        skill_name: String,
    },

    /// An error occurred
    Error {
        source: String,
        message: String,
    },

    /// Gateway started
    GatewayStarted {
        timestamp: u64,
    },

    /// Gateway stopped
    GatewayStopped {
        timestamp: u64,
        reason: String,
    },

    /// Channel connected
    ChannelConnected {
        channel_id: String,
    },

    /// Channel disconnected
    ChannelDisconnected {
        channel_id: String,
        reason: String,
    },

    /// Custom event for extensions
    Custom {
        name: String,
        data: serde_json::Value,
    },
}

impl GatewayEvent {
    /// Get the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            GatewayEvent::MessageReceived { .. } => "message_received",
            GatewayEvent::MessageProcessed { .. } => "message_processed",
            GatewayEvent::ActionExecuted { .. } => "action_executed",
            GatewayEvent::SessionCreated { .. } => "session_created",
            GatewayEvent::SessionUpdated { .. } => "session_updated",
            GatewayEvent::SessionExpired { .. } => "session_expired",
            GatewayEvent::SkillInvoked { .. } => "skill_invoked",
            GatewayEvent::Error { .. } => "error",
            GatewayEvent::GatewayStarted { .. } => "gateway_started",
            GatewayEvent::GatewayStopped { .. } => "gateway_stopped",
            GatewayEvent::ChannelConnected { .. } => "channel_connected",
            GatewayEvent::ChannelDisconnected { .. } => "channel_disconnected",
            GatewayEvent::Custom { .. } => "custom",
        }
    }

    /// Create an error event.
    pub fn error(source: &str, message: &str) -> Self {
        GatewayEvent::Error {
            source: source.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a custom event.
    pub fn custom(name: &str, data: serde_json::Value) -> Self {
        GatewayEvent::Custom {
            name: name.to_string(),
            data,
        }
    }
}

/// Statistics about event bus usage.
#[derive(Debug, Clone, Default)]
pub struct EventBusStats {
    /// Total events published
    pub events_published: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Current subscribers
    pub subscriber_count: usize,
}

/// Event bus for gateway-wide communication.
pub struct EventBus {
    /// Broadcast sender for events
    sender: broadcast::Sender<GatewayEvent>,
    
    /// Statistics
    stats: Arc<RwLock<EventBusStats>>,
    
    /// Event history (optional, for debugging)
    history: Arc<RwLock<Vec<GatewayEvent>>>,
    
    /// Maximum history size
    max_history: usize,
    
    /// Whether to keep history
    keep_history: bool,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    /// Create an event bus with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        
        Self {
            sender,
            stats: Arc::new(RwLock::new(EventBusStats::default())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 1000,
            keep_history: false,
        }
    }

    /// Enable event history for debugging.
    pub fn with_history(mut self, max_size: usize) -> Self {
        self.keep_history = true;
        self.max_history = max_size;
        self
    }

    /// Publish an event to all subscribers.
    pub async fn publish(&self, event: GatewayEvent) {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.events_published += 1;
            *stats.events_by_type
                .entry(event.event_type().to_string())
                .or_insert(0) += 1;
        }

        // Add to history if enabled
        if self.keep_history {
            let mut history = self.history.write().await;
            if history.len() >= self.max_history {
                history.remove(0);
            }
            history.push(event.clone());
        }

        // Broadcast to subscribers (ignore errors if no subscribers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> EventSubscriber {
        let receiver = self.sender.subscribe();
        EventSubscriber { receiver }
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get event statistics.
    pub async fn stats(&self) -> EventBusStats {
        let mut stats = self.stats.read().await.clone();
        stats.subscriber_count = self.subscriber_count();
        stats
    }

    /// Get event history (if enabled).
    pub async fn history(&self) -> Vec<GatewayEvent> {
        self.history.read().await.clone()
    }

    /// Clear event history.
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    /// Publish a message received event.
    pub async fn message_received(&self, channel_id: &str, sender_id: &str, message_hash: ContentHash) {
        self.publish(GatewayEvent::MessageReceived {
            channel_id: channel_id.to_string(),
            sender_id: sender_id.to_string(),
            message_hash,
        }).await;
    }

    /// Publish an error event.
    pub async fn error(&self, source: &str, message: &str) {
        self.publish(GatewayEvent::error(source, message)).await;
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            stats: self.stats.clone(),
            history: self.history.clone(),
            max_history: self.max_history,
            keep_history: self.keep_history,
        }
    }
}

/// Event subscriber for receiving events.
pub struct EventSubscriber {
    receiver: broadcast::Receiver<GatewayEvent>,
}

impl EventSubscriber {
    /// Receive the next event.
    pub async fn recv(&mut self) -> Result<GatewayEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&mut self) -> Result<GatewayEvent, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Event filter for selective subscription.
pub struct EventFilter {
    /// Event types to include (empty = all)
    include_types: Vec<String>,
    
    /// Event types to exclude
    exclude_types: Vec<String>,
}

impl EventFilter {
    /// Create a new filter that accepts all events.
    pub fn all() -> Self {
        Self {
            include_types: Vec::new(),
            exclude_types: Vec::new(),
        }
    }

    /// Include only specific event types.
    pub fn include(mut self, event_type: &str) -> Self {
        self.include_types.push(event_type.to_string());
        self
    }

    /// Exclude specific event types.
    pub fn exclude(mut self, event_type: &str) -> Self {
        self.exclude_types.push(event_type.to_string());
        self
    }

    /// Check if an event matches the filter.
    pub fn matches(&self, event: &GatewayEvent) -> bool {
        let event_type = event.event_type();
        
        // Check exclusions first
        if self.exclude_types.contains(&event_type.to_string()) {
            return false;
        }
        
        // If include list is empty, accept all (except excluded)
        if self.include_types.is_empty() {
            return true;
        }
        
        // Check inclusions
        self.include_types.contains(&event_type.to_string())
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new();
        let mut subscriber = bus.subscribe();

        let event = GatewayEvent::GatewayStarted {
            timestamp: 12345,
        };
        
        bus.publish(event.clone()).await;
        
        let received = subscriber.recv().await.unwrap();
        assert_eq!(received.event_type(), "gateway_started");
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new();
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        let event = GatewayEvent::error("test", "test error");
        bus.publish(event).await;

        assert!(sub1.recv().await.is_ok());
        assert!(sub2.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_event_bus_stats() {
        let bus = EventBus::new();
        let _sub = bus.subscribe();

        bus.publish(GatewayEvent::error("test", "error1")).await;
        bus.publish(GatewayEvent::error("test", "error2")).await;
        
        let stats = bus.stats().await;
        assert_eq!(stats.events_published, 2);
        assert_eq!(stats.events_by_type.get("error"), Some(&2));
        assert_eq!(stats.subscriber_count, 1);
    }

    #[tokio::test]
    async fn test_event_bus_history() {
        let bus = EventBus::new().with_history(10);

        bus.publish(GatewayEvent::error("test", "error1")).await;
        bus.publish(GatewayEvent::error("test", "error2")).await;
        
        let history = bus.history().await;
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter::all()
            .include("error")
            .exclude("custom");

        let error_event = GatewayEvent::error("test", "msg");
        let custom_event = GatewayEvent::custom("test", serde_json::json!({}));
        let started_event = GatewayEvent::GatewayStarted { timestamp: 0 };

        assert!(filter.matches(&error_event));
        assert!(!filter.matches(&custom_event));
        assert!(!filter.matches(&started_event)); // Not in include list
    }

    #[test]
    fn test_event_type_names() {
        assert_eq!(GatewayEvent::error("", "").event_type(), "error");
        assert_eq!(GatewayEvent::GatewayStarted { timestamp: 0 }.event_type(), "gateway_started");
        assert_eq!(GatewayEvent::custom("", serde_json::json!({})).event_type(), "custom");
    }
}
