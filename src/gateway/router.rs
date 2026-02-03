//! Graph-based message routing for 0-openclaw.
//!
//! The router loads a 0-lang graph and executes it to determine which skill
//! should handle each incoming message. All routing logic is defined in the
//! graph file (graphs/core/router.0), not in Rust code.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::types::{ContentHash, IncomingMessage};
use crate::error::GatewayError;
use crate::runtime::{GraphInterpreter, Graph, Value, ExecutionResult};
use super::proof::ExecutionTrace;

/// Route information describing how to handle a message.
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Target skill hash
    pub skill_hash: ContentHash,
    
    /// Confidence in this routing decision
    pub confidence: f32,
    
    /// Name of the matched route
    pub route_name: String,
    
    /// Extracted parameters from the message
    pub params: HashMap<String, String>,
}

/// Graph-based message router.
/// 
/// This router executes a 0-lang graph to make routing decisions.
/// All routing logic is in the graph file, not in Rust code.
pub struct Router {
    /// The routing graph
    graph: Graph,
    
    /// Graph interpreter
    interpreter: Arc<GraphInterpreter>,
    
    /// Default skill hash (fallback)
    default_skill: ContentHash,
    
    /// Cached routes for fast lookup
    route_cache: HashMap<ContentHash, RouteResult>,
    
    /// Whether to use caching
    caching_enabled: bool,
}

impl Router {
    /// Create a new router from a graph file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, GatewayError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| GatewayError::RouterError(format!("Failed to read graph: {}", e)))?;
        Self::from_source(&content)
    }

    /// Create a new router from 0-lang source.
    pub fn from_source(source: &str) -> Result<Self, GatewayError> {
        let graph = crate::runtime::parse_graph(source)?;
        Ok(Self {
            graph,
            interpreter: Arc::new(GraphInterpreter::default()),
            default_skill: ContentHash::from_string("skill:default"),
            route_cache: HashMap::new(),
            caching_enabled: true,
        })
    }

    /// Create a router with a pre-parsed graph.
    pub fn new(graph: Graph) -> Self {
        Self {
            graph,
            interpreter: Arc::new(GraphInterpreter::default()),
            default_skill: ContentHash::from_string("skill:default"),
            route_cache: HashMap::new(),
            caching_enabled: true,
        }
    }

    /// Create a simple router with default commands.
    /// This builds a graph programmatically for quick setup.
    pub fn with_defaults() -> Self {
        let graph = Self::build_default_graph();
        Self::new(graph)
    }

    /// Build a default routing graph programmatically.
    fn build_default_graph() -> Graph {
        use crate::runtime::types::{GraphNode, NodeType, RouteCondition};
        
        Graph {
            name: "default_router".to_string(),
            version: 1,
            description: "Default message router".to_string(),
            nodes: vec![
                // Input nodes
                GraphNode {
                    id: "message".to_string(),
                    node_type: NodeType::External { uri: "input://message".to_string() },
                    inputs: vec![],
                    params: serde_json::json!({}),
                },
                GraphNode {
                    id: "sender".to_string(),
                    node_type: NodeType::External { uri: "input://sender".to_string() },
                    inputs: vec![],
                    params: serde_json::json!({}),
                },
                GraphNode {
                    id: "channel".to_string(),
                    node_type: NodeType::External { uri: "input://channel".to_string() },
                    inputs: vec![],
                    params: serde_json::json!({}),
                },
                
                // Check if message is a command
                GraphNode {
                    id: "is_command".to_string(),
                    node_type: NodeType::Operation { op: "StartsWith".to_string() },
                    inputs: vec!["message".to_string()],
                    params: serde_json::json!({"prefix": "/"}),
                },
                
                // Extract command name
                GraphNode {
                    id: "command_name".to_string(),
                    node_type: NodeType::Operation { op: "ExtractFirstWord".to_string() },
                    inputs: vec!["message".to_string()],
                    params: serde_json::json!({}),
                },
                
                // Command lookup table
                GraphNode {
                    id: "command_lookup".to_string(),
                    node_type: NodeType::Lookup {
                        table: [
                            ("/help".to_string(), "skill:help".to_string()),
                            ("/status".to_string(), "skill:status".to_string()),
                            ("/skills".to_string(), "skill:list".to_string()),
                            ("/search".to_string(), "skill:search".to_string()),
                            ("/remind".to_string(), "skill:reminder".to_string()),
                        ].into_iter().collect(),
                        default: Some("skill:unknown_command".to_string()),
                    },
                    inputs: vec!["command_name".to_string()],
                    params: serde_json::json!({}),
                },
                
                // Intent classification for non-commands
                GraphNode {
                    id: "intent".to_string(),
                    node_type: NodeType::Operation { op: "ClassifyIntent".to_string() },
                    inputs: vec!["message".to_string()],
                    params: serde_json::json!({
                        "classes": ["greeting", "question", "request", "statement", "other"]
                    }),
                },
                
                // Conversation skill lookup
                GraphNode {
                    id: "conversation_skill".to_string(),
                    node_type: NodeType::Lookup {
                        table: [
                            ("greeting".to_string(), "skill:greeting".to_string()),
                            ("question".to_string(), "skill:qa".to_string()),
                            ("request".to_string(), "skill:assistant".to_string()),
                            ("statement".to_string(), "skill:acknowledge".to_string()),
                            ("other".to_string(), "skill:conversation".to_string()),
                        ].into_iter().collect(),
                        default: Some("skill:conversation".to_string()),
                    },
                    inputs: vec!["intent".to_string()],
                    params: serde_json::json!({}),
                },
                
                // Route decision
                GraphNode {
                    id: "route_decision".to_string(),
                    node_type: NodeType::Route {
                        conditions: vec![
                            RouteCondition {
                                input: "is_command".to_string(),
                                match_value: None,
                                threshold: 0.9,
                                target: "command_lookup".to_string(),
                                confidence: 0.95,
                            },
                            RouteCondition {
                                input: "default".to_string(),
                                match_value: None,
                                threshold: 0.0,
                                target: "conversation_skill".to_string(),
                                confidence: 0.7,
                            },
                        ],
                    },
                    inputs: vec!["is_command".to_string(), "command_lookup".to_string(), "conversation_skill".to_string()],
                    params: serde_json::json!({}),
                },
                
                // Extract parameters
                GraphNode {
                    id: "params".to_string(),
                    node_type: NodeType::Operation { op: "ExtractParams".to_string() },
                    inputs: vec!["message".to_string()],
                    params: serde_json::json!({}),
                },
                
                // Output: skill target
                GraphNode {
                    id: "skill_target".to_string(),
                    node_type: NodeType::Operation { op: "If".to_string() },
                    inputs: vec!["is_command".to_string(), "command_lookup".to_string(), "conversation_skill".to_string()],
                    params: serde_json::json!({}),
                },
            ],
            outputs: vec!["skill_target".to_string(), "params".to_string(), "route_decision".to_string()],
            entry_point: "message".to_string(),
            metadata: serde_json::json!({
                "author": "0-openclaw",
                "version": "1.0"
            }),
        }
    }

    /// Set the default skill.
    pub fn set_default_skill(&mut self, skill_hash: ContentHash) {
        self.default_skill = skill_hash;
    }

    /// Enable or disable caching.
    pub fn set_caching(&mut self, enabled: bool) {
        self.caching_enabled = enabled;
        if !enabled {
            self.route_cache.clear();
        }
    }

    /// Route a message to a skill by executing the routing graph.
    pub async fn route(
        &mut self,
        message: &IncomingMessage,
    ) -> Result<(RouteResult, ExecutionTrace), GatewayError> {
        // Check cache first
        if self.caching_enabled {
            let cache_key = Self::cache_key(message);
            if let Some(cached) = self.route_cache.get(&cache_key) {
                return Ok((cached.clone(), ExecutionTrace::cached()));
            }
        }

        // Build graph inputs
        let mut inputs = HashMap::new();
        inputs.insert("message".to_string(), Value::String(message.content.clone()));
        inputs.insert("sender".to_string(), Value::String(message.sender_id.clone()));
        inputs.insert("channel".to_string(), Value::String(message.channel_id.clone()));

        // Execute the routing graph
        let exec_result = self.interpreter.execute(&self.graph, inputs).await?;

        // Extract routing result from graph outputs
        let result = self.extract_route_result(&exec_result, message)?;

        // Build execution trace
        let trace = ExecutionTrace::from_graph_execution(&exec_result);

        // Cache the result
        if self.caching_enabled {
            let cache_key = Self::cache_key(message);
            self.route_cache.insert(cache_key, result.clone());
        }

        Ok((result, trace))
    }

    /// Extract RouteResult from graph execution.
    fn extract_route_result(
        &self,
        exec_result: &ExecutionResult,
        message: &IncomingMessage,
    ) -> Result<RouteResult, GatewayError> {
        // Get skill target from outputs
        let skill_hash = exec_result.outputs.get("skill_target")
            .and_then(|v| v.as_string())
            .map(|s| ContentHash::from_string(s))
            .unwrap_or_else(|| self.default_skill);

        // Get route decision info
        let route_info = exec_result.outputs.get("route_decision")
            .and_then(|v| v.as_map());

        let (confidence, route_name) = if let Some(info) = route_info {
            let conf = info.get("confidence")
                .and_then(|v| v.as_float())
                .unwrap_or(0.5) as f32;
            let name = info.get("matched_input")
                .and_then(|v| v.as_string())
                .unwrap_or("default")
                .to_string();
            (conf, name)
        } else {
            (exec_result.confidence as f32, "graph_route".to_string())
        };

        // Get params from outputs
        let params = exec_result.outputs.get("params")
            .and_then(|v| {
                if let Value::Array(arr) = v {
                    let mut map = HashMap::new();
                    for (i, param) in arr.iter().enumerate() {
                        if let Some(s) = param.as_string() {
                            map.insert(format!("arg{}", i), s.to_string());
                        }
                    }
                    if !map.is_empty() {
                        // Also add combined args
                        let args: Vec<&str> = arr.iter()
                            .filter_map(|v| v.as_string())
                            .collect();
                        map.insert("args".to_string(), args.join(" "));
                    }
                    Some(map)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        Ok(RouteResult {
            skill_hash,
            confidence,
            route_name,
            params,
        })
    }

    /// Generate a cache key for a message.
    fn cache_key(message: &IncomingMessage) -> ContentHash {
        if message.content.starts_with('/') {
            let command = message.content.split_whitespace().next().unwrap_or("");
            ContentHash::from_string(command)
        } else {
            ContentHash::from_bytes(format!("nocache:{}", message.id.to_hex()).as_bytes())
        }
    }

    /// Clear the route cache.
    pub fn clear_cache(&mut self) {
        self.route_cache.clear();
    }

    /// Get the routing graph.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Get the cache size.
    pub fn cache_size(&self) -> usize {
        self.route_cache.len()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_message(content: &str) -> IncomingMessage {
        IncomingMessage::new("test", "user", content)
    }

    #[tokio::test]
    async fn test_router_command_routing() {
        let mut router = Router::with_defaults();
        
        let (result, trace) = router.route(&test_message("/help")).await.unwrap();
        
        assert_eq!(result.skill_hash, ContentHash::from_string("skill:help"));
        assert!(!trace.cached);
    }

    #[tokio::test]
    async fn test_router_conversation_routing() {
        let mut router = Router::with_defaults();
        
        let (result, _trace) = router.route(&test_message("hello there")).await.unwrap();
        
        // Should route to greeting skill based on intent classification
        // The result should have a valid skill hash
        assert!(result.confidence > 0.0);
        assert!(!result.skill_hash.is_zero());
    }

    #[tokio::test]
    async fn test_router_caching() {
        let mut router = Router::with_defaults();
        
        // First call
        let (_, trace1) = router.route(&test_message("/help")).await.unwrap();
        assert!(!trace1.cached);
        
        // Second call should be cached
        let (_, trace2) = router.route(&test_message("/help")).await.unwrap();
        assert!(trace2.cached);
        
        // Clear cache
        router.clear_cache();
        let (_, trace3) = router.route(&test_message("/help")).await.unwrap();
        assert!(!trace3.cached);
    }

    #[tokio::test]
    async fn test_router_param_extraction() {
        let mut router = Router::with_defaults();
        
        let (result, _) = router.route(&test_message("/search rust async")).await.unwrap();
        
        // Should extract parameters
        assert!(result.params.contains_key("args") || result.params.contains_key("arg0"));
    }

    #[tokio::test]
    async fn test_router_default_skill() {
        let mut router = Router::with_defaults();
        let custom_default = ContentHash::from_string("skill:custom_default");
        router.set_default_skill(custom_default);
        
        // Unknown command should fallback appropriately
        let (result, _) = router.route(&test_message("/unknowncommand123")).await.unwrap();
        // Should route to unknown_command skill from lookup
        assert!(!result.skill_hash.is_zero());
        assert!(result.confidence > 0.0);
    }
}
