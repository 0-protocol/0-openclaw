//! Skill Composer - compose multiple skills into workflows.
//!
//! The SkillComposer allows combining multiple skills into a single
//! unified graph by connecting outputs of one skill to inputs of another.

use std::collections::{HashMap, HashSet};
use crate::types::ContentHash;
use crate::error::SkillError;
use super::graph::{SkillGraph, SkillNode, Op, SafetyProof};

/// A connection between two skills.
#[derive(Debug, Clone)]
pub struct SkillConnection {
    /// Source skill content hash.
    pub from_skill: ContentHash,
    /// Source output name.
    pub from_output: String,
    /// Target skill content hash.
    pub to_skill: ContentHash,
    /// Target input name.
    pub to_input: String,
}

/// Result of skill composition.
#[derive(Debug)]
pub struct ComposedSkill {
    /// The unified graph.
    pub graph: SkillGraph,
    /// Source skill hashes.
    pub source_skills: Vec<ContentHash>,
    /// Content hash of the composed graph.
    pub composition_hash: ContentHash,
}

/// Error types for composition.
#[derive(Debug, thiserror::Error)]
pub enum ComposerError {
    #[error("Skill not found: {0}")]
    SkillNotFound(ContentHash),
    
    #[error("Output '{1}' not found in skill {0}")]
    OutputNotFound(ContentHash, String),
    
    #[error("Input '{1}' not found in skill {0}")]
    InputNotFound(ContentHash, String),
    
    #[error("Cycle detected in skill composition")]
    CycleDetected,
    
    #[error("Graph build error: {0}")]
    GraphBuildError(String),
    
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },
    
    #[error("Empty composition - no skills added")]
    EmptyComposition,
}

impl From<ComposerError> for SkillError {
    fn from(e: ComposerError) -> Self {
        SkillError::CompositionError(e.to_string())
    }
}

/// Compose multiple skills into a workflow.
#[derive(Debug, Default)]
pub struct SkillComposer {
    /// Source skill graphs indexed by hash.
    skills: HashMap<ContentHash, SkillGraph>,
    /// Connection definitions.
    connections: Vec<SkillConnection>,
}

impl SkillComposer {
    /// Create a new composer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a skill to the composition.
    ///
    /// Returns the content hash of the added skill.
    pub fn add_skill(&mut self, graph: SkillGraph) -> ContentHash {
        let hash = graph.content_hash();
        self.skills.insert(hash, graph);
        hash
    }

    /// Connect two skills.
    ///
    /// # Arguments
    /// * `from_skill` - Hash of the source skill
    /// * `from_output` - Name of the output in the source skill
    /// * `to_skill` - Hash of the target skill
    /// * `to_input` - Name of the input in the target skill
    pub fn connect(
        &mut self,
        from_skill: ContentHash,
        from_output: &str,
        to_skill: ContentHash,
        to_input: &str,
    ) -> &mut Self {
        self.connections.push(SkillConnection {
            from_skill,
            from_output: from_output.to_string(),
            to_skill,
            to_input: to_input.to_string(),
        });
        self
    }

    /// Compose all skills into a single unified graph.
    pub fn compose(&self, name: &str) -> Result<ComposedSkill, ComposerError> {
        if self.skills.is_empty() {
            return Err(ComposerError::EmptyComposition);
        }

        // Validate all connections
        self.validate_connections()?;
        
        // Check for cycles
        self.detect_cycles()?;
        
        // Create unified graph
        let mut builder = SkillGraph::builder(name)
            .description(&format!("Composed from {} skills", self.skills.len()));
        
        // Track node ID mappings (original -> new)
        let mut node_mapping: HashMap<(ContentHash, String), String> = HashMap::new();
        
        // Add nodes from each skill with prefixed IDs
        for (skill_hash, skill) in &self.skills {
            let prefix = &skill_hash.to_hex()[..8];
            
            for node in &skill.nodes {
                let new_id = format!("{}_{}", prefix, node.id());
                node_mapping.insert((*skill_hash, node.id().to_string()), new_id.clone());
                
                // Remap node inputs
                let remapped_node = self.remap_node(node, skill_hash, &node_mapping);
                builder = builder.add_node(remapped_node);
            }
            
            // Collect permissions
            for perm in &skill.permissions {
                builder = builder.permission(perm);
            }
        }
        
        // Add bridge nodes for connections
        for (i, conn) in self.connections.iter().enumerate() {
            let from_id = node_mapping
                .get(&(conn.from_skill, conn.from_output.clone()))
                .cloned()
                .unwrap_or_else(|| format!("{}_{}", &conn.from_skill.to_hex()[..8], conn.from_output));
            
            let bridge_id = format!("bridge_{}", i);
            
            builder = builder.add_operation(
                &bridge_id,
                Op::Identity,
                vec![&from_id],
            );
            
            // Update mapping so target skill's input references the bridge
            let _target_input_id = format!("{}_{}", &conn.to_skill.to_hex()[..8], conn.to_input);
            node_mapping.insert((conn.to_skill, conn.to_input.clone()), bridge_id);
        }
        
        // Determine outputs (from skills that have no outgoing connections)
        let _source_hashes: HashSet<ContentHash> = self.connections
            .iter()
            .map(|c| c.from_skill)
            .collect();
        
        for (skill_hash, skill) in &self.skills {
            if !self.connections.iter().any(|c| c.from_skill == *skill_hash) {
                // This skill's outputs become composed outputs
                for output in &skill.outputs {
                    let mapped_id = node_mapping
                        .get(&(*skill_hash, output.clone()))
                        .cloned()
                        .unwrap_or_else(|| format!("{}_{}", &skill_hash.to_hex()[..8], output));
                    builder = builder.output(&mapped_id);
                }
            }
        }
        
        // Add combined safety proof
        let combined_proof = self.combine_proofs();
        builder = builder.proof(combined_proof);
        
        let graph = builder.build();
        let composition_hash = graph.content_hash();
        let source_skills: Vec<ContentHash> = self.skills.keys().copied().collect();
        
        Ok(ComposedSkill {
            graph,
            source_skills,
            composition_hash,
        })
    }

    /// Validate all connections are valid.
    fn validate_connections(&self) -> Result<(), ComposerError> {
        for conn in &self.connections {
            // Check source skill exists
            let from_skill = self.skills
                .get(&conn.from_skill)
                .ok_or(ComposerError::SkillNotFound(conn.from_skill))?;
            
            // Check source has the output
            if !from_skill.has_output(&conn.from_output) {
                // Check if it's a node ID that produces output
                if from_skill.get_node(&conn.from_output).is_none() {
                    return Err(ComposerError::OutputNotFound(
                        conn.from_skill,
                        conn.from_output.clone(),
                    ));
                }
            }
            
            // Check target skill exists
            let to_skill = self.skills
                .get(&conn.to_skill)
                .ok_or(ComposerError::SkillNotFound(conn.to_skill))?;
            
            // Check target has the input
            if !to_skill.has_input(&conn.to_input) {
                return Err(ComposerError::InputNotFound(
                    conn.to_skill,
                    conn.to_input.clone(),
                ));
            }
        }
        
        Ok(())
    }

    /// Detect cycles in the skill graph using topological sort.
    fn detect_cycles(&self) -> Result<(), ComposerError> {
        // Build adjacency list
        let mut adj: HashMap<ContentHash, Vec<ContentHash>> = HashMap::new();
        let mut in_degree: HashMap<ContentHash, usize> = HashMap::new();
        
        for hash in self.skills.keys() {
            adj.insert(*hash, Vec::new());
            in_degree.insert(*hash, 0);
        }
        
        for conn in &self.connections {
            adj.get_mut(&conn.from_skill)
                .map(|v| v.push(conn.to_skill));
            *in_degree.entry(conn.to_skill).or_insert(0) += 1;
        }
        
        // Kahn's algorithm
        let mut queue: Vec<ContentHash> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&h, _)| h)
            .collect();
        
        let mut processed = 0;
        
        while let Some(hash) = queue.pop() {
            processed += 1;
            
            if let Some(neighbors) = adj.get(&hash) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(&neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }
        
        if processed != self.skills.len() {
            Err(ComposerError::CycleDetected)
        } else {
            Ok(())
        }
    }

    /// Remap a node's inputs to use the new ID scheme.
    fn remap_node(
        &self,
        node: &SkillNode,
        skill_hash: &ContentHash,
        mapping: &HashMap<(ContentHash, String), String>,
    ) -> SkillNode {
        let prefix = &skill_hash.to_hex()[..8];
        
        match node {
            SkillNode::Input { name, tensor_type } => SkillNode::Input {
                name: format!("{}_{}", prefix, name),
                tensor_type: tensor_type.clone(),
            },
            SkillNode::Operation { id, op, inputs } => {
                let new_inputs: Vec<String> = inputs
                    .iter()
                    .map(|input| {
                        mapping
                            .get(&(*skill_hash, input.clone()))
                            .cloned()
                            .unwrap_or_else(|| format!("{}_{}", prefix, input))
                    })
                    .collect();
                
                SkillNode::Operation {
                    id: format!("{}_{}", prefix, id),
                    op: op.clone(),
                    inputs: new_inputs,
                }
            }
            SkillNode::External { id, uri, inputs } => {
                let new_inputs: Vec<String> = inputs
                    .iter()
                    .map(|input| {
                        mapping
                            .get(&(*skill_hash, input.clone()))
                            .cloned()
                            .unwrap_or_else(|| format!("{}_{}", prefix, input))
                    })
                    .collect();
                
                SkillNode::External {
                    id: format!("{}_{}", prefix, id),
                    uri: uri.clone(),
                    inputs: new_inputs,
                }
            }
            SkillNode::Constant { id, value } => SkillNode::Constant {
                id: format!("{}_{}", prefix, id),
                value: value.clone(),
            },
        }
    }

    /// Combine safety proofs from all skills.
    fn combine_proofs(&self) -> SafetyProof {
        let mut total_steps = 0u64;
        let mut total_fuel = 0u64;
        let mut total_memory = 0u64;
        let mut all_halting_proven = true;
        
        for skill in self.skills.values() {
            for proof in &skill.proofs {
                total_steps = total_steps.saturating_add(proof.max_steps);
                total_fuel = total_fuel.saturating_add(proof.fuel_budget);
                if let Some(mem) = proof.memory_bound {
                    total_memory = total_memory.saturating_add(mem);
                }
                all_halting_proven = all_halting_proven && proof.halting_proven;
            }
        }
        
        SafetyProof {
            max_steps: total_steps.max(1000),
            fuel_budget: total_fuel.max(10000),
            halting_proven: all_halting_proven,
            memory_bound: Some(total_memory.max(1024 * 1024)),
        }
    }

    /// Get the number of skills in the composition.
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    /// Get the number of connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Clear all skills and connections.
    pub fn clear(&mut self) {
        self.skills.clear();
        self.connections.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composer_new() {
        let composer = SkillComposer::new();
        assert_eq!(composer.skill_count(), 0);
        assert_eq!(composer.connection_count(), 0);
    }

    #[test]
    fn test_add_skill() {
        let mut composer = SkillComposer::new();
        
        let skill1 = SkillGraph::builder("skill1")
            .add_input("x", "string")
            .build();
        
        let hash1 = composer.add_skill(skill1);
        
        assert_eq!(composer.skill_count(), 1);
        assert!(composer.skills.contains_key(&hash1));
    }

    #[test]
    fn test_compose_single_skill() {
        let mut composer = SkillComposer::new();
        
        let skill = SkillGraph::builder("single")
            .description("Single skill")
            .add_input("input", "string")
            .add_operation("output", Op::Identity, vec!["input"])
            .output("output")
            .build();
        
        composer.add_skill(skill);
        
        let composed = composer.compose("composed").unwrap();
        assert_eq!(composed.source_skills.len(), 1);
    }

    #[test]
    fn test_compose_with_connection() {
        let mut composer = SkillComposer::new();
        
        let skill1 = SkillGraph::builder("skill1")
            .add_input("input", "string")
            .add_operation("output", Op::StringFormat { template: "Hello: {}".to_string() }, vec!["input"])
            .output("output")
            .build();
        
        let skill2 = SkillGraph::builder("skill2")
            .add_input("input", "string")
            .add_operation("output", Op::Identity, vec!["input"])
            .output("output")
            .build();
        
        let hash1 = composer.add_skill(skill1);
        let hash2 = composer.add_skill(skill2);
        
        composer.connect(hash1, "output", hash2, "input");
        
        let composed = composer.compose("pipeline").unwrap();
        assert_eq!(composed.source_skills.len(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut composer = SkillComposer::new();
        
        let skill1 = SkillGraph::builder("skill1")
            .add_input("input", "string")
            .output("input")
            .build();
        
        let skill2 = SkillGraph::builder("skill2")
            .add_input("input", "string")
            .output("input")
            .build();
        
        let hash1 = composer.add_skill(skill1);
        let hash2 = composer.add_skill(skill2);
        
        // Create a cycle
        composer.connect(hash1, "input", hash2, "input");
        composer.connect(hash2, "input", hash1, "input");
        
        let result = composer.compose("cyclic");
        assert!(matches!(result, Err(ComposerError::CycleDetected)));
    }
}
