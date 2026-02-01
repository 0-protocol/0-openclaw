//! Built-in skills for 0-openclaw.
//!
//! These skills are bundled with the platform and provide essential functionality.
//! They serve as both useful tools and examples for creating custom skills.

mod echo;
mod search;
mod browser;
mod calendar;

pub use echo::create_echo_skill;
pub use search::create_search_skill;
pub use browser::create_browser_skill;
pub use calendar::create_calendar_skill;

use super::graph::SkillGraph;

/// List all built-in skill names.
pub fn builtin_skill_names() -> Vec<&'static str> {
    vec!["echo", "search", "browser", "calendar"]
}

/// Create all built-in skills.
pub fn create_all_builtin() -> Vec<(&'static str, SkillGraph)> {
    vec![
        ("echo", create_echo_skill()),
        ("search", create_search_skill()),
        ("browser", create_browser_skill()),
        ("calendar", create_calendar_skill()),
    ]
}

/// Get a built-in skill by name.
pub fn get_builtin(name: &str) -> Option<SkillGraph> {
    match name {
        "echo" => Some(create_echo_skill()),
        "search" => Some(create_search_skill()),
        "browser" => Some(create_browser_skill()),
        "calendar" => Some(create_calendar_skill()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_names() {
        let names = builtin_skill_names();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"search"));
        assert!(names.contains(&"browser"));
        assert!(names.contains(&"calendar"));
    }

    #[test]
    fn test_create_all() {
        let skills = create_all_builtin();
        assert_eq!(skills.len(), 4);
    }

    #[test]
    fn test_get_builtin() {
        assert!(get_builtin("echo").is_some());
        assert!(get_builtin("nonexistent").is_none());
    }
}
