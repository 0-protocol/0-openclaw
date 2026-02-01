//! Skill Registry - content-addressed storage and retrieval of skills.
//!
//! The SkillRegistry manages installed skills using content-addressed hashing,
//! ensuring that identical skills always produce the same hash regardless of
//! when or where they were created.

use std::collections::HashMap;
use std::path::PathBuf;
use crate::types::ContentHash;
use crate::error::SkillError;
use super::graph::{SkillGraph, SkillNode};
use super::verifier::SkillVerifier;

/// Metadata about a skill.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMetadata {
    /// Skill name (unique identifier).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Semantic version string.
    pub version: String,
    /// Optional author information.
    pub author: Option<String>,
    /// Required permissions (e.g., "network", "filesystem").
    pub permissions: Vec<String>,
    /// Input definitions.
    pub inputs: Vec<SkillInput>,
    /// Output definitions.
    pub outputs: Vec<SkillOutput>,
}

/// Input definition for a skill.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillInput {
    /// Input name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Expected tensor type (e.g., "string", "f32", "i64").
    pub tensor_type: String,
    /// Whether this input is required.
    pub required: bool,
}

/// Output definition for a skill.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillOutput {
    /// Output name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Tensor type produced.
    pub tensor_type: String,
}

impl SkillMetadata {
    /// Create new skill metadata with minimal information.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            version: "1.0.0".to_string(),
            author: None,
            permissions: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Set the version.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Set the author.
    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    /// Add a permission requirement.
    pub fn with_permission(mut self, permission: &str) -> Self {
        self.permissions.push(permission.to_string());
        self
    }

    /// Add an input definition.
    pub fn with_input(mut self, name: &str, description: &str, tensor_type: &str, required: bool) -> Self {
        self.inputs.push(SkillInput {
            name: name.to_string(),
            description: description.to_string(),
            tensor_type: tensor_type.to_string(),
            required,
        });
        self
    }

    /// Add an output definition.
    pub fn with_output(mut self, name: &str, description: &str, tensor_type: &str) -> Self {
        self.outputs.push(SkillOutput {
            name: name.to_string(),
            description: description.to_string(),
            tensor_type: tensor_type.to_string(),
        });
        self
    }
}

/// A registered skill entry.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// Content hash of the skill graph.
    pub hash: ContentHash,
    /// Skill metadata.
    pub metadata: SkillMetadata,
    /// The skill graph itself.
    pub graph: SkillGraph,
    /// Whether the skill has been verified.
    pub verified: bool,
    /// Whether this is a built-in skill.
    pub builtin: bool,
    /// When the skill was installed (Unix timestamp ms).
    pub installed_at: u64,
}

/// Registry for managing skill graphs.
///
/// The SkillRegistry provides content-addressed storage for skills,
/// ensuring deterministic behavior verification.
#[derive(Debug)]
pub struct SkillRegistry {
    /// Installed skills indexed by content hash.
    skills: HashMap<ContentHash, SkillEntry>,
    /// Name to hash mapping for lookup by name.
    name_index: HashMap<String, ContentHash>,
    /// Directory for skill graph files.
    skills_dir: PathBuf,
}

impl SkillRegistry {
    /// Create a new skill registry with the specified skills directory.
    pub fn new(skills_dir: impl Into<PathBuf>) -> Self {
        Self {
            skills: HashMap::new(),
            name_index: HashMap::new(),
            skills_dir: skills_dir.into(),
        }
    }

    /// Load built-in skills into the registry.
    pub fn load_builtin(&mut self) -> Result<(), SkillError> {
        use super::builtin;
        
        // Load echo skill
        let echo = builtin::create_echo_skill();
        self.install_graph("echo", echo, true)?;
        
        // Load search skill
        let search = builtin::create_search_skill();
        self.install_graph("search", search, true)?;
        
        // Load browser skill
        let browser = builtin::create_browser_skill();
        self.install_graph("browser", browser, true)?;
        
        // Load calendar skill
        let calendar = builtin::create_calendar_skill();
        self.install_graph("calendar", calendar, true)?;
        
        tracing::info!("Loaded {} built-in skills", 4);
        Ok(())
    }

    /// Install a skill from a graph.
    ///
    /// # Arguments
    /// * `name` - Unique name for the skill
    /// * `graph` - The skill graph to install
    /// * `builtin` - Whether this is a built-in skill (skips verification)
    ///
    /// # Returns
    /// The content hash of the installed skill.
    pub fn install_graph(
        &mut self,
        name: &str,
        graph: SkillGraph,
        builtin: bool,
    ) -> Result<ContentHash, SkillError> {
        let hash = graph.content_hash();
        
        // Check if already installed
        if self.skills.contains_key(&hash) {
            tracing::debug!("Skill '{}' already installed with hash {:?}", name, hash);
            return Ok(hash);
        }
        
        // Verify skill unless it's built-in
        let verified = if builtin {
            true
        } else {
            let result = SkillVerifier::verify(&graph)?;
            if !result.safe {
                let error_msgs: Vec<String> = result.errors.iter()
                    .map(|e| e.to_string())
                    .collect();
                return Err(SkillError::VerificationFailed(
                    error_msgs.join("; ")
                ));
            }
            true
        };
        
        // Extract metadata
        let metadata = Self::extract_metadata(&graph, name);
        
        let entry = SkillEntry {
            hash,
            metadata,
            graph,
            verified,
            builtin,
            installed_at: chrono::Utc::now().timestamp_millis() as u64,
        };
        
        // Check for name conflicts
        if let Some(existing_hash) = self.name_index.get(name) {
            if *existing_hash != hash {
                return Err(SkillError::AlreadyInstalled(format!(
                    "Skill '{}' already installed with different content",
                    name
                )));
            }
        }
        
        self.skills.insert(hash, entry);
        self.name_index.insert(name.to_string(), hash);
        
        tracing::info!("Installed skill '{}' with hash {:?}", name, hash);
        Ok(hash)
    }

    /// Get a skill by its content hash.
    pub fn get(&self, hash: &ContentHash) -> Option<&SkillEntry> {
        self.skills.get(hash)
    }

    /// Get a skill by name.
    pub fn get_by_name(&self, name: &str) -> Option<&SkillEntry> {
        self.name_index
            .get(name)
            .and_then(|hash| self.skills.get(hash))
    }

    /// List all installed skills.
    pub fn list(&self) -> Vec<&SkillEntry> {
        self.skills.values().collect()
    }

    /// List skills by filter criteria.
    pub fn list_filtered<F>(&self, predicate: F) -> Vec<&SkillEntry>
    where
        F: Fn(&SkillEntry) -> bool,
    {
        self.skills.values().filter(|e| predicate(e)).collect()
    }

    /// List only built-in skills.
    pub fn list_builtin(&self) -> Vec<&SkillEntry> {
        self.list_filtered(|e| e.builtin)
    }

    /// List only custom (non-built-in) skills.
    pub fn list_custom(&self) -> Vec<&SkillEntry> {
        self.list_filtered(|e| !e.builtin)
    }

    /// Check if a skill is installed by hash.
    pub fn is_installed(&self, hash: &ContentHash) -> bool {
        self.skills.contains_key(hash)
    }

    /// Check if a skill is installed by name.
    pub fn is_installed_by_name(&self, name: &str) -> bool {
        self.name_index.contains_key(name)
    }

    /// Get the total number of installed skills.
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Uninstall a skill by hash.
    pub fn uninstall(&mut self, hash: &ContentHash) -> Result<(), SkillError> {
        if let Some(entry) = self.skills.remove(hash) {
            if entry.builtin {
                // Re-insert and return error
                self.skills.insert(*hash, entry);
                return Err(SkillError::VerificationFailed(
                    "Cannot uninstall built-in skills".to_string()
                ));
            }
            self.name_index.remove(&entry.metadata.name);
            tracing::info!("Uninstalled skill '{}'", entry.metadata.name);
            Ok(())
        } else {
            Err(SkillError::NotFound(hash.to_string()))
        }
    }

    /// Uninstall a skill by name.
    pub fn uninstall_by_name(&mut self, name: &str) -> Result<(), SkillError> {
        if let Some(hash) = self.name_index.get(name).copied() {
            self.uninstall(&hash)
        } else {
            Err(SkillError::NotFound(name.to_string()))
        }
    }

    /// Get the skills directory path.
    pub fn skills_dir(&self) -> &PathBuf {
        &self.skills_dir
    }

    /// Extract metadata from a skill graph.
    fn extract_metadata(graph: &SkillGraph, name: &str) -> SkillMetadata {
        let mut metadata = SkillMetadata::new(
            name,
            graph.description.as_deref().unwrap_or("No description"),
        );
        
        metadata.version = graph.version.clone();
        
        // Extract inputs from graph
        for node in &graph.nodes {
            if let SkillNode::Input { name, tensor_type } = node {
                metadata.inputs.push(SkillInput {
                    name: name.clone(),
                    description: String::new(),
                    tensor_type: tensor_type.clone(),
                    required: true,
                });
            }
        }
        
        // Extract outputs
        for output_name in &graph.outputs {
            metadata.outputs.push(SkillOutput {
                name: output_name.clone(),
                description: String::new(),
                tensor_type: "any".to_string(),
            });
        }
        
        // Extract permissions
        metadata.permissions = graph.permissions.clone();
        
        metadata
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new("graphs/skills")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = SkillRegistry::new("/tmp/skills");
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_skill_metadata() {
        let metadata = SkillMetadata::new("test", "A test skill")
            .with_version("2.0.0")
            .with_author("Test Author")
            .with_permission("network")
            .with_input("query", "The search query", "string", true)
            .with_output("result", "The search result", "string");

        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.version, "2.0.0");
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.permissions.len(), 1);
        assert_eq!(metadata.inputs.len(), 1);
        assert_eq!(metadata.outputs.len(), 1);
    }

    #[test]
    fn test_install_and_get() {
        let mut registry = SkillRegistry::new("/tmp/skills");
        
        let graph = SkillGraph::builder("test")
            .description("Test skill")
            .build();
        
        let hash = registry.install_graph("test", graph, true).unwrap();
        
        assert!(registry.is_installed(&hash));
        assert!(registry.is_installed_by_name("test"));
        
        let entry = registry.get(&hash).unwrap();
        assert_eq!(entry.metadata.name, "test");
        
        let entry_by_name = registry.get_by_name("test").unwrap();
        assert_eq!(entry_by_name.hash, hash);
    }

    #[test]
    fn test_content_addressed() {
        let mut registry = SkillRegistry::new("/tmp/skills");
        
        // Same content should produce same hash
        let graph1 = SkillGraph::builder("same")
            .description("Same content")
            .build();
        let graph2 = SkillGraph::builder("same")
            .description("Same content")
            .build();
        
        let hash1 = registry.install_graph("same1", graph1, true).unwrap();
        
        // Installing same content again should return same hash
        let hash2 = registry.install_graph("same1", graph2, true).unwrap();
        
        assert_eq!(hash1, hash2);
        assert_eq!(registry.count(), 1);
    }
}
