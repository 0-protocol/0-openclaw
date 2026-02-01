//! Session management for 0-openclaw.
//!
//! Sessions track the state of interactions between users and the assistant
//! across channels. Each session maintains history, trust scores, and context.

use std::collections::HashMap;
use crate::types::{ContentHash, Confidence, ProofCarryingAction};
use crate::error::SessionError;

/// Session manager responsible for creating and maintaining sessions.
pub struct SessionManager {
    /// Active sessions by session ID
    sessions: HashMap<ContentHash, Session>,
    
    /// Index: (channel_id, user_id) -> session_id
    user_sessions: HashMap<(String, String), ContentHash>,
    
    /// Session configuration
    config: SessionManagerConfig,
}

/// Configuration for the session manager.
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// Session timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum sessions per user
    pub max_per_user: usize,
    
    /// Initial trust score for new sessions
    pub initial_trust: f32,
    
    /// Trust score decay rate
    pub trust_decay: f32,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 3600,
            max_per_user: 10,
            initial_trust: 0.5,
            trust_decay: 0.01,
        }
    }
}

/// A user session.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub id: ContentHash,
    
    /// Channel this session belongs to
    pub channel_id: String,
    
    /// User identifier
    pub user_id: String,
    
    /// Current session state
    pub state: SessionState,
    
    /// History of action hashes
    pub history: Vec<ContentHash>,
    
    /// Accumulated trust score
    pub trust_score: Confidence,
    
    /// Creation timestamp (Unix milliseconds)
    pub created_at: u64,
    
    /// Last activity timestamp (Unix milliseconds)
    pub last_activity: u64,
}

/// Session state data.
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    /// Serialized state data
    pub data: Vec<u8>,
    
    /// State version for conflict detection
    pub version: u64,
    
    /// Hash of current state
    pub hash: ContentHash,
    
    /// Custom context variables
    pub context: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session.
    pub fn new(channel_id: &str, user_id: &str, initial_trust: f32) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let id = Self::generate_id(channel_id, user_id, now);
        
        Self {
            id,
            channel_id: channel_id.to_string(),
            user_id: user_id.to_string(),
            state: SessionState::default(),
            history: Vec::new(),
            trust_score: Confidence::new(initial_trust),
            created_at: now,
            last_activity: now,
        }
    }

    /// Generate a unique session ID.
    fn generate_id(channel_id: &str, user_id: &str, timestamp: u64) -> ContentHash {
        ContentHash::from_bytes(
            format!("session:{}:{}:{}", channel_id, user_id, timestamp).as_bytes()
        )
    }

    /// Get the session hash (alias for id).
    pub fn hash(&self) -> ContentHash {
        self.id
    }

    /// Check if the session has expired.
    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let elapsed_seconds = (now - self.last_activity) / 1000;
        elapsed_seconds > timeout_seconds
    }

    /// Update the last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = chrono::Utc::now().timestamp_millis() as u64;
    }

    /// Add an action to the history.
    pub fn add_to_history(&mut self, action_hash: ContentHash) {
        self.history.push(action_hash);
        self.touch();
    }

    /// Get the number of actions in history.
    pub fn history_length(&self) -> usize {
        self.history.len()
    }

    /// Set a context variable.
    pub fn set_context(&mut self, key: &str, value: serde_json::Value) {
        self.state.context.insert(key.to_string(), value);
        self.state.version += 1;
    }

    /// Get a context variable.
    pub fn get_context(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.context.get(key)
    }
}


impl SessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        Self::with_config(SessionManagerConfig::default())
    }

    /// Create a session manager with custom configuration.
    pub fn with_config(config: SessionManagerConfig) -> Self {
        Self {
            sessions: HashMap::new(),
            user_sessions: HashMap::new(),
            config,
        }
    }

    /// Get or create a session for a user.
    pub fn get_or_create(
        &mut self,
        channel_id: &str,
        user_id: &str,
    ) -> Result<&Session, SessionError> {
        let key = (channel_id.to_string(), user_id.to_string());
        
        // Check if session exists and is not expired
        if let Some(&session_id) = self.user_sessions.get(&key) {
            if let Some(session) = self.sessions.get(&session_id) {
                if !session.is_expired(self.config.timeout_seconds) {
                    return Ok(self.sessions.get(&session_id).unwrap());
                }
                // Session expired, remove it
                self.sessions.remove(&session_id);
            }
            self.user_sessions.remove(&key);
        }

        // Create new session
        let session = Session::new(channel_id, user_id, self.config.initial_trust);
        let session_id = session.id;
        
        self.sessions.insert(session_id, session);
        self.user_sessions.insert(key, session_id);

        Ok(self.sessions.get(&session_id).unwrap())
    }

    /// Get a session by ID.
    pub fn get(&self, session_id: &ContentHash) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    /// Get a mutable session by ID.
    pub fn get_mut(&mut self, session_id: &ContentHash) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    /// Update a session after an action.
    pub fn update(
        &mut self,
        session_id: &ContentHash,
        action: &ProofCarryingAction,
    ) -> Result<(), SessionError> {
        let session = self.sessions.get_mut(session_id)
            .ok_or(SessionError::NotFound)?;

        // Update history
        session.add_to_history(action.input_hash);

        // Update trust score using exponential moving average
        session.trust_score = Self::update_trust(
            session.trust_score,
            action.confidence,
        );

        // Update state version
        session.state.version += 1;
        session.state.hash = ContentHash::from_bytes(
            &session.state.version.to_le_bytes()
        );

        Ok(())
    }

    /// Calculate new trust score using exponential moving average.
    fn update_trust(current: Confidence, action_confidence: Confidence) -> Confidence {
        let alpha = 0.1;
        let new_value = (1.0 - alpha) * current.value() + alpha * action_confidence.value();
        Confidence::new(new_value)
    }

    /// Remove a session.
    pub fn remove(&mut self, session_id: &ContentHash) -> Option<Session> {
        if let Some(session) = self.sessions.remove(session_id) {
            let key = (session.channel_id.clone(), session.user_id.clone());
            self.user_sessions.remove(&key);
            Some(session)
        } else {
            None
        }
    }

    /// Clean up expired sessions.
    pub fn cleanup_expired(&mut self) -> usize {
        let expired: Vec<ContentHash> = self.sessions
            .iter()
            .filter(|(_, s)| s.is_expired(self.config.timeout_seconds))
            .map(|(id, _)| *id)
            .collect();

        let count = expired.len();
        for id in expired {
            self.remove(&id);
        }
        count
    }

    /// Get all active sessions.
    pub fn list(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    /// Get the number of active sessions.
    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    /// Get sessions for a specific channel.
    pub fn sessions_for_channel(&self, channel_id: &str) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|s| s.channel_id == channel_id)
            .collect()
    }

    /// Get sessions for a specific user.
    pub fn sessions_for_user(&self, user_id: &str) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .collect()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Session information for API responses.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    /// Session ID (hex)
    pub id: String,
    /// Channel ID
    pub channel_id: String,
    /// User ID
    pub user_id: String,
    /// Trust score
    pub trust_score: f32,
    /// Number of actions in history
    pub history_length: usize,
    /// Creation timestamp
    pub created_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.to_hex(),
            channel_id: session.channel_id.clone(),
            user_id: session.user_id.clone(),
            trust_score: session.trust_score.value(),
            history_length: session.history.len(),
            created_at: session.created_at,
            last_activity: session.last_activity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("telegram", "user123", 0.5);
        assert_eq!(session.channel_id, "telegram");
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.trust_score.value(), 0.5);
        assert!(session.history.is_empty());
    }

    #[test]
    fn test_session_manager_get_or_create() {
        let mut manager = SessionManager::new();
        
        let session1 = manager.get_or_create("telegram", "user1").unwrap();
        let id1 = session1.id;
        
        let session2 = manager.get_or_create("telegram", "user1").unwrap();
        assert_eq!(id1, session2.id); // Same session
        
        let session3 = manager.get_or_create("discord", "user1").unwrap();
        assert_ne!(id1, session3.id); // Different channel, different session
    }

    #[test]
    fn test_session_update() {
        let mut manager = SessionManager::new();
        let session = manager.get_or_create("test", "user").unwrap();
        let session_id = session.id;

        let pca = ProofCarryingAction::pending();
        manager.update(&session_id, &pca).unwrap();

        let session = manager.get(&session_id).unwrap();
        assert_eq!(session.history.len(), 1);
    }

    #[test]
    fn test_trust_update() {
        let current = Confidence::new(0.5);
        let action = Confidence::new(0.9);
        let updated = SessionManager::update_trust(current, action);
        
        // Should move toward action confidence
        assert!(updated.value() > 0.5);
        assert!(updated.value() < 0.9);
    }

    #[test]
    fn test_session_context() {
        let mut session = Session::new("test", "user", 0.5);
        
        session.set_context("key", serde_json::json!("value"));
        assert_eq!(
            session.get_context("key"),
            Some(&serde_json::json!("value"))
        );
    }
}
