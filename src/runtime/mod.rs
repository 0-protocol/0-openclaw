//! 0-lang Graph Runtime
//!
//! This module provides the core interpreter for executing 0-lang graphs.
//! All business logic should be expressed as 0-lang graphs, with this runtime
//! providing only the execution engine and built-in operations.

mod interpreter;
mod builtins;
pub mod types;

pub use interpreter::{GraphInterpreter, ExecutionContext, ExecutionResult};
pub use builtins::{BuiltinOp, BuiltinRegistry};
pub use types::{Value, GraphNode, Graph, NodeType, Edge};

use crate::error::GatewayError;

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum execution steps (prevents infinite loops)
    pub max_steps: usize,
    /// Enable execution tracing
    pub trace_enabled: bool,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_steps: 10000,
            trace_enabled: true,
            timeout_ms: 30000,
        }
    }
}

/// Create a new graph interpreter with default configuration
pub fn create_interpreter() -> GraphInterpreter {
    GraphInterpreter::new(RuntimeConfig::default())
}

/// Load a graph from a .0 file
pub fn load_graph(path: &str) -> Result<Graph, GatewayError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| GatewayError::ConfigError(format!("Failed to read graph: {}", e)))?;
    parse_graph(&content)
}

/// Parse a graph from 0-lang source
pub fn parse_graph(source: &str) -> Result<Graph, GatewayError> {
    types::parse_graph_from_source(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_interpreter() {
        let interp = create_interpreter();
        assert!(interp.builtins().len() > 0);
    }

    #[tokio::test]
    async fn test_simple_graph_execution() {
        let interp = create_interpreter();
        
        // Create a simple echo graph
        let graph = Graph {
            name: "test_echo".to_string(),
            version: 1,
            description: "Test echo graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "input".to_string(),
                    node_type: NodeType::External {
                        uri: "input://message".to_string(),
                    },
                    inputs: vec![],
                    params: serde_json::json!({}),
                },
                GraphNode {
                    id: "output".to_string(),
                    node_type: NodeType::Operation {
                        op: "Identity".to_string(),
                    },
                    inputs: vec!["input".to_string()],
                    params: serde_json::json!({}),
                },
            ],
            outputs: vec!["output".to_string()],
            entry_point: "input".to_string(),
            metadata: serde_json::json!({}),
        };

        let mut inputs = std::collections::HashMap::new();
        inputs.insert("message".to_string(), Value::String("hello".to_string()));

        let result = interp.execute(&graph, inputs).await.unwrap();
        assert!(result.outputs.contains_key("output"));
    }
}
