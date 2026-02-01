//! Skill Loader - load skills from files and network.
//!
//! The SkillLoader provides functionality to load skill graphs from
//! various sources including local files and remote URLs.

use std::path::{Path, PathBuf};
use crate::error::SkillError;
use super::graph::{SkillGraph, SkillNode};
use super::verifier::SkillVerifier;

/// Loader for skill graphs from various sources.
pub struct SkillLoader {
    /// Base directory for skill files.
    base_dir: PathBuf,
    /// Whether to verify skills on load.
    verify_on_load: bool,
    /// Cache of loaded skills.
    cache: std::collections::HashMap<PathBuf, SkillGraph>,
}

impl SkillLoader {
    /// Create a new skill loader.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            verify_on_load: true,
            cache: std::collections::HashMap::new(),
        }
    }

    /// Set whether to verify skills on load.
    pub fn with_verify(mut self, verify: bool) -> Self {
        self.verify_on_load = verify;
        self
    }

    /// Load a skill from a file.
    ///
    /// Supports `.0` (custom format) and `.json` files.
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<SkillGraph, SkillError> {
        let path = self.resolve_path(path.as_ref());
        
        // Check cache
        if let Some(graph) = self.cache.get(&path) {
            return Ok(graph.clone());
        }
        
        // Read file
        let content = std::fs::read_to_string(&path)
            .map_err(|e| SkillError::NotFound(format!("{}: {}", path.display(), e)))?;
        
        // Parse based on extension
        let graph = match path.extension().and_then(|e| e.to_str()) {
            Some("json") => self.parse_json(&content)?,
            Some("0") => self.parse_zero_format(&content)?,
            _ => self.parse_auto(&content)?,
        };
        
        // Verify if enabled
        if self.verify_on_load {
            let result = SkillVerifier::verify(&graph)?;
            if !result.safe {
                return Err(SkillError::VerificationFailed(
                    result.errors.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }
        
        // Cache and return
        self.cache.insert(path, graph.clone());
        Ok(graph)
    }

    /// Load a skill from a URL.
    pub async fn load_url(&mut self, url: &str) -> Result<SkillGraph, SkillError> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| SkillError::NotFound(format!("Failed to fetch {}: {}", url, e)))?;
        
        if !response.status().is_success() {
            return Err(SkillError::NotFound(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }
        
        let content = response.text()
            .await
            .map_err(|e| SkillError::NotFound(format!("Failed to read response: {}", e)))?;
        
        // Parse content
        let graph = self.parse_auto(&content)?;
        
        // Verify
        if self.verify_on_load {
            let result = SkillVerifier::verify(&graph)?;
            if !result.safe {
                return Err(SkillError::VerificationFailed(
                    result.errors.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }
        
        Ok(graph)
    }

    /// Load all skills from a directory.
    pub fn load_directory(&mut self, dir: impl AsRef<Path>) -> Result<Vec<SkillGraph>, SkillError> {
        let dir = self.resolve_path(dir.as_ref());
        
        let mut skills = Vec::new();
        
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| SkillError::NotFound(format!("{}: {}", dir.display(), e)))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| SkillError::NotFound(e.to_string()))?;
            let path = entry.path();
            
            // Skip directories and non-skill files
            if path.is_dir() {
                continue;
            }
            
            match path.extension().and_then(|e| e.to_str()) {
                Some("0") | Some("json") => {
                    match self.load_file(&path) {
                        Ok(graph) => skills.push(graph),
                        Err(e) => {
                            tracing::warn!("Failed to load skill {}: {}", path.display(), e);
                        }
                    }
                }
                _ => continue,
            }
        }
        
        tracing::info!("Loaded {} skills from {}", skills.len(), dir.display());
        Ok(skills)
    }

    /// Resolve a path relative to the base directory.
    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        }
    }

    /// Parse JSON format skill.
    fn parse_json(&self, content: &str) -> Result<SkillGraph, SkillError> {
        serde_json::from_str(content)
            .map_err(|e| SkillError::InvalidGraph(format!("JSON parse error: {}", e)))
    }

    /// Parse .0 format skill (custom format).
    fn parse_zero_format(&self, content: &str) -> Result<SkillGraph, SkillError> {
        let mut name = String::new();
        let mut version = "1".to_string();
        let mut description = None;
        let mut nodes = Vec::new();
        let mut outputs = Vec::new();
        let mut permissions = Vec::new();
        let proofs = Vec::new();
        
        let mut in_graph = false;
        let mut current_section = "";
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse Graph block
            if line.starts_with("Graph {") || line == "Graph{" {
                in_graph = true;
                continue;
            }
            
            if line == "}" && in_graph {
                in_graph = false;
                continue;
            }
            
            if !in_graph {
                continue;
            }
            
            // Parse key-value pairs
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().trim_matches(',');
                
                match key.as_str() {
                    "name" => {
                        name = value.trim_matches('"').to_string();
                    }
                    "version" => {
                        version = value.trim_matches('"').to_string();
                    }
                    "description" => {
                        description = Some(value.trim_matches('"').to_string());
                    }
                    "nodes" => {
                        current_section = "nodes";
                    }
                    "outputs" => {
                        // Parse inline array: ["output"]
                        if value.starts_with('[') {
                            let items: Vec<&str> = value
                                .trim_matches(|c| c == '[' || c == ']')
                                .split(',')
                                .map(|s| s.trim().trim_matches('"'))
                                .filter(|s| !s.is_empty())
                                .collect();
                            outputs = items.iter().map(|s| s.to_string()).collect();
                        } else {
                            current_section = "outputs";
                        }
                    }
                    "permissions" => {
                        if value.starts_with('[') {
                            let items: Vec<&str> = value
                                .trim_matches(|c| c == '[' || c == ']')
                                .split(',')
                                .map(|s| s.trim().trim_matches('"'))
                                .filter(|s| !s.is_empty())
                                .collect();
                            permissions = items.iter().map(|s| s.to_string()).collect();
                        }
                    }
                    "proofs" => {
                        current_section = "proofs";
                    }
                    _ => {}
                }
            }
            
            // Parse node definitions
            if line.starts_with('{') && current_section == "nodes" {
                if let Some(node) = self.parse_node_definition(line) {
                    nodes.push(node);
                }
            }
        }
        
        Ok(SkillGraph {
            name,
            version,
            description,
            nodes,
            entry_point: None,
            outputs,
            permissions,
            proofs,
        })
    }

    /// Parse a single node definition.
    fn parse_node_definition(&self, line: &str) -> Option<SkillNode> {
        use super::graph::Op;
        
        // Extract key-value pairs from the node definition
        let content = line.trim_matches(|c| c == '{' || c == '}' || c == ',');
        
        let mut id = String::new();
        let mut node_type = String::new();
        let mut uri = String::new();
        let mut op = String::new();
        let mut inputs = Vec::new();
        let mut template = String::new();
        let mut path = String::new();
        
        for part in content.split(',') {
            let part = part.trim();
            if let Some(pos) = part.find(':') {
                let key = part[..pos].trim().to_lowercase();
                let value = part[pos + 1..].trim().trim_matches('"');
                
                match key.as_str() {
                    "id" => id = value.to_string(),
                    "type" => node_type = value.to_string(),
                    "uri" => uri = value.to_string(),
                    "op" => op = value.to_string(),
                    "inputs" => {
                        inputs = value
                            .trim_matches(|c| c == '[' || c == ']')
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "template" => template = value.to_string(),
                    "path" => path = value.to_string(),
                    _ => {}
                }
            }
        }
        
        // Create node based on type
        match node_type.as_str() {
            "External" => Some(SkillNode::External { id, uri, inputs }),
            "Operation" => {
                let operation = match op.as_str() {
                    "Identity" => Op::Identity,
                    "StringFormat" => Op::StringFormat { template },
                    "StringConcat" => Op::StringConcat,
                    "JsonParse" => Op::JsonParse,
                    "JsonGet" => Op::JsonGet { path },
                    "JsonStringify" => Op::JsonStringify,
                    "HttpGet" => Op::HttpGet,
                    "HttpPost" => Op::HttpPost,
                    _ => Op::Identity,
                };
                Some(SkillNode::Operation { id, op: operation, inputs })
            }
            "Input" => Some(SkillNode::Input { 
                name: id, 
                tensor_type: "string".to_string() 
            }),
            _ => None,
        }
    }

    /// Auto-detect format and parse.
    fn parse_auto(&self, content: &str) -> Result<SkillGraph, SkillError> {
        // Try JSON first
        if content.trim().starts_with('{') {
            if let Ok(graph) = self.parse_json(content) {
                return Ok(graph);
            }
        }
        
        // Try .0 format
        self.parse_zero_format(content)
    }

    /// Clear the cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the base directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new("graphs/skills")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_loader_new() {
        let loader = SkillLoader::new("/tmp/skills");
        assert_eq!(loader.base_dir(), Path::new("/tmp/skills"));
    }

    #[test]
    fn test_load_json_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        
        let json_content = r#"{
            "name": "test",
            "version": "1",
            "description": "Test skill",
            "nodes": [
                {"Input": {"name": "input", "tensor_type": "string"}}
            ],
            "entry_point": null,
            "outputs": ["input"],
            "permissions": [],
            "proofs": []
        }"#;
        
        std::fs::write(&file_path, json_content).unwrap();
        
        let mut loader = SkillLoader::new(dir.path()).with_verify(false);
        let graph = loader.load_file(&file_path).unwrap();
        
        assert_eq!(graph.name, "test");
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_parse_zero_format() {
        let content = r#"
            # Test skill
            Graph {
                name: "test",
                version: 1,
                description: "A test skill",
                
                nodes: [
                    { id: "input", type: External, uri: "input://message" },
                    { id: "output", type: Operation, op: Identity, inputs: ["input"] },
                ],
                
                outputs: ["output"],
            }
        "#;
        
        let loader = SkillLoader::new("/tmp");
        let graph = loader.parse_zero_format(content).unwrap();
        
        assert_eq!(graph.name, "test");
        assert_eq!(graph.description, Some("A test skill".to_string()));
    }

    #[test]
    fn test_cache() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("cached.json");
        
        let json_content = r#"{
            "name": "cached",
            "version": "1",
            "nodes": [{"Input": {"name": "x", "tensor_type": "string"}}],
            "outputs": ["x"],
            "permissions": [],
            "proofs": []
        }"#;
        
        std::fs::write(&file_path, json_content).unwrap();
        
        let mut loader = SkillLoader::new(dir.path()).with_verify(false);
        
        // First load
        let _ = loader.load_file(&file_path).unwrap();
        assert_eq!(loader.cache.len(), 1);
        
        // Second load (from cache)
        let _ = loader.load_file(&file_path).unwrap();
        assert_eq!(loader.cache.len(), 1);
        
        // Clear cache
        loader.clear_cache();
        assert_eq!(loader.cache.len(), 0);
    }
}
