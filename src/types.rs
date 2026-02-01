//! Core types for 0-openclaw.
//!
//! This module defines the fundamental types used throughout the system,
//! including content hashes, confidence scores, messages, and proof-carrying actions.

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fmt;

/// Unique identifier based on content hash (SHA-256).
///
/// ContentHash provides content-addressed identification: the same content
/// always produces the same hash, enabling deterministic behavior verification.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Create a ContentHash from arbitrary bytes.
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Self(hasher.finalize().into())
    }

    /// Create a ContentHash from a string.
    pub fn from_string(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Create a zero hash (for testing/defaults).
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Get the hash as a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from hex string.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", &self.to_hex()[..16])
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.to_hex()[..16])
    }
}

/// Confidence score (0.0 to 1.0).
///
/// Confidence scores replace boolean permissions with probabilistic trust.
/// A confidence of 0.0 means completely untrusted, 1.0 means fully trusted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Confidence(f32);

impl Confidence {
    /// Create a new confidence score, clamped to [0.0, 1.0].
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the confidence value.
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Check if confidence meets a threshold.
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.0 >= threshold
    }

    /// Combine multiple confidence scores using geometric mean.
    pub fn combine(scores: &[Confidence]) -> Confidence {
        if scores.is_empty() {
            return Confidence::new(0.5);
        }
        let product: f32 = scores.iter().map(|c| c.0).product();
        let combined = product.powf(1.0 / scores.len() as f32);
        Confidence::new(combined)
    }

    /// Full confidence (1.0).
    pub fn full() -> Self {
        Self(1.0)
    }

    /// No confidence (0.0).
    pub fn none() -> Self {
        Self(0.0)
    }

    /// Neutral confidence (0.5).
    pub fn neutral() -> Self {
        Self(0.5)
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self::neutral()
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

/// Incoming message from any channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// Content hash of the message (unique identifier).
    pub id: ContentHash,
    
    /// Channel this message came from (e.g., "telegram", "discord").
    pub channel_id: String,
    
    /// Sender's identifier within the channel.
    pub sender_id: String,
    
    /// Message content.
    pub content: String,
    
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    
    /// Channel-specific metadata.
    pub metadata: serde_json::Value,
}

impl IncomingMessage {
    /// Create a new incoming message with auto-generated ID.
    pub fn new(channel_id: &str, sender_id: &str, content: &str) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let id_data = format!("{}:{}:{}:{}", channel_id, sender_id, content, timestamp);
        
        Self {
            id: ContentHash::from_string(&id_data),
            channel_id: channel_id.to_string(),
            sender_id: sender_id.to_string(),
            content: content.to_string(),
            timestamp,
            metadata: serde_json::Value::Null,
        }
    }

    /// Add metadata to the message.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Outgoing message to any channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    /// Target channel.
    pub channel_id: String,
    
    /// Recipient's identifier within the channel.
    pub recipient_id: String,
    
    /// Message content.
    pub content: String,
    
    /// Optional: message this is replying to.
    pub reply_to: Option<ContentHash>,
}

impl OutgoingMessage {
    /// Create a new outgoing message.
    pub fn new(channel_id: &str, recipient_id: &str, content: &str) -> Self {
        Self {
            channel_id: channel_id.to_string(),
            recipient_id: recipient_id.to_string(),
            content: content.to_string(),
            reply_to: None,
        }
    }

    /// Set the message this is replying to.
    pub fn reply_to(mut self, hash: ContentHash) -> Self {
        self.reply_to = Some(hash);
        self
    }
}

/// Actions the assistant can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Send a message to a channel.
    SendMessage(OutgoingMessage),
    
    /// Execute a skill.
    ExecuteSkill {
        skill_hash: ContentHash,
        inputs: serde_json::Value,
    },
    
    /// Update session state.
    UpdateSession {
        session_id: ContentHash,
        updates: serde_json::Value,
    },
    
    /// No operation (with reason).
    NoOp {
        reason: String,
    },
}

impl Action {
    /// Check if this is a no-op action.
    pub fn is_noop(&self) -> bool {
        matches!(self, Action::NoOp { .. })
    }

    /// Get a description of the action type.
    pub fn action_type(&self) -> &'static str {
        match self {
            Action::SendMessage(_) => "SendMessage",
            Action::ExecuteSkill { .. } => "ExecuteSkill",
            Action::UpdateSession { .. } => "UpdateSession",
            Action::NoOp { .. } => "NoOp",
        }
    }
}

/// Proof-Carrying Action - the core innovation of 0-openclaw.
///
/// Every action includes cryptographic proof of the decision path,
/// allowing verification without trusting the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCarryingAction {
    /// The action to perform.
    pub action: Action,
    
    /// Hash of the session context.
    pub session_hash: ContentHash,
    
    /// Hash of the input that triggered this action.
    pub input_hash: ContentHash,
    
    /// Hashes of all graph nodes evaluated (execution trace).
    pub execution_trace: Vec<ContentHash>,
    
    /// Confidence score for this action.
    pub confidence: Confidence,
    
    /// Ed25519 signature over all fields (hex-encoded for serde compatibility).
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
    
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
}

/// Custom serde module for [u8; 64] signature.
mod signature_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        hex::encode(bytes).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("signature must be 64 bytes"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

impl ProofCarryingAction {
    /// Create a pending PCA (used before signing).
    pub fn pending() -> Self {
        Self {
            action: Action::NoOp { reason: "pending".to_string() },
            session_hash: ContentHash::zero(),
            input_hash: ContentHash::zero(),
            execution_trace: Vec::new(),
            confidence: Confidence::none(),
            signature: [0u8; 64],
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    /// Get the number of nodes in the execution trace.
    pub fn trace_length(&self) -> usize {
        self.execution_trace.len()
    }

    /// Check if the PCA has been signed.
    pub fn is_signed(&self) -> bool {
        self.signature.iter().any(|&b| b != 0)
    }
}

impl fmt::Display for ProofCarryingAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PCA[{}] conf={} trace={} signed={}",
            self.action.action_type(),
            self.confidence,
            self.trace_length(),
            self.is_signed()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let hash1 = ContentHash::from_string("hello");
        let hash2 = ContentHash::from_string("hello");
        let hash3 = ContentHash::from_string("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_content_hash_hex() {
        let hash = ContentHash::from_string("test");
        let hex = hash.to_hex();
        let restored = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(hash, restored);
    }

    #[test]
    fn test_confidence() {
        let conf = Confidence::new(0.8);
        assert!(conf.meets_threshold(0.7));
        assert!(!conf.meets_threshold(0.9));
    }

    #[test]
    fn test_confidence_clamping() {
        let conf_over = Confidence::new(1.5);
        assert_eq!(conf_over.value(), 1.0);

        let conf_under = Confidence::new(-0.5);
        assert_eq!(conf_under.value(), 0.0);
    }

    #[test]
    fn test_confidence_combine() {
        let scores = vec![
            Confidence::new(0.9),
            Confidence::new(0.8),
            Confidence::new(0.7),
        ];
        let combined = Confidence::combine(&scores);
        // Geometric mean of 0.9, 0.8, 0.7 ≈ 0.797
        assert!(combined.value() > 0.79 && combined.value() < 0.81);
    }

    #[test]
    fn test_incoming_message() {
        let msg = IncomingMessage::new("telegram", "user123", "Hello");
        assert_eq!(msg.channel_id, "telegram");
        assert_eq!(msg.sender_id, "user123");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_outgoing_message() {
        let msg = OutgoingMessage::new("discord", "channel456", "Hi there");
        assert_eq!(msg.channel_id, "discord");
        assert_eq!(msg.recipient_id, "channel456");
        assert!(msg.reply_to.is_none());
    }

    #[test]
    fn test_action_type() {
        let action = Action::SendMessage(OutgoingMessage::new("test", "user", "msg"));
        assert_eq!(action.action_type(), "SendMessage");

        let noop = Action::NoOp { reason: "test".to_string() };
        assert!(noop.is_noop());
    }
}
