//! Core types for the 0-lang runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::GatewayError;

/// A value in the 0-lang runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    /// Content hash (32 bytes)
    Hash([u8; 32]),
    /// Confidence score (0.0-1.0)
    Confidence(f64),
}

impl Value {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::Confidence(c) => Some(*c),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Bytes(b) => !b.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Hash(_) => true,
            Value::Confidence(c) => *c > 0.0,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

/// Type of a graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeType {
    /// External input node
    External { uri: String },
    /// Operation node (calls a builtin or custom op)
    Operation { op: String },
    /// Lookup table node
    Lookup { table: HashMap<String, String>, default: Option<String> },
    /// Routing decision node
    Route { conditions: Vec<RouteCondition> },
    /// Permission check node
    Permission { action: String, min_confidence: f64 },
    /// Constant value node
    Constant { value: Value },
}

/// A condition for routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCondition {
    pub input: String,
    #[serde(default)]
    pub match_value: Option<String>,
    #[serde(default)]
    pub threshold: f64,
    pub target: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

/// A node in a 0-lang graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    #[serde(flatten)]
    pub node_type: NodeType,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// An edge in the graph (implicit from inputs).
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub field: Option<String>,
}

/// A 0-lang graph definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub description: String,
    pub nodes: Vec<GraphNode>,
    pub outputs: Vec<String>,
    #[serde(default)]
    pub entry_point: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

fn default_version() -> u32 {
    1
}

impl Graph {
    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all edges in the graph.
    pub fn edges(&self) -> Vec<Edge> {
        let mut edges = Vec::new();
        for node in &self.nodes {
            for input in &node.inputs {
                // Handle field references like "node.field"
                let (from_node, field) = if input.contains('.') {
                    let parts: Vec<&str> = input.splitn(2, '.').collect();
                    (parts[0].to_string(), Some(parts[1].to_string()))
                } else {
                    (input.clone(), None)
                };
                edges.push(Edge {
                    from: from_node,
                    to: node.id.clone(),
                    field,
                });
            }
        }
        edges
    }

    /// Topologically sort nodes for execution order.
    pub fn topo_sort(&self) -> Result<Vec<&GraphNode>, GatewayError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize
        for node in &self.nodes {
            in_degree.entry(&node.id).or_insert(0);
            adj.entry(&node.id).or_insert_with(Vec::new);
        }

        // Build adjacency and in-degree
        for node in &self.nodes {
            for input in &node.inputs {
                let from_node = input.split('.').next().unwrap();
                if let Some(degree) = in_degree.get_mut(node.id.as_str()) {
                    *degree += 1;
                }
                adj.entry(from_node).or_insert_with(Vec::new).push(&node.id);
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut result = Vec::new();

        while let Some(node_id) = queue.pop() {
            if let Some(node) = self.get_node(node_id) {
                result.push(node);
            }
            if let Some(neighbors) = adj.get(node_id) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err(GatewayError::ConfigError("Cycle detected in graph".to_string()));
        }

        Ok(result)
    }
}

/// Parse a graph from 0-lang source.
pub fn parse_graph_from_source(source: &str) -> Result<Graph, GatewayError> {
    // Simple parser for 0-lang graph format
    // This is a basic implementation that handles the JSON-like format
    
    let mut cleaned = String::new();
    let mut in_comment = false;
    
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue; // Skip comment lines
        }
        // Remove inline comments
        let line_without_comment = if let Some(idx) = line.find('#') {
            &line[..idx]
        } else {
            line
        };
        cleaned.push_str(line_without_comment);
        cleaned.push('\n');
    }

    // Find the Graph { ... } block
    let graph_start = cleaned.find("Graph").ok_or_else(|| {
        GatewayError::ConfigError("No Graph definition found".to_string())
    })?;
    
    let brace_start = cleaned[graph_start..].find('{').ok_or_else(|| {
        GatewayError::ConfigError("No opening brace found".to_string())
    })? + graph_start;

    // Find matching closing brace
    let mut depth = 0;
    let mut brace_end = brace_start;
    for (i, c) in cleaned[brace_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    brace_end = brace_start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    let graph_content = &cleaned[brace_start..brace_end];
    
    // Convert to JSON-compatible format
    let json_content = convert_to_json(graph_content)?;
    
    // Parse as JSON
    let graph: Graph = serde_json::from_str(&json_content)
        .map_err(|e| GatewayError::ConfigError(format!("Failed to parse graph: {}", e)))?;
    
    Ok(graph)
}

/// Convert 0-lang format to JSON.
fn convert_to_json(source: &str) -> Result<String, GatewayError> {
    let mut result = source.to_string();
    
    // Replace unquoted keys with quoted keys
    // This is a simplified conversion
    let key_pattern = regex::Regex::new(r"(\s*)(\w+)(\s*):").unwrap();
    result = key_pattern.replace_all(&result, r#"$1"$2"$3:"#).to_string();
    
    // Handle trailing commas (remove them)
    let trailing_comma = regex::Regex::new(r",(\s*[}\]])").unwrap();
    result = trailing_comma.replace_all(&result, "$1").to_string();
    
    // Handle unquoted string values for known fields
    // This is simplified - a full parser would be more robust
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversions() {
        let v: Value = "hello".into();
        assert_eq!(v.as_string(), Some("hello"));

        let v: Value = true.into();
        assert_eq!(v.as_bool(), Some(true));

        let v: Value = 42i64.into();
        assert_eq!(v.as_int(), Some(42));
    }

    #[test]
    fn test_value_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
    }

    #[test]
    fn test_graph_topo_sort() {
        let graph = Graph {
            name: "test".to_string(),
            version: 1,
            description: "".to_string(),
            nodes: vec![
                GraphNode {
                    id: "a".to_string(),
                    node_type: NodeType::External { uri: "input://a".to_string() },
                    inputs: vec![],
                    params: serde_json::json!({}),
                },
                GraphNode {
                    id: "b".to_string(),
                    node_type: NodeType::Operation { op: "Identity".to_string() },
                    inputs: vec!["a".to_string()],
                    params: serde_json::json!({}),
                },
                GraphNode {
                    id: "c".to_string(),
                    node_type: NodeType::Operation { op: "Identity".to_string() },
                    inputs: vec!["b".to_string()],
                    params: serde_json::json!({}),
                },
            ],
            outputs: vec!["c".to_string()],
            entry_point: "a".to_string(),
            metadata: serde_json::json!({}),
        };

        let sorted = graph.topo_sort().unwrap();
        assert_eq!(sorted.len(), 3);
        // a should come before b, b should come before c
        let ids: Vec<&str> = sorted.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.iter().position(|&x| x == "a") < ids.iter().position(|&x| x == "b"));
        assert!(ids.iter().position(|&x| x == "b") < ids.iter().position(|&x| x == "c"));
    }
}
