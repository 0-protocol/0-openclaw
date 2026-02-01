//! Echo skill - echoes input back to output.
//!
//! This is the simplest possible skill, useful for testing
//! and as a template for creating other skills.

use crate::skills::graph::{SkillGraph, Op, SafetyProof};

/// Create the echo skill graph.
///
/// The echo skill takes a message input and returns it unchanged,
/// optionally with a prefix.
///
/// # Inputs
/// - `message`: StringTensor containing the message to echo
///
/// # Outputs  
/// - `response`: StringTensor containing the echoed message
pub fn create_echo_skill() -> SkillGraph {
    SkillGraph::builder("echo")
        .description("Echoes input back to output")
        .version("1.0.0")
        .add_input("message", "string")
        .add_operation(
            "format",
            Op::StringFormat { template: "Echo: {}".to_string() },
            vec!["message"],
        )
        .add_operation(
            "output",
            Op::Identity,
            vec!["format"],
        )
        .output("output")
        .proof(SafetyProof {
            max_steps: 3,
            fuel_budget: 100,
            halting_proven: true,
            memory_bound: Some(1024),
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::verifier::SkillVerifier;

    #[test]
    fn test_echo_skill_structure() {
        let skill = create_echo_skill();
        
        assert_eq!(skill.name, "echo");
        assert!(skill.description.is_some());
        assert!(!skill.nodes.is_empty());
        assert!(!skill.outputs.is_empty());
    }

    #[test]
    fn test_echo_skill_verifies() {
        let skill = create_echo_skill();
        let result = SkillVerifier::verify(&skill).unwrap();
        
        assert!(result.safe, "Echo skill should be safe: {:?}", result.errors);
    }

    #[test]
    fn test_echo_skill_has_input() {
        let skill = create_echo_skill();
        assert!(skill.has_input("message"));
    }

    #[test]
    fn test_echo_skill_deterministic() {
        let skill1 = create_echo_skill();
        let skill2 = create_echo_skill();
        
        assert_eq!(skill1.content_hash(), skill2.content_hash());
    }
}
