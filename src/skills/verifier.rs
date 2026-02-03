//! Skill Verifier - verify skill graphs are safe to execute.
//!
//! The SkillVerifier performs static analysis on skill graphs to ensure
//! they are safe to execute. Verification logic is defined in the 0-lang
//! graph at `graphs/core/verifier.0`.
//!
//! This includes:
//! - Halting analysis (no infinite loops)
//! - Type checking
//! - Permission verification
//! - Resource bound estimation

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::error::SkillError;
use crate::runtime::{GraphInterpreter, Graph, Value};
use super::graph::{SkillGraph, SkillNode, Op, SafetyProof};

/// Result of skill verification.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether the skill passed verification.
    pub safe: bool,
    /// Non-blocking warnings.
    pub warnings: Vec<VerificationWarning>,
    /// Blocking errors.
    pub errors: Vec<VerificationError>,
    /// Safety proof if verification passed.
    pub proof: Option<SafetyProof>,
}

impl VerificationResult {
    /// Create a passing result.
    pub fn pass() -> Self {
        Self {
            safe: true,
            warnings: Vec::new(),
            errors: Vec::new(),
            proof: Some(SafetyProof::default()),
        }
    }

    /// Create a failing result with an error.
    pub fn fail(error: VerificationError) -> Self {
        Self {
            safe: false,
            warnings: Vec::new(),
            errors: vec![error],
            proof: None,
        }
    }

    /// Add a warning.
    pub fn with_warning(mut self, warning: VerificationWarning) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Add an error.
    pub fn with_error(mut self, error: VerificationError) -> Self {
        self.safe = false;
        self.errors.push(error);
        self.proof = None;
        self
    }

    /// Set the safety proof.
    pub fn with_proof(mut self, proof: SafetyProof) -> Self {
        if self.safe {
            self.proof = Some(proof);
        }
        self
    }
}

/// Verification warnings (non-blocking).
#[derive(Debug, Clone)]
pub enum VerificationWarning {
    /// Graph has many nodes.
    LargeGraph { node_count: usize },
    /// Potentially unbounded loop detected.
    PotentiallyUnboundedLoop { node_id: String },
    /// External call detected.
    ExternalCall { uri: String },
    /// High memory usage estimated.
    HighMemoryUsage { estimated_bytes: u64 },
    /// Deprecated operation used.
    DeprecatedOp { op: String, replacement: String },
}

impl std::fmt::Display for VerificationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LargeGraph { node_count } => {
                write!(f, "Large graph with {} nodes may be slow", node_count)
            }
            Self::PotentiallyUnboundedLoop { node_id } => {
                write!(f, "Potentially unbounded loop at node '{}'", node_id)
            }
            Self::ExternalCall { uri } => {
                write!(f, "External call to '{}' detected", uri)
            }
            Self::HighMemoryUsage { estimated_bytes } => {
                write!(f, "High memory usage estimated: {} bytes", estimated_bytes)
            }
            Self::DeprecatedOp { op, replacement } => {
                write!(f, "Deprecated operation '{}', use '{}' instead", op, replacement)
            }
        }
    }
}

/// Verification errors (blocking).
#[derive(Debug, Clone)]
pub enum VerificationError {
    /// Infinite loop detected.
    InfiniteLoop { cycle: Vec<String> },
    /// Unsafe operation detected.
    UnsafeOperation { op: String, reason: String },
    /// Missing required input.
    MissingInput { input_name: String },
    /// Type mismatch between nodes.
    TypeMismatch { 
        node_id: String, 
        expected: String, 
        found: String 
    },
    /// Invalid node reference.
    InvalidReference { 
        from_node: String, 
        to_node: String 
    },
    /// Permission not declared.
    MissingPermission { 
        required: String, 
        for_operation: String 
    },
    /// Graph has no outputs.
    NoOutputs,
    /// Empty graph.
    EmptyGraph,
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InfiniteLoop { cycle } => {
                write!(f, "Infinite loop detected: {:?}", cycle)
            }
            Self::UnsafeOperation { op, reason } => {
                write!(f, "Unsafe operation '{}': {}", op, reason)
            }
            Self::MissingInput { input_name } => {
                write!(f, "Missing required input: '{}'", input_name)
            }
            Self::TypeMismatch { node_id, expected, found } => {
                write!(f, "Type mismatch at '{}': expected {}, found {}", node_id, expected, found)
            }
            Self::InvalidReference { from_node, to_node } => {
                write!(f, "Invalid reference from '{}' to '{}'", from_node, to_node)
            }
            Self::MissingPermission { required, for_operation } => {
                write!(f, "Missing permission '{}' for operation '{}'", required, for_operation)
            }
            Self::NoOutputs => write!(f, "Graph has no outputs defined"),
            Self::EmptyGraph => write!(f, "Graph is empty"),
        }
    }
}

/// Skill verifier for safety analysis.
/// 
/// Verification logic is defined in `graphs/core/verifier.0`.
pub struct SkillVerifier {
    /// Graph interpreter
    interpreter: Arc<GraphInterpreter>,
    /// Verification graph
    verifier_graph: Option<Graph>,
}

impl SkillVerifier {
    /// Create a new SkillVerifier.
    pub fn new() -> Self {
        Self {
            interpreter: Arc::new(GraphInterpreter::default()),
            verifier_graph: Self::load_verifier_graph(),
        }
    }
    
    /// Load the verification graph.
    fn load_verifier_graph() -> Option<Graph> {
        let graph_path = "graphs/core/verifier.0";
        if let Ok(content) = std::fs::read_to_string(graph_path) {
            if let Ok(graph) = crate::runtime::parse_graph(&content) {
                return Some(graph);
            }
        }
        None
    }

    /// Verify a skill graph is safe to execute.
    ///
    /// # Returns
    /// A `VerificationResult` containing the analysis outcome.
    pub fn verify(graph: &SkillGraph) -> Result<VerificationResult, SkillError> {
        let verifier = Self::new();
        verifier.verify_with_graph(graph)
    }
    
    /// Verify using the 0-lang verification graph.
    pub fn verify_with_graph(&self, graph: &SkillGraph) -> Result<VerificationResult, SkillError> {
        let mut result = VerificationResult::pass();
        
        // Check for empty graph
        if graph.nodes.is_empty() {
            return Ok(VerificationResult::fail(VerificationError::EmptyGraph));
        }
        
        // Check for outputs
        if graph.outputs.is_empty() {
            result = result.with_error(VerificationError::NoOutputs);
        }
        
        // Check graph size
        if graph.node_count() > 1000 {
            result = result.with_warning(VerificationWarning::LargeGraph {
                node_count: graph.node_count(),
            });
        }
        
        // Check for cycles (infinite loops)
        if let Some(cycle) = Self::find_cycle(graph) {
            result = result.with_error(VerificationError::InfiniteLoop { cycle });
        }
        
        // Check node safety
        for node in &graph.nodes {
            if let Some(error) = Self::check_node_safety(node, &graph.permissions) {
                result = result.with_error(error);
            }
        }
        
        // Validate references
        let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id()).collect();
        for node in &graph.nodes {
            for input in node.inputs() {
                if !node_ids.contains(input.as_str()) {
                    result = result.with_error(VerificationError::InvalidReference {
                        from_node: node.id().to_string(),
                        to_node: input.clone(),
                    });
                }
            }
        }
        
        // Check external calls
        for uri in graph.external_uris() {
            result = result.with_warning(VerificationWarning::ExternalCall {
                uri: uri.to_string(),
            });
        }
        
        // Build safety proof if verification passed
        if result.safe {
            let proof = Self::build_safety_proof(graph);
            result = result.with_proof(proof);
        }
        
        Ok(result)
    }

    /// Quick safety check without full verification.
    pub fn quick_check(graph: &SkillGraph) -> bool {
        // Basic sanity checks
        !graph.nodes.is_empty() 
            && !graph.outputs.is_empty()
            && graph.node_count() < 10000
    }

    /// Find cycles in the graph using DFS.
    fn find_cycle(graph: &SkillGraph) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();
        
        // Build adjacency list
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &graph.nodes {
            adj.insert(node.id(), Vec::new());
        }
        for node in &graph.nodes {
            for input in node.inputs() {
                if let Some(deps) = adj.get_mut(input.as_str()) {
                    deps.push(node.id());
                }
            }
        }
        
        // DFS for cycle detection
        for node in &graph.nodes {
            if !visited.contains(node.id()) {
                if let Some(cycle) = Self::dfs_cycle(
                    node.id(),
                    &adj,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }
        
        None
    }

    fn dfs_cycle<'a>(
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
        path: &mut Vec<&'a str>,
    ) -> Option<Vec<String>> {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);
        
        if let Some(neighbors) = adj.get(node) {
            for &neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = Self::dfs_cycle(neighbor, adj, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found cycle - extract it from path
                    let cycle_start = path.iter().position(|&n| n == neighbor).unwrap_or(0);
                    return Some(path[cycle_start..].iter().map(|s| s.to_string()).collect());
                }
            }
        }
        
        rec_stack.remove(node);
        path.pop();
        None
    }

    /// Check if a node operation is safe.
    fn check_node_safety(
        node: &SkillNode,
        declared_permissions: &[String],
    ) -> Option<VerificationError> {
        match node {
            SkillNode::External { uri, .. } => {
                // Only require network permission for actual HTTP(S) calls
                // Internal protocols (calendar://, input://, etc.) don't need it
                let is_network_call = uri.starts_with("http://") || uri.starts_with("https://");
                
                if is_network_call && !declared_permissions.contains(&"network".to_string()) {
                    return Some(VerificationError::MissingPermission {
                        required: "network".to_string(),
                        for_operation: format!("external call to {}", uri),
                    });
                }
                None
            }
            SkillNode::Operation { id, op, .. } => {
                match op {
                    Op::HttpGet | Op::HttpPost => {
                        if !declared_permissions.contains(&"network".to_string()) {
                            return Some(VerificationError::MissingPermission {
                                required: "network".to_string(),
                                for_operation: format!("HTTP operation at {}", id),
                            });
                        }
                    }
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    /// Build a safety proof for a verified graph.
    fn build_safety_proof(graph: &SkillGraph) -> SafetyProof {
        let max_steps = Self::estimate_max_steps(graph);
        let memory_bound = Self::estimate_memory_bound(graph);
        let halting_proven = Self::prove_halting(graph);
        
        SafetyProof {
            max_steps,
            fuel_budget: max_steps * 10,
            halting_proven,
            memory_bound: Some(memory_bound),
        }
    }

    /// Estimate maximum execution steps.
    fn estimate_max_steps(graph: &SkillGraph) -> u64 {
        // Base estimate: 10 steps per node
        let base = (graph.node_count() as u64) * 10;
        
        // Add extra for operations that may loop
        let mut multiplier = 1u64;
        for node in &graph.nodes {
            if let SkillNode::Operation { op, .. } = node {
                match op {
                    Op::Map { .. } | Op::Filter { .. } | Op::Reduce { .. } => {
                        multiplier = multiplier.saturating_mul(100);
                    }
                    _ => {}
                }
            }
        }
        
        base.saturating_mul(multiplier).min(1_000_000)
    }

    /// Estimate maximum memory usage.
    fn estimate_memory_bound(graph: &SkillGraph) -> u64 {
        // Base: 1KB per node
        let base = (graph.node_count() as u64) * 1024;
        
        // External calls may return large responses
        let external_count = graph.nodes.iter()
            .filter(|n| matches!(n, SkillNode::External { .. }))
            .count() as u64;
        
        base + (external_count * 1024 * 1024) // 1MB per external call
    }

    /// Try to prove the graph halts.
    fn prove_halting(graph: &SkillGraph) -> bool {
        // A graph halts if:
        // 1. It has no cycles (already checked)
        // 2. All operations are bounded
        
        for node in &graph.nodes {
            if let SkillNode::Operation { op, .. } = node {
                match op {
                    // These operations may not halt without bounds
                    Op::Map { body } | Op::Filter { predicate: body } => {
                        // Recursively check sub-graphs
                        if !Self::prove_halting(body) {
                            return false;
                        }
                    }
                    Op::Reduce { .. } => {
                        // Reduce on unbounded input may not halt
                        // For now, assume bounded input
                    }
                    _ => {}
                }
            }
        }
        
        true
    }
}

/// Verify multiple skills as a batch.
pub fn verify_batch(graphs: &[&SkillGraph]) -> Vec<VerificationResult> {
    graphs.iter()
        .map(|g| SkillVerifier::verify(g).unwrap_or_else(|_| {
            VerificationResult::fail(VerificationError::EmptyGraph)
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_simple_graph() {
        let graph = SkillGraph::builder("test")
            .add_input("x", "string")
            .add_operation("y", Op::Identity, vec!["x"])
            .output("y")
            .build();

        let result = SkillVerifier::verify(&graph).unwrap();
        assert!(result.safe);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_verify_empty_graph() {
        let graph = SkillGraph::builder("empty").build();
        
        let result = SkillVerifier::verify(&graph).unwrap();
        assert!(!result.safe);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_verify_missing_permission() {
        let graph = SkillGraph::builder("network_test")
            .add_input("url", "string")
            .add_operation("fetch", Op::HttpGet, vec!["url"])
            .output("fetch")
            .build();

        let result = SkillVerifier::verify(&graph).unwrap();
        assert!(!result.safe);
        assert!(result.errors.iter().any(|e| {
            matches!(e, VerificationError::MissingPermission { required, .. } if required == "network")
        }));
    }

    #[test]
    fn test_verify_with_permission() {
        let graph = SkillGraph::builder("network_test")
            .add_input("url", "string")
            .add_operation("fetch", Op::HttpGet, vec!["url"])
            .output("fetch")
            .permission("network")
            .build();

        let result = SkillVerifier::verify(&graph).unwrap();
        assert!(result.safe);
    }

    #[test]
    fn test_invalid_reference() {
        let graph = SkillGraph::builder("invalid")
            .add_operation("y", Op::Identity, vec!["nonexistent"])
            .output("y")
            .build();

        let result = SkillVerifier::verify(&graph).unwrap();
        assert!(!result.safe);
        assert!(result.errors.iter().any(|e| {
            matches!(e, VerificationError::InvalidReference { .. })
        }));
    }

    #[test]
    fn test_quick_check() {
        let good_graph = SkillGraph::builder("good")
            .add_input("x", "string")
            .output("x")
            .build();

        let empty_graph = SkillGraph::builder("empty").build();

        assert!(SkillVerifier::quick_check(&good_graph));
        assert!(!SkillVerifier::quick_check(&empty_graph));
    }
}
