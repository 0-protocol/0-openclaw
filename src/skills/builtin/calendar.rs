//! Calendar skill - calendar management and scheduling.
//!
//! This skill provides calendar functionality including
//! event creation, listing, and availability checking.

use crate::skills::graph::{SkillGraph, Op, SafetyProof};

/// Create the calendar skill graph.
///
/// The calendar skill retrieves and formats calendar events.
///
/// # Inputs
/// - `date`: StringTensor containing the date to query (YYYY-MM-DD)
///
/// # Outputs
/// - `events`: StringTensor containing formatted calendar events
pub fn create_calendar_skill() -> SkillGraph {
    SkillGraph::builder("calendar")
        .description("Retrieves and formats calendar events")
        .version("1.0.0")
        .add_input("date", "string")
        .add_constant(
            "date_format",
            serde_json::json!("YYYY-MM-DD"),
        )
        .add_operation(
            "validate_date",
            Op::Identity,
            vec!["date"],
        )
        .add_external(
            "calendar_api",
            "calendar://events",
            vec!["validate_date"],
        )
        .add_operation(
            "parse_events",
            Op::JsonParse,
            vec!["calendar_api"],
        )
        .add_operation(
            "format_events",
            Op::StringFormat { 
                template: "Calendar Events for {}:\n{}".to_string() 
            },
            vec!["date", "parse_events"],
        )
        .output("format_events")
        .proof(SafetyProof {
            max_steps: 50,
            fuel_budget: 1000,
            halting_proven: true,
            memory_bound: Some(512 * 1024), // 512KB
        })
        .build()
}

/// Create a calendar skill for adding events.
pub fn create_calendar_add_skill() -> SkillGraph {
    SkillGraph::builder("calendar_add")
        .description("Adds a new calendar event")
        .version("1.0.0")
        .add_input("title", "string")
        .add_input("date", "string")
        .add_input("time", "string")
        .add_input("duration", "string")
        .add_operation(
            "build_event",
            Op::StringFormat {
                template: r#"{"title": "{}", "date": "{}", "time": "{}", "duration": "{}"}"#.to_string()
            },
            vec!["title", "date", "time", "duration"],
        )
        .add_operation(
            "parse_event",
            Op::JsonParse,
            vec!["build_event"],
        )
        .add_external(
            "calendar_api",
            "calendar://events/add",
            vec!["parse_event"],
        )
        .add_operation(
            "format_result",
            Op::StringFormat {
                template: "Event '{}' added for {} at {}".to_string()
            },
            vec!["title", "date", "time"],
        )
        .output("format_result")
        .proof(SafetyProof {
            max_steps: 30,
            fuel_budget: 500,
            halting_proven: true,
            memory_bound: Some(256 * 1024), // 256KB
        })
        .build()
}

/// Create a calendar skill for checking availability.
pub fn create_calendar_availability_skill() -> SkillGraph {
    SkillGraph::builder("calendar_availability")
        .description("Checks calendar availability for a time range")
        .version("1.0.0")
        .add_input("start_date", "string")
        .add_input("end_date", "string")
        .add_external(
            "calendar_api",
            "calendar://availability",
            vec!["start_date", "end_date"],
        )
        .add_operation(
            "parse",
            Op::JsonParse,
            vec!["calendar_api"],
        )
        .add_operation(
            "extract_slots",
            Op::JsonGet { path: "available_slots".to_string() },
            vec!["parse"],
        )
        .add_operation(
            "format",
            Op::StringFormat {
                template: "Available slots from {} to {}:\n{}".to_string()
            },
            vec!["start_date", "end_date", "extract_slots"],
        )
        .output("format")
        .proof(SafetyProof {
            max_steps: 40,
            fuel_budget: 800,
            halting_proven: true,
            memory_bound: Some(512 * 1024),
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::verifier::SkillVerifier;

    #[test]
    fn test_calendar_skill_structure() {
        let skill = create_calendar_skill();
        
        assert_eq!(skill.name, "calendar");
        assert!(skill.has_input("date"));
    }

    #[test]
    fn test_calendar_skill_verifies() {
        let skill = create_calendar_skill();
        let result = SkillVerifier::verify(&skill).unwrap();
        
        // Note: calendar uses external URIs so it may have warnings
        // but should still be safe
        assert!(result.safe, "Calendar skill should be safe: {:?}", result.errors);
    }

    #[test]
    fn test_calendar_add_skill() {
        let skill = create_calendar_add_skill();
        
        assert_eq!(skill.name, "calendar_add");
        assert!(skill.has_input("title"));
        assert!(skill.has_input("date"));
        assert!(skill.has_input("time"));
    }

    #[test]
    fn test_calendar_availability_skill() {
        let skill = create_calendar_availability_skill();
        
        assert_eq!(skill.name, "calendar_availability");
        assert!(skill.has_input("start_date"));
        assert!(skill.has_input("end_date"));
    }
}
