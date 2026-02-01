//! Skill Graph - the core data structure for skills.
//!
//! A SkillGraph represents a directed acyclic graph (DAG) of operations
//! that define skill behavior. This is a placeholder implementation
//! that will be replaced with zerolang::RuntimeGraph when 0-lang is available.

use serde::{Serialize, Deserialize};
use crate::types::ContentHash;

/// A node in the skill graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillNode {
    /// Input node - receives external data.
    Input {
        name: String,
        tensor_type: String,
    },
    /// Operation node - performs computation.
    Operation {
        id: String,
        op: Op,
        inputs: Vec<String>,
    },
    /// External node - calls external service.
    External {
        id: String,
        uri: String,
        inputs: Vec<String>,
    },
    /// Constant node - holds a constant value.
    Constant {
        id: String,
        value: serde_json::Value,
    },
}

impl SkillNode {
    /// Get the ID of this node.
    pub fn id(&self) -> &str {
        match self {
            SkillNode::Input { name, .. } => name,
            SkillNode::Operation { id, .. } => id,
            SkillNode::External { id, .. } => id,
            SkillNode::Constant { id, .. } => id,
        }
    }

    /// Get the input dependencies of this node.
    pub fn inputs(&self) -> &[String] {
        match self {
            SkillNode::Input { .. } => &[],
            SkillNode::Operation { inputs, .. } => inputs,
            SkillNode::External { inputs, .. } => inputs,
            SkillNode::Constant { .. } => &[],
        }
    }
}

/// Operations available in skill graphs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Op {
    /// Identity - pass through input unchanged.
    Identity,
    /// String formatting with template.
    StringFormat { template: String },
    /// String concatenation.
    StringConcat,
    /// JSON parsing.
    JsonParse,
    /// JSON field extraction.
    JsonGet { path: String },
    /// JSON to string.
    JsonStringify,
    /// Conditional branch.
    Conditional,
    /// Map over array.
    Map { body: Box<SkillGraph> },
    /// Filter array.
    Filter { predicate: Box<SkillGraph> },
    /// Reduce array.
    Reduce { initial: serde_json::Value },
    /// HTTP GET request.
    HttpGet,
    /// HTTP POST request.
    HttpPost,
    /// Wait/delay operation.
    Wait { ms: u64 },
    /// Log operation (for debugging).
    Log { level: String },
}

/// Safety proof attached to a skill graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SafetyProof {
    /// Maximum execution steps before timeout.
    pub max_steps: u64,
    /// Fuel budget for execution.
    pub fuel_budget: u64,
    /// Whether halting is proven.
    pub halting_proven: bool,
    /// Maximum memory usage in bytes.
    pub memory_bound: Option<u64>,
}

impl Default for SafetyProof {
    fn default() -> Self {
        Self {
            max_steps: 1000,
            fuel_budget: 10000,
            halting_proven: false,
            memory_bound: Some(1024 * 1024), // 1MB
        }
    }
}

/// A skill graph - the core execution unit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillGraph {
    /// Unique name for the skill.
    pub name: String,
    /// Schema version.
    pub version: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Nodes in the graph.
    pub nodes: Vec<SkillNode>,
    /// Entry point node ID.
    pub entry_point: Option<String>,
    /// Output node IDs.
    pub outputs: Vec<String>,
    /// Required permissions.
    pub permissions: Vec<String>,
    /// Safety proofs.
    pub proofs: Vec<SafetyProof>,
}

impl SkillGraph {
    /// Create a new skill graph builder.
    pub fn builder(name: &str) -> SkillGraphBuilder {
        SkillGraphBuilder::new(name)
    }

    /// Compute the content hash of this graph.
    pub fn content_hash(&self) -> ContentHash {
        let serialized = serde_json::to_vec(self).unwrap_or_default();
        ContentHash::from_bytes(&serialized)
    }

    /// Check if the graph has an output with the given name.
    pub fn has_output(&self, name: &str) -> bool {
        self.outputs.contains(&name.to_string())
    }

    /// Check if the graph has an input with the given name.
    pub fn has_input(&self, name: &str) -> bool {
        self.nodes.iter().any(|node| {
            matches!(node, SkillNode::Input { name: n, .. } if n == name)
        })
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&SkillNode> {
        self.nodes.iter().find(|n| n.id() == id)
    }

    /// Get all external URIs in the graph.
    pub fn external_uris(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .filter_map(|node| {
                if let SkillNode::External { uri, .. } = node {
                    Some(uri.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if graph has any external calls.
    pub fn has_external_calls(&self) -> bool {
        self.nodes.iter().any(|n| matches!(n, SkillNode::External { .. }))
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Serialize to bytes.
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Builder for SkillGraph.
#[derive(Debug)]
pub struct SkillGraphBuilder {
    name: String,
    version: String,
    description: Option<String>,
    nodes: Vec<SkillNode>,
    entry_point: Option<String>,
    outputs: Vec<String>,
    permissions: Vec<String>,
    proofs: Vec<SafetyProof>,
}

impl SkillGraphBuilder {
    /// Create a new builder with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1".to_string(),
            description: None,
            nodes: Vec::new(),
            entry_point: None,
            outputs: Vec::new(),
            permissions: Vec::new(),
            proofs: Vec::new(),
        }
    }

    /// Set the description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Set the version.
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Add an input node.
    pub fn add_input(mut self, name: &str, tensor_type: &str) -> Self {
        self.nodes.push(SkillNode::Input {
            name: name.to_string(),
            tensor_type: tensor_type.to_string(),
        });
        if self.entry_point.is_none() {
            self.entry_point = Some(name.to_string());
        }
        self
    }

    /// Add an operation node.
    pub fn add_operation(mut self, id: &str, op: Op, inputs: Vec<&str>) -> Self {
        self.nodes.push(SkillNode::Operation {
            id: id.to_string(),
            op,
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Add an external node.
    pub fn add_external(mut self, id: &str, uri: &str, inputs: Vec<&str>) -> Self {
        self.nodes.push(SkillNode::External {
            id: id.to_string(),
            uri: uri.to_string(),
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Add a constant node.
    pub fn add_constant(mut self, id: &str, value: serde_json::Value) -> Self {
        self.nodes.push(SkillNode::Constant {
            id: id.to_string(),
            value,
        });
        self
    }

    /// Add a raw node.
    pub fn add_node(mut self, node: SkillNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Set the entry point.
    pub fn entry_point(mut self, id: &str) -> Self {
        self.entry_point = Some(id.to_string());
        self
    }

    /// Add an output.
    pub fn output(mut self, id: &str) -> Self {
        self.outputs.push(id.to_string());
        self
    }

    /// Add multiple outputs.
    pub fn outputs(mut self, ids: Vec<&str>) -> Self {
        self.outputs.extend(ids.iter().map(|s| s.to_string()));
        self
    }

    /// Add a permission requirement.
    pub fn permission(mut self, perm: &str) -> Self {
        self.permissions.push(perm.to_string());
        self
    }

    /// Add a safety proof.
    pub fn proof(mut self, proof: SafetyProof) -> Self {
        self.proofs.push(proof);
        self
    }

    /// Build the skill graph.
    pub fn build(self) -> SkillGraph {
        let outputs = if self.outputs.is_empty() {
            // Default to last node as output
            self.nodes.last().map(|n| vec![n.id().to_string()]).unwrap_or_default()
        } else {
            self.outputs
        };

        SkillGraph {
            name: self.name,
            version: self.version,
            description: self.description,
            nodes: self.nodes,
            entry_point: self.entry_point,
            outputs,
            permissions: self.permissions,
            proofs: self.proofs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_graph_builder() {
        let graph = SkillGraph::builder("test")
            .description("Test skill")
            .version("1.0.0")
            .add_input("query", "string")
            .add_operation("format", Op::StringFormat { template: "Hello: {}".to_string() }, vec!["query"])
            .output("format")
            .build();

        assert_eq!(graph.name, "test");
        assert_eq!(graph.description, Some("Test skill".to_string()));
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.outputs, vec!["format"]);
    }

    #[test]
    fn test_content_hash_deterministic() {
        let graph1 = SkillGraph::builder("test")
            .description("Same content")
            .add_input("x", "string")
            .build();

        let graph2 = SkillGraph::builder("test")
            .description("Same content")
            .add_input("x", "string")
            .build();

        assert_eq!(graph1.content_hash(), graph2.content_hash());
    }

    #[test]
    fn test_content_hash_different() {
        let graph1 = SkillGraph::builder("test1")
            .description("Content 1")
            .build();

        let graph2 = SkillGraph::builder("test2")
            .description("Content 2")
            .build();

        assert_ne!(graph1.content_hash(), graph2.content_hash());
    }
}
