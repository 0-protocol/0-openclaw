//! Proof-Carrying Action generation for 0-openclaw.
//!
//! This module handles cryptographic signing and verification of actions,
//! creating the core "proof-carrying" property that makes 0-openclaw trustworthy.

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use std::path::Path;

use crate::types::{ContentHash, Confidence, Action, ProofCarryingAction};
use crate::error::ProofError;

/// Execution trace from graph evaluation.
#[derive(Debug, Clone, Default)]
pub struct ExecutionTrace {
    /// Hashes of evaluated nodes
    pub nodes: Vec<ContentHash>,
    
    /// Whether this trace came from cache
    pub cached: bool,
    
    /// Total execution time in microseconds
    pub execution_time_us: u64,
}

impl ExecutionTrace {
    /// Create a new execution trace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cached trace (for repeated/cached operations).
    pub fn cached() -> Self {
        Self {
            nodes: Vec::new(),
            cached: true,
            execution_time_us: 0,
        }
    }

    /// Add a node to the trace.
    pub fn add_node(&mut self, hash: ContentHash) {
        self.nodes.push(hash);
    }

    /// Get the number of nodes in the trace.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the trace is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Create from a graph execution result.
    pub fn from_graph_execution(exec_result: &crate::runtime::ExecutionResult) -> Self {
        Self {
            nodes: exec_result.trace.iter()
                .map(|node_id| ContentHash::from_string(node_id))
                .collect(),
            cached: false,
            execution_time_us: 0,
        }
    }
}

/// Generator for Proof-Carrying Actions.
pub struct ProofGenerator {
    /// Ed25519 signing key
    signing_key: SigningKey,
    
    /// Cached verifying key
    verifying_key: VerifyingKey,
    
    /// Graph interpreter for proof calculations
    interpreter: std::sync::Arc<crate::runtime::GraphInterpreter>,
    
    /// Proof generation graph
    proof_graph: Option<crate::runtime::Graph>,
}

impl ProofGenerator {
    /// Create a new ProofGenerator with a random keypair.
    pub fn new_random() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let interpreter = std::sync::Arc::new(crate::runtime::GraphInterpreter::default());
        let proof_graph = Self::load_proof_graph();
        
        Self {
            signing_key,
            verifying_key,
            interpreter,
            proof_graph,
        }
    }

    /// Load the proof generation graph.
    fn load_proof_graph() -> Option<crate::runtime::Graph> {
        let graph_path = "graphs/core/proof.0";
        if let Ok(content) = std::fs::read_to_string(graph_path) {
            if let Ok(graph) = crate::runtime::parse_graph(&content) {
                return Some(graph);
            }
        }
        None
    }

    /// Load a ProofGenerator from a keypair file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ProofError> {
        let key_bytes = std::fs::read(path)
            .map_err(|e| ProofError::KeyGenerationFailed(e.to_string()))?;

        if key_bytes.len() != 32 {
            return Err(ProofError::KeyGenerationFailed(
                "Invalid key length, expected 32 bytes".to_string()
            ));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&key_bytes);
        
        let signing_key = SigningKey::from_bytes(&arr);
        let verifying_key = signing_key.verifying_key();
        let interpreter = std::sync::Arc::new(crate::runtime::GraphInterpreter::default());
        let proof_graph = Self::load_proof_graph();

        Ok(Self {
            signing_key,
            verifying_key,
            interpreter,
            proof_graph,
        })
    }

    /// Save the keypair to a file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ProofError> {
        std::fs::write(path, self.signing_key.to_bytes())
            .map_err(|e| ProofError::KeyGenerationFailed(e.to_string()))
    }

    /// Get the public verifying key.
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Get the public key as bytes.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Generate a Proof-Carrying Action.
    pub fn generate(
        &self,
        action: Action,
        session_hash: ContentHash,
        input_hash: ContentHash,
        traces: Vec<ExecutionTrace>,
    ) -> Result<ProofCarryingAction, ProofError> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;

        // Combine all traces
        let execution_trace: Vec<ContentHash> = traces
            .iter()
            .flat_map(|t| t.nodes.iter().copied())
            .collect();

        // Calculate combined confidence
        let confidence = self.calculate_confidence(&execution_trace, &traces);

        // Build message to sign
        let message = self.build_sign_message(
            &action,
            &session_hash,
            &input_hash,
            &execution_trace,
            confidence,
            timestamp,
        );

        // Sign the message
        let signature: Signature = self.signing_key.sign(&message);

        Ok(ProofCarryingAction {
            action,
            session_hash,
            input_hash,
            execution_trace,
            confidence,
            signature: signature.to_bytes(),
            timestamp,
        })
    }

    /// Verify a Proof-Carrying Action.
    pub fn verify(&self, pca: &ProofCarryingAction) -> Result<bool, ProofError> {
        let message = self.build_sign_message(
            &pca.action,
            &pca.session_hash,
            &pca.input_hash,
            &pca.execution_trace,
            pca.confidence,
            pca.timestamp,
        );

        let signature = Signature::from_bytes(&pca.signature);
        
        self.verifying_key
            .verify(&message, &signature)
            .map(|_| true)
            .map_err(|e| ProofError::VerificationFailed(e.to_string()))
    }

    /// Verify a PCA with a specific public key.
    pub fn verify_with_key(
        pca: &ProofCarryingAction,
        public_key: &VerifyingKey,
    ) -> Result<bool, ProofError> {
        let message = Self::build_sign_message_static(
            &pca.action,
            &pca.session_hash,
            &pca.input_hash,
            &pca.execution_trace,
            pca.confidence,
            pca.timestamp,
        );

        let signature = Signature::from_bytes(&pca.signature);
        
        public_key
            .verify(&message, &signature)
            .map(|_| true)
            .map_err(|e| ProofError::VerificationFailed(e.to_string()))
    }

    /// Build the message to be signed.
    fn build_sign_message(
        &self,
        action: &Action,
        session_hash: &ContentHash,
        input_hash: &ContentHash,
        execution_trace: &[ContentHash],
        confidence: Confidence,
        timestamp: u64,
    ) -> Vec<u8> {
        Self::build_sign_message_static(
            action,
            session_hash,
            input_hash,
            execution_trace,
            confidence,
            timestamp,
        )
    }

    /// Static version of build_sign_message for use without self.
    fn build_sign_message_static(
        action: &Action,
        session_hash: &ContentHash,
        input_hash: &ContentHash,
        execution_trace: &[ContentHash],
        confidence: Confidence,
        timestamp: u64,
    ) -> Vec<u8> {
        let mut message = Vec::new();

        // Serialize action
        let action_bytes = serde_json::to_vec(action).unwrap_or_default();
        message.extend_from_slice(&action_bytes);
        
        // Add session hash
        message.extend_from_slice(session_hash.as_bytes());
        
        // Add input hash
        message.extend_from_slice(input_hash.as_bytes());

        // Add execution trace
        for trace_hash in execution_trace {
            message.extend_from_slice(trace_hash.as_bytes());
        }

        // Add confidence
        message.extend_from_slice(&confidence.value().to_le_bytes());
        
        // Add timestamp
        message.extend_from_slice(&timestamp.to_le_bytes());

        message
    }

    /// Calculate confidence score from execution traces using the 0-lang graph.
    fn calculate_confidence(
        &self,
        trace: &[ContentHash],
        traces: &[ExecutionTrace],
    ) -> Confidence {
        // Try graph-based calculation if available
        if let Some(_graph) = &self.proof_graph {
            // For now, use fallback - graph-based confidence would require
            // async execution which is complex for this sync context
            // The graph is prepared for future async refactoring
        }
        
        // Fallback to direct calculation
        Self::calculate_confidence_fallback(trace, traces)
    }
    
    /// Fallback confidence calculation without graph.
    fn calculate_confidence_fallback(
        trace: &[ContentHash],
        traces: &[ExecutionTrace],
    ) -> Confidence {
        // Start with high confidence
        let base = 0.99;

        // Decay based on trace length (more complex paths = slightly lower confidence)
        let length_decay = 0.001 * trace.len() as f32;

        // Bonus for cached operations (they've been verified before)
        let cache_bonus = traces.iter()
            .filter(|t| t.cached)
            .count() as f32 * 0.001;

        let value = (base - length_decay + cache_bonus).clamp(0.5, 1.0);
        Confidence::new(value)
    }
}

/// Builder for creating Proof-Carrying Actions step by step.
pub struct ProofBuilder<'a> {
    generator: &'a ProofGenerator,
    action: Option<Action>,
    session_hash: Option<ContentHash>,
    input_hash: Option<ContentHash>,
    traces: Vec<ExecutionTrace>,
}

impl<'a> ProofBuilder<'a> {
    /// Create a new proof builder.
    pub fn new(generator: &'a ProofGenerator) -> Self {
        Self {
            generator,
            action: None,
            session_hash: None,
            input_hash: None,
            traces: Vec::new(),
        }
    }

    /// Set the action.
    pub fn action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }

    /// Set the session hash.
    pub fn session_hash(mut self, hash: ContentHash) -> Self {
        self.session_hash = Some(hash);
        self
    }

    /// Set the input hash.
    pub fn input_hash(mut self, hash: ContentHash) -> Self {
        self.input_hash = Some(hash);
        self
    }

    /// Add an execution trace.
    pub fn add_trace(mut self, trace: ExecutionTrace) -> Self {
        self.traces.push(trace);
        self
    }

    /// Build the Proof-Carrying Action.
    pub fn build(self) -> Result<ProofCarryingAction, ProofError> {
        let action = self.action
            .ok_or_else(|| ProofError::SigningFailed("Missing action".to_string()))?;
        let session_hash = self.session_hash
            .ok_or_else(|| ProofError::SigningFailed("Missing session hash".to_string()))?;
        let input_hash = self.input_hash
            .ok_or_else(|| ProofError::SigningFailed("Missing input hash".to_string()))?;

        self.generator.generate(action, session_hash, input_hash, self.traces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OutgoingMessage;

    #[test]
    fn test_proof_generator_creation() {
        let generator = ProofGenerator::new_random();
        assert_eq!(generator.public_key_bytes().len(), 32);
    }

    #[test]
    fn test_generate_and_verify() {
        let generator = ProofGenerator::new_random();
        
        let action = Action::SendMessage(OutgoingMessage::new("test", "user", "Hello"));
        let session_hash = ContentHash::from_string("session");
        let input_hash = ContentHash::from_string("input");
        
        let pca = generator.generate(
            action,
            session_hash,
            input_hash,
            vec![ExecutionTrace::new()],
        ).unwrap();

        assert!(pca.is_signed());
        assert!(generator.verify(&pca).unwrap());
    }

    #[test]
    fn test_tampered_pca_fails_verification() {
        let generator = ProofGenerator::new_random();
        
        let action = Action::SendMessage(OutgoingMessage::new("test", "user", "Hello"));
        let session_hash = ContentHash::from_string("session");
        let input_hash = ContentHash::from_string("input");
        
        let mut pca = generator.generate(
            action,
            session_hash,
            input_hash,
            vec![],
        ).unwrap();

        // Tamper with the PCA
        pca.confidence = Confidence::new(0.1);

        // Verification should fail
        assert!(generator.verify(&pca).is_err());
    }

    #[test]
    fn test_execution_trace() {
        let mut trace = ExecutionTrace::new();
        assert!(trace.is_empty());

        trace.add_node(ContentHash::from_string("node1"));
        trace.add_node(ContentHash::from_string("node2"));
        
        assert_eq!(trace.len(), 2);
        assert!(!trace.is_empty());
    }

    #[test]
    fn test_cached_trace() {
        let trace = ExecutionTrace::cached();
        assert!(trace.cached);
        assert!(trace.is_empty());
    }

    #[test]
    fn test_proof_builder() {
        let generator = ProofGenerator::new_random();
        
        let pca = ProofBuilder::new(&generator)
            .action(Action::NoOp { reason: "test".to_string() })
            .session_hash(ContentHash::from_string("session"))
            .input_hash(ContentHash::from_string("input"))
            .add_trace(ExecutionTrace::new())
            .build()
            .unwrap();

        assert!(pca.is_signed());
    }

    #[test]
    fn test_confidence_calculation() {
        let generator = ProofGenerator::new_random();
        
        // Short trace should have high confidence
        let short_trace = vec![ContentHash::from_string("node1")];
        let short_traces = vec![ExecutionTrace { nodes: short_trace.clone(), cached: false, execution_time_us: 0 }];
        let short_conf = generator.calculate_confidence(&short_trace, &short_traces);
        
        // Long trace should have lower confidence
        let long_trace: Vec<ContentHash> = (0..100)
            .map(|i| ContentHash::from_string(&format!("node{}", i)))
            .collect();
        let long_traces = vec![ExecutionTrace { nodes: long_trace.clone(), cached: false, execution_time_us: 0 }];
        let long_conf = generator.calculate_confidence(&long_trace, &long_traces);
        
        assert!(short_conf.value() > long_conf.value());
    }
}
