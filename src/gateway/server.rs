//! WebSocket server for 0-openclaw Gateway.
//!
//! Provides a WebSocket API for external clients to interact with the gateway,
//! including real-time event streaming and action submission.

use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade, Message},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
    Json,
};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{CorsLayer, Any};
use serde::{Deserialize, Serialize};

use crate::error::GatewayError;
use super::session::SessionInfo;
use super::events::{EventBus, GatewayEvent};

/// Server message sent to WebSocket clients.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// An action was executed
    ActionExecuted {
        action_type: String,
        success: bool,
        message_hash: String,
    },

    /// A session was updated
    SessionUpdated {
        session_id: String,
        trust_score: f32,
    },

    /// An event occurred
    Event {
        event_type: String,
        data: serde_json::Value,
    },

    /// Error message
    Error {
        code: String,
        message: String,
    },

    /// Welcome message on connection
    Welcome {
        server_version: String,
        session_count: usize,
    },

    /// Pong response
    Pong {
        timestamp: u64,
    },
}

/// Client message received from WebSocket clients.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Send a message to be processed
    SendMessage {
        channel_id: String,
        content: String,
    },

    /// Subscribe to specific event types
    Subscribe {
        event_types: Vec<String>,
    },

    /// Unsubscribe from event types
    Unsubscribe {
        event_types: Vec<String>,
    },

    /// Ping for keepalive
    Ping {
        timestamp: u64,
    },

    /// Request session info
    GetSession {
        session_id: String,
    },
}

/// Shared server state.
pub struct ServerState {
    /// Event bus for broadcasting
    pub event_bus: EventBus,

    /// Broadcast sender for server messages
    broadcast_tx: broadcast::Sender<ServerMessage>,

    /// Session count (updated periodically)
    session_count: Arc<RwLock<usize>>,

    /// Server version
    version: String,
}

impl ServerState {
    /// Create new server state.
    pub fn new(event_bus: EventBus) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        
        Self {
            event_bus,
            broadcast_tx,
            session_count: Arc::new(RwLock::new(0)),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Update the session count.
    pub async fn update_session_count(&self, count: usize) {
        *self.session_count.write().await = count;
    }

    /// Broadcast a message to all connected clients.
    pub fn broadcast(&self, message: ServerMessage) {
        let _ = self.broadcast_tx.send(message);
    }

    /// Subscribe to broadcast messages.
    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }
}

/// Gateway WebSocket server.
pub struct GatewayServer {
    /// Server state
    state: Arc<ServerState>,
    
    /// Server configuration
    host: String,
    port: u16,
}

impl GatewayServer {
    /// Create a new gateway server.
    pub fn new(event_bus: EventBus, host: &str, port: u16) -> Self {
        Self {
            state: Arc::new(ServerState::new(event_bus)),
            host: host.to_string(),
            port,
        }
    }

    /// Get a reference to the server state.
    pub fn state(&self) -> Arc<ServerState> {
        self.state.clone()
    }

    /// Start the server.
    pub async fn start(&self) -> Result<(), GatewayError> {
        let state = self.state.clone();

        // Build router
        let app = Router::new()
            .route("/ws", get(Self::ws_handler))
            .route("/health", get(Self::health_handler))
            .route("/sessions", get(Self::sessions_handler))
            .route("/stats", get(Self::stats_handler))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .with_state(state);

        let addr: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| GatewayError::ServerError(format!("Invalid address: {}", e)))?;

        tracing::info!("Gateway server listening on {}", addr);

        // Publish gateway started event
        self.state.event_bus.publish(GatewayEvent::GatewayStarted {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }).await;

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| GatewayError::ServerError(e.to_string()))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| GatewayError::ServerError(e.to_string()))
    }

    /// WebSocket handler.
    async fn ws_handler(
        ws: WebSocketUpgrade,
        State(state): State<Arc<ServerState>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(move |socket| Self::handle_socket(socket, state))
    }

    /// Handle a WebSocket connection.
    async fn handle_socket(socket: WebSocket, state: Arc<ServerState>) {
        let (mut sender, mut receiver) = socket.split();
        use futures::SinkExt;
        use futures::StreamExt;

        // Send welcome message
        let session_count = *state.session_count.read().await;
        let welcome = ServerMessage::Welcome {
            server_version: state.version.clone(),
            session_count,
        };
        
        if let Ok(json) = serde_json::to_string(&welcome) {
            let _ = sender.send(Message::Text(json.into())).await;
        }

        // Subscribe to broadcasts
        let mut broadcast_rx = state.subscribe();

        loop {
            tokio::select! {
                // Handle incoming messages
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                                let response = Self::handle_client_message(client_msg, &state).await;
                                if let Ok(json) = serde_json::to_string(&response) {
                                    if sender.send(Message::Text(json.into())).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => break,
                        Some(Err(_)) => break,
                        None => break,
                        _ => {}
                    }
                }
                // Broadcast server messages
                Ok(server_msg) = broadcast_rx.recv() => {
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Handle a client message.
    async fn handle_client_message(
        msg: ClientMessage,
        _state: &ServerState,
    ) -> ServerMessage {
        match msg {
            ClientMessage::Ping { timestamp } => {
                ServerMessage::Pong { timestamp }
            }
            ClientMessage::SendMessage { channel_id, content } => {
                // TODO: Forward to gateway for processing
                ServerMessage::Error {
                    code: "NOT_IMPLEMENTED".to_string(),
                    message: format!("Message processing not yet implemented: {} - {}", channel_id, content),
                }
            }
            ClientMessage::Subscribe { event_types } => {
                // TODO: Implement per-connection subscriptions
                ServerMessage::Event {
                    event_type: "subscribed".to_string(),
                    data: serde_json::json!({ "types": event_types }),
                }
            }
            ClientMessage::Unsubscribe { event_types } => {
                ServerMessage::Event {
                    event_type: "unsubscribed".to_string(),
                    data: serde_json::json!({ "types": event_types }),
                }
            }
            ClientMessage::GetSession { session_id } => {
                // TODO: Look up session
                ServerMessage::Error {
                    code: "NOT_FOUND".to_string(),
                    message: format!("Session not found: {}", session_id),
                }
            }
        }
    }

    /// Health check handler.
    async fn health_handler() -> &'static str {
        "ok"
    }

    /// Sessions list handler.
    async fn sessions_handler(
        State(_state): State<Arc<ServerState>>,
    ) -> Json<Vec<SessionInfo>> {
        // TODO: Get sessions from gateway
        Json(Vec::new())
    }

    /// Stats handler.
    async fn stats_handler(
        State(state): State<Arc<ServerState>>,
    ) -> Json<StatsResponse> {
        let event_stats = state.event_bus.stats().await;
        
        Json(StatsResponse {
            version: state.version.clone(),
            session_count: *state.session_count.read().await,
            events_published: event_stats.events_published,
            subscriber_count: event_stats.subscriber_count,
        })
    }
}

/// Stats response.
#[derive(Serialize)]
pub struct StatsResponse {
    /// Server version
    pub version: String,
    /// Number of active sessions
    pub session_count: usize,
    /// Total events published
    pub events_published: u64,
    /// Number of event subscribers
    pub subscriber_count: usize,
}

/// Server handle for controlling the running server.
pub struct ServerHandle {
    /// Shutdown signal sender
    _shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl ServerHandle {
    /// Signal the server to shut down.
    pub fn shutdown(self) {
        // Dropping the sender signals shutdown
        drop(self._shutdown_tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Welcome {
            server_version: "0.1.0".to_string(),
            session_count: 5,
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Welcome"));
        assert!(json.contains("0.1.0"));
    }

    #[test]
    fn test_client_message_deserialization() {
        let json = r#"{"type":"Ping","timestamp":12345}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        
        match msg {
            ClientMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_server_state_creation() {
        let event_bus = EventBus::new();
        let state = ServerState::new(event_bus);
        
        assert_eq!(state.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_server_state_session_count() {
        let event_bus = EventBus::new();
        let state = ServerState::new(event_bus);
        
        state.update_session_count(10).await;
        assert_eq!(*state.session_count.read().await, 10);
    }
}
