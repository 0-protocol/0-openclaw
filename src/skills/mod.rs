//! Skills platform for 0-openclaw.
//!
//! Skills are verified graph modules that provide specific capabilities
//! to the assistant. They can be composed, verified, and shared.
//!
//! ## Skill Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  Skill Registry                      │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
//! │  │  Built-in   │  │   Custom    │  │  Composed   │  │
//! │  │   Skills    │  │   Skills    │  │   Skills    │  │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │
//! │         └────────────────┼────────────────┘          │
//! │                          ↓                           │
//! │              ┌─────────────────────┐                │
//! │              │    Skill Verifier   │                │
//! │              └─────────────────────┘                │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Implementation Status
//!
//! This module is implemented by **Agent #9**.
//!
//! See: `AGENT-9-0OPENCLAW-SKILLS.md`

use std::collections::HashMap;
use crate::types::ContentHash;
use crate::error::SkillError;

// Submodules to be implemented by Agent #9
// pub mod registry;
// pub mod composer;
// pub mod verifier;
// pub mod builtin;

/// Metadata about a skill.
#[derive(Debug, Clone)]
pub struct SkillMetadata {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Skill version
    pub version: String,
    /// Skill author
    pub author: Option<String>,
    /// Required permissions
    pub permissions: Vec<String>,
}

impl SkillMetadata {
    /// Create new skill metadata.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            version: "1.0.0".to_string(),
            author: None,
            permissions: Vec::new(),
        }
    }
}

/// A registered skill entry.
#[derive(Debug)]
pub struct SkillEntry {
    /// Content hash of the skill graph
    pub hash: ContentHash,
    /// Skill metadata
    pub metadata: SkillMetadata,
    /// Whether the skill has been verified
    pub verified: bool,
    /// When the skill was installed (Unix timestamp)
    pub installed_at: u64,
}

/// Registry for managing skills.
pub struct SkillRegistry {
    /// Installed skills by hash
    skills: HashMap<ContentHash, SkillEntry>,
    /// Name to hash mapping
    name_index: HashMap<String, ContentHash>,
}

impl SkillRegistry {
    /// Create a new skill registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    /// Get a skill by hash.
    pub fn get(&self, hash: &ContentHash) -> Option<&SkillEntry> {
        self.skills.get(hash)
    }

    /// Get a skill by name.
    pub fn get_by_name(&self, name: &str) -> Option<&SkillEntry> {
        self.name_index.get(name).and_then(|h| self.skills.get(h))
    }

    /// List all installed skills.
    pub fn list(&self) -> Vec<&SkillEntry> {
        self.skills.values().collect()
    }

    /// Check if a skill is installed.
    pub fn is_installed(&self, hash: &ContentHash) -> bool {
        self.skills.contains_key(hash)
    }

    /// Get the number of installed skills.
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Install a skill (placeholder - Agent #9 implements fully).
    pub fn install(&mut self, name: &str, metadata: SkillMetadata) -> Result<ContentHash, SkillError> {
        let hash = ContentHash::from_string(&format!("skill:{}", name));
        
        if self.skills.contains_key(&hash) {
            return Err(SkillError::AlreadyInstalled(name.to_string()));
        }

        let entry = SkillEntry {
            hash,
            metadata,
            verified: false,
            installed_at: chrono::Utc::now().timestamp_millis() as u64,
        };

        self.skills.insert(hash, entry);
        self.name_index.insert(name.to_string(), hash);

        Ok(hash)
    }

    /// Uninstall a skill.
    pub fn uninstall(&mut self, hash: &ContentHash) -> Result<(), SkillError> {
        if let Some(entry) = self.skills.remove(hash) {
            self.name_index.remove(&entry.metadata.name);
            Ok(())
        } else {
            Err(SkillError::NotFound(hash.to_string()))
        }
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of skill verification.
#[derive(Debug)]
pub struct VerificationResult {
    /// Whether the skill is safe
    pub safe: bool,
    /// Warnings (non-blocking issues)
    pub warnings: Vec<String>,
    /// Errors (blocking issues)
    pub errors: Vec<String>,
}

impl VerificationResult {
    /// Create a passing verification result.
    pub fn pass() -> Self {
        Self {
            safe: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create a failing verification result.
    pub fn fail(error: &str) -> Self {
        Self {
            safe: false,
            warnings: Vec::new(),
            errors: vec![error.to_string()],
        }
    }
}
