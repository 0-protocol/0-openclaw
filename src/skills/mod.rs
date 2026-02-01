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
//! ## Components
//!
//! - **SkillGraph**: The core data structure representing a skill as a DAG
//! - **SkillRegistry**: Content-addressed storage for installed skills
//! - **SkillComposer**: Combine multiple skills into workflows
//! - **SkillVerifier**: Verify safety properties of skill graphs
//! - **SkillLoader**: Load skills from files and network
//!
//! ## Example
//!
//! ```rust,ignore
//! use zero_openclaw::skills::{SkillRegistry, SkillGraph, SkillVerifier};
//!
//! // Create a registry
//! let mut registry = SkillRegistry::new("graphs/skills");
//!
//! // Load built-in skills
//! registry.load_builtin()?;
//!
//! // Get a skill by name
//! if let Some(echo) = registry.get_by_name("echo") {
//!     println!("Echo skill: {:?}", echo.hash);
//! }
//!
//! // Create a custom skill
//! let custom = SkillGraph::builder("my_skill")
//!     .description("My custom skill")
//!     .add_input("message", "string")
//!     .add_operation("output", Op::Identity, vec!["message"])
//!     .output("output")
//!     .build();
//!
//! // Verify before installing
//! let result = SkillVerifier::verify(&custom)?;
//! if result.safe {
//!     registry.install_graph("my_skill", custom, false)?;
//! }
//! ```
//!
//! ## Implementation Status
//!
//! This module is implemented by **Agent #9**.
//!
//! See: `AGENT-9-0OPENCLAW-SKILLS.md`

// Core graph module
pub mod graph;

// Registry for skill management
pub mod registry;

// Skill composition
pub mod composer;

// Safety verification
pub mod verifier;

// File/network loader
pub mod loader;

// Built-in skills
pub mod builtin;

// Re-export main types
pub use graph::{SkillGraph, SkillNode, Op, SafetyProof, SkillGraphBuilder};
pub use registry::{SkillRegistry, SkillEntry, SkillMetadata, SkillInput, SkillOutput};
pub use composer::{SkillComposer, SkillConnection, ComposedSkill, ComposerError};
pub use verifier::{SkillVerifier, VerificationResult, VerificationWarning, VerificationError};
pub use loader::SkillLoader;

use crate::error::SkillError;

/// Create a pre-configured skill registry with built-in skills loaded.
///
/// # Arguments
/// * `skills_dir` - Directory for skill graph files
///
/// # Example
/// ```rust,ignore
/// let registry = create_registry("graphs/skills")?;
/// assert!(registry.get_by_name("echo").is_some());
/// ```
pub fn create_registry(skills_dir: impl Into<std::path::PathBuf>) -> Result<SkillRegistry, SkillError> {
    let mut registry = SkillRegistry::new(skills_dir);
    registry.load_builtin()?;
    Ok(registry)
}

/// Verify a skill graph and return whether it's safe.
///
/// This is a convenience function that performs full verification
/// and returns a boolean result.
pub fn is_skill_safe(graph: &SkillGraph) -> bool {
    match SkillVerifier::verify(graph) {
        Ok(result) => result.safe,
        Err(_) => false,
    }
}

/// Quick composition helper for chaining two skills.
///
/// # Example
/// ```rust,ignore
/// let composed = compose_skills(
///     "pipeline",
///     skill1,
///     "output",
///     skill2,
///     "input",
/// )?;
/// ```
pub fn compose_skills(
    name: &str,
    skill1: SkillGraph,
    output1: &str,
    skill2: SkillGraph,
    input2: &str,
) -> Result<ComposedSkill, ComposerError> {
    let mut composer = SkillComposer::new();
    let hash1 = composer.add_skill(skill1);
    let hash2 = composer.add_skill(skill2);
    composer.connect(hash1, output1, hash2, input2);
    composer.compose(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_registry() {
        let registry = create_registry("/tmp/skills").unwrap();
        
        // Should have built-in skills
        assert!(registry.get_by_name("echo").is_some());
        assert!(registry.get_by_name("search").is_some());
        assert!(registry.get_by_name("browser").is_some());
        assert!(registry.get_by_name("calendar").is_some());
    }

    #[test]
    fn test_is_skill_safe() {
        let safe_skill = SkillGraph::builder("safe")
            .add_input("x", "string")
            .output("x")
            .build();
        
        assert!(is_skill_safe(&safe_skill));
        
        let empty_skill = SkillGraph::builder("empty").build();
        assert!(!is_skill_safe(&empty_skill));
    }

    #[test]
    fn test_compose_skills_helper() {
        let skill1 = SkillGraph::builder("s1")
            .add_input("input", "string")
            .add_operation("output", Op::Identity, vec!["input"])
            .output("output")
            .build();
        
        let skill2 = SkillGraph::builder("s2")
            .add_input("input", "string")
            .add_operation("output", Op::Identity, vec!["input"])
            .output("output")
            .build();
        
        let composed = compose_skills("test", skill1, "output", skill2, "input").unwrap();
        assert_eq!(composed.source_skills.len(), 2);
    }

    #[test]
    fn test_builtin_skills_verify() {
        for (name, skill) in builtin::create_all_builtin() {
            let result = SkillVerifier::verify(&skill).unwrap();
            assert!(result.safe, "Built-in skill '{}' should be safe: {:?}", name, result.errors);
        }
    }

    #[test]
    fn test_skill_content_addressing() {
        // Same skill created twice should have same hash
        let skill1 = builtin::create_echo_skill();
        let skill2 = builtin::create_echo_skill();
        
        assert_eq!(skill1.content_hash(), skill2.content_hash());
    }
}
