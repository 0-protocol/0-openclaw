//! Graph interpreter for 0-lang.
//!
//! This is the core execution engine that interprets 0-lang graphs.
//! All business logic should be expressed as graphs; this interpreter
//! provides the minimal runtime to execute them.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::builtins::BuiltinRegistry;
use super::types::{Graph, GraphNode, NodeType, Value, RouteCondition};
use super::RuntimeConfig;
use crate::error::GatewayError;
use crate::types::ContentHash;

/// Result of graph execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Output values keyed by output node ID.
    pub outputs: HashMap<String, Value>,
    /// Execution trace (node IDs in execution order).
    pub trace: Vec<String>,
    /// Content hash of the execution.
    pub hash: ContentHash,
    /// Final confidence score.
    pub confidence: f64,
}

/// Execution context for a graph.
#[derive(Debug)]
pub struct ExecutionContext {
    /// Node outputs computed so far.
    pub node_values: HashMap<String, Value>,
    /// Execution trace.
    pub trace: Vec<String>,
    /// Current confidence score.
    pub confidence: f64,
    /// Step counter.
    pub steps: usize,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            node_values: HashMap::new(),
            trace: Vec::new(),
            confidence: 1.0,
            steps: 0,
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// The 0-lang graph interpreter.
pub struct GraphInterpreter {
    /// Built-in operations.
    builtins: BuiltinRegistry,
    /// Runtime configuration.
    config: RuntimeConfig,
    /// State store for cross-execution state.
    state_store: Arc<RwLock<HashMap<String, Value>>>,
}

impl GraphInterpreter {
    /// Create a new interpreter with the given configuration.
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            builtins: BuiltinRegistry::new(),
            config,
            state_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the builtin registry.
    pub fn builtins(&self) -> &BuiltinRegistry {
        &self.builtins
    }

    /// Execute a graph with the given inputs.
    pub async fn execute(
        &self,
        graph: &Graph,
        inputs: HashMap<String, Value>,
    ) -> Result<ExecutionResult, GatewayError> {
        let mut ctx = ExecutionContext::new();

        // Topologically sort nodes
        let sorted_nodes = graph.topo_sort()?;

        // Execute nodes in order
        for node in sorted_nodes {
            if ctx.steps >= self.config.max_steps {
                return Err(GatewayError::ExecutionError(
                    "Maximum execution steps exceeded".to_string(),
                ));
            }

            let value = self.execute_node(node, &inputs, &mut ctx).await?;
            ctx.node_values.insert(node.id.clone(), value);
            ctx.trace.push(node.id.clone());
            ctx.steps += 1;
        }

        // Collect outputs
        let mut outputs = HashMap::new();
        for output_id in &graph.outputs {
            if let Some(value) = ctx.node_values.get(output_id) {
                outputs.insert(output_id.clone(), value.clone());
            }
        }

        // Compute execution hash
        let hash = self.compute_execution_hash(&ctx)?;

        Ok(ExecutionResult {
            outputs,
            trace: ctx.trace,
            hash,
            confidence: ctx.confidence,
        })
    }

    /// Execute a single node.
    async fn execute_node(
        &self,
        node: &GraphNode,
        inputs: &HashMap<String, Value>,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, GatewayError> {
        match &node.node_type {
            NodeType::External { uri } => {
                // Extract input from provided inputs
                let key = uri.strip_prefix("input://").unwrap_or(uri);
                Ok(inputs.get(key).cloned().unwrap_or(Value::Null))
            }

            NodeType::Constant { value } => {
                Ok(value.clone())
            }

            NodeType::Operation { op } => {
                // Gather inputs
                let input_values = self.gather_inputs(&node.inputs, ctx)?;

                // Execute builtin
                if let Some(builtin) = self.builtins.get(op) {
                    builtin.execute(input_values, &node.params).await
                } else {
                    Err(GatewayError::ExecutionError(format!(
                        "Unknown operation: {}",
                        op
                    )))
                }
            }

            NodeType::Lookup { table, default } => {
                // Get lookup key from first input
                let key = self.gather_inputs(&node.inputs, ctx)?
                    .first()
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                let result = table.get(&key)
                    .or(default.as_ref())
                    .cloned()
                    .unwrap_or_default();

                Ok(Value::String(result))
            }

            NodeType::Route { conditions } => {
                self.execute_route(conditions, ctx).await
            }

            NodeType::Permission { action, min_confidence } => {
                // Check permission based on sender context
                let sender_confidence = ctx.node_values
                    .get("sender_confidence")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.5);

                let granted = sender_confidence >= *min_confidence;
                let mut result = HashMap::new();
                result.insert("granted".to_string(), Value::Bool(granted));
                result.insert("confidence".to_string(), Value::Confidence(sender_confidence));
                result.insert("action".to_string(), Value::String(action.clone()));

                // Update context confidence
                if granted {
                    ctx.confidence *= sender_confidence;
                } else {
                    ctx.confidence *= 0.1; // Heavily penalize denied permissions
                }

                Ok(Value::Map(result))
            }
        }
    }

    /// Execute a routing decision.
    async fn execute_route(
        &self,
        conditions: &[RouteCondition],
        ctx: &mut ExecutionContext,
    ) -> Result<Value, GatewayError> {
        for condition in conditions {
            let input_value = ctx.node_values
                .get(&condition.input)
                .cloned()
                .unwrap_or(Value::Null);

            let matches = if let Some(match_value) = &condition.match_value {
                // Exact match
                input_value.as_string().map(|s| s == match_value).unwrap_or(false)
            } else if condition.threshold > 0.0 {
                // Threshold match
                input_value.is_truthy()
            } else {
                // Default route
                true
            };

            if matches {
                ctx.confidence *= condition.confidence;
                
                let mut result = HashMap::new();
                result.insert("target".to_string(), Value::String(condition.target.clone()));
                result.insert("confidence".to_string(), Value::Confidence(condition.confidence));
                result.insert("matched_input".to_string(), Value::String(condition.input.clone()));
                
                return Ok(Value::Map(result));
            }
        }

        // No match - return null
        Ok(Value::Null)
    }

    /// Gather input values for a node.
    fn gather_inputs(
        &self,
        input_refs: &[String],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Value>, GatewayError> {
        let mut values = Vec::new();
        
        for input_ref in input_refs {
            // Handle field references like "node.field"
            let value = if input_ref.contains('.') {
                let parts: Vec<&str> = input_ref.splitn(2, '.').collect();
                let node_id = parts[0];
                let field = parts[1];
                
                ctx.node_values
                    .get(node_id)
                    .and_then(|v| {
                        if let Value::Map(m) = v {
                            m.get(field).cloned()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(Value::Null)
            } else {
                ctx.node_values.get(input_ref).cloned().unwrap_or(Value::Null)
            };
            
            values.push(value);
        }
        
        Ok(values)
    }

    /// Compute content hash of the execution.
    fn compute_execution_hash(&self, ctx: &ExecutionContext) -> Result<ContentHash, GatewayError> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Hash the trace
        for node_id in &ctx.trace {
            hasher.update(node_id.as_bytes());
            if let Some(value) = ctx.node_values.get(node_id) {
                let value_bytes = serde_json::to_vec(value).unwrap_or_default();
                hasher.update(&value_bytes);
            }
        }
        
        let result = hasher.finalize();
        Ok(ContentHash::from_bytes(&result))
    }

    /// Load state for a session.
    pub async fn load_state(&self, session_id: &str) -> Value {
        let store = self.state_store.read().await;
        store.get(session_id).cloned().unwrap_or(Value::Null)
    }

    /// Save state for a session.
    pub async fn save_state(&self, session_id: &str, state: Value) {
        let mut store = self.state_store.write().await;
        store.insert(session_id.to_string(), state);
    }
}

impl Default for GraphInterpreter {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::types::NodeType;

    fn create_test_graph() -> Graph {
        Graph {
            name: "test".to_string(),
            version: 1,
            description: "Test graph".to_string(),
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
                    id: "check_command".to_string(),
                    node_type: NodeType::Operation {
                        op: "StartsWith".to_string(),
                    },
                    inputs: vec!["input".to_string()],
                    params: serde_json::json!({"prefix": "/"}),
                },
                GraphNode {
                    id: "output".to_string(),
                    node_type: NodeType::Operation {
                        op: "Identity".to_string(),
                    },
                    inputs: vec!["check_command".to_string()],
                    params: serde_json::json!({}),
                },
            ],
            outputs: vec!["output".to_string()],
            entry_point: "input".to_string(),
            metadata: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_execute_simple_graph() {
        let interp = GraphInterpreter::default();
        let graph = create_test_graph();
        
        let mut inputs = HashMap::new();
        inputs.insert("message".to_string(), Value::String("/help".to_string()));
        
        let result = interp.execute(&graph, inputs).await.unwrap();
        
        assert!(result.outputs.contains_key("output"));
        assert_eq!(result.outputs.get("output"), Some(&Value::Bool(true)));
        assert_eq!(result.trace.len(), 3);
    }

    #[tokio::test]
    async fn test_execute_with_false_condition() {
        let interp = GraphInterpreter::default();
        let graph = create_test_graph();
        
        let mut inputs = HashMap::new();
        inputs.insert("message".to_string(), Value::String("hello".to_string()));
        
        let result = interp.execute(&graph, inputs).await.unwrap();
        
        assert_eq!(result.outputs.get("output"), Some(&Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_lookup_node() {
        let interp = GraphInterpreter::default();
        
        let mut table = HashMap::new();
        table.insert("/help".to_string(), "skill:help".to_string());
        table.insert("/status".to_string(), "skill:status".to_string());
        
        let graph = Graph {
            name: "lookup_test".to_string(),
            version: 1,
            description: "".to_string(),
            nodes: vec![
                GraphNode {
                    id: "input".to_string(),
                    node_type: NodeType::External {
                        uri: "input://command".to_string(),
                    },
                    inputs: vec![],
                    params: serde_json::json!({}),
                },
                GraphNode {
                    id: "lookup".to_string(),
                    node_type: NodeType::Lookup {
                        table,
                        default: Some("skill:unknown".to_string()),
                    },
                    inputs: vec!["input".to_string()],
                    params: serde_json::json!({}),
                },
            ],
            outputs: vec!["lookup".to_string()],
            entry_point: "input".to_string(),
            metadata: serde_json::json!({}),
        };
        
        let mut inputs = HashMap::new();
        inputs.insert("command".to_string(), Value::String("/help".to_string()));
        
        let result = interp.execute(&graph, inputs).await.unwrap();
        assert_eq!(
            result.outputs.get("lookup"),
            Some(&Value::String("skill:help".to_string()))
        );
    }

    #[tokio::test]
    async fn test_state_persistence() {
        let interp = GraphInterpreter::default();
        
        // Save state
        let mut state = HashMap::new();
        state.insert("count".to_string(), Value::Int(42));
        interp.save_state("session1", Value::Map(state)).await;
        
        // Load state
        let loaded = interp.load_state("session1").await;
        if let Value::Map(m) = loaded {
            assert_eq!(m.get("count"), Some(&Value::Int(42)));
        } else {
            panic!("Expected map");
        }
    }

    #[tokio::test]
    async fn test_execution_hash_deterministic() {
        let interp = GraphInterpreter::default();
        let graph = create_test_graph();
        
        let mut inputs = HashMap::new();
        inputs.insert("message".to_string(), Value::String("/help".to_string()));
        
        let result1 = interp.execute(&graph, inputs.clone()).await.unwrap();
        let result2 = interp.execute(&graph, inputs).await.unwrap();
        
        // Same inputs should produce same hash
        assert_eq!(result1.hash, result2.hash);
    }
}
