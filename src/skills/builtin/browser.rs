//! Browser skill - web browsing and automation.
//!
//! This skill provides browser automation functionality including
//! page navigation, content extraction, and interaction.

use crate::skills::graph::{SkillGraph, Op, SafetyProof};

/// Create the browser skill graph.
///
/// The browser skill navigates to a URL and extracts page content.
///
/// # Inputs
/// - `url`: StringTensor containing the URL to navigate to
///
/// # Outputs
/// - `content`: StringTensor containing the page content
///
/// # Permissions
/// - `network`: Required for making HTTP requests
pub fn create_browser_skill() -> SkillGraph {
    SkillGraph::builder("browser")
        .description("Fetches and extracts web page content")
        .version("1.0.0")
        .add_input("url", "string")
        .add_operation(
            "fetch",
            Op::HttpGet,
            vec!["url"],
        )
        .add_operation(
            "extract_text",
            Op::StringFormat { template: "Page Content:\n{}".to_string() },
            vec!["fetch"],
        )
        .output("extract_text")
        .permission("network")
        .proof(SafetyProof {
            max_steps: 100,
            fuel_budget: 5000,
            halting_proven: true,
            memory_bound: Some(5 * 1024 * 1024), // 5MB for web pages
        })
        .build()
}

/// Create a browser skill with content extraction.
///
/// This variant extracts specific elements from the page.
pub fn create_browser_extract_skill() -> SkillGraph {
    SkillGraph::builder("browser_extract")
        .description("Fetches web page and extracts specified content")
        .version("1.0.0")
        .add_input("url", "string")
        .add_input("selector", "string")
        .add_operation(
            "fetch",
            Op::HttpGet,
            vec!["url"],
        )
        .add_operation(
            "parse",
            Op::JsonParse,
            vec!["fetch"],
        )
        .add_operation(
            "extract",
            Op::JsonGet { path: "body".to_string() },
            vec!["parse"],
        )
        .add_operation(
            "format",
            Op::StringFormat { template: "Extracted Content:\n{}".to_string() },
            vec!["extract"],
        )
        .output("format")
        .permission("network")
        .proof(SafetyProof {
            max_steps: 200,
            fuel_budget: 10000,
            halting_proven: true,
            memory_bound: Some(10 * 1024 * 1024), // 10MB
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::verifier::SkillVerifier;

    #[test]
    fn test_browser_skill_structure() {
        let skill = create_browser_skill();
        
        assert_eq!(skill.name, "browser");
        assert!(skill.has_input("url"));
        assert!(skill.permissions.contains(&"network".to_string()));
    }

    #[test]
    fn test_browser_skill_verifies() {
        let skill = create_browser_skill();
        let result = SkillVerifier::verify(&skill).unwrap();
        
        assert!(result.safe, "Browser skill should be safe: {:?}", result.errors);
    }

    #[test]
    fn test_browser_extract_skill() {
        let skill = create_browser_extract_skill();
        
        assert_eq!(skill.name, "browser_extract");
        assert!(skill.has_input("url"));
        assert!(skill.has_input("selector"));
    }
}
