//! Search skill - performs web searches.
//!
//! This skill provides web search functionality by calling
//! an external search API and formatting the results.

use crate::skills::graph::{SkillGraph, Op, SafetyProof};

/// Create the search skill graph.
///
/// The search skill takes a query and returns formatted search results.
///
/// # Inputs
/// - `query`: StringTensor containing the search query
///
/// # Outputs
/// - `results`: StringTensor containing formatted search results
///
/// # Permissions
/// - `network`: Required for making HTTP requests
pub fn create_search_skill() -> SkillGraph {
    SkillGraph::builder("search")
        .description("Performs web search and returns results")
        .version("1.0.0")
        .add_input("query", "string")
        .add_external(
            "search_api",
            "https://api.search.example/search",
            vec!["query"],
        )
        .add_operation(
            "parse",
            Op::JsonParse,
            vec!["search_api"],
        )
        .add_operation(
            "extract",
            Op::JsonGet { path: "results".to_string() },
            vec!["parse"],
        )
        .add_operation(
            "format",
            Op::StringFormat { template: "Search Results:\n{}".to_string() },
            vec!["extract"],
        )
        .output("format")
        .permission("network")
        .proof(SafetyProof {
            max_steps: 50,
            fuel_budget: 1000,
            halting_proven: true,
            memory_bound: Some(1024 * 1024), // 1MB
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::verifier::SkillVerifier;

    #[test]
    fn test_search_skill_structure() {
        let skill = create_search_skill();
        
        assert_eq!(skill.name, "search");
        assert!(skill.has_input("query"));
        assert!(skill.permissions.contains(&"network".to_string()));
    }

    #[test]
    fn test_search_skill_verifies() {
        let skill = create_search_skill();
        let result = SkillVerifier::verify(&skill).unwrap();
        
        assert!(result.safe, "Search skill should be safe: {:?}", result.errors);
        
        // Should have external call warning
        assert!(result.warnings.iter().any(|w| {
            matches!(w, crate::skills::verifier::VerificationWarning::ExternalCall { .. })
        }));
    }

    #[test]
    fn test_search_has_external_calls() {
        let skill = create_search_skill();
        assert!(skill.has_external_calls());
        
        let uris = skill.external_uris();
        assert!(!uris.is_empty());
    }
}
