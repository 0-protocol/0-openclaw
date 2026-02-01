//! Message routing for 0-openclaw.
//!
//! The router determines which skill should handle an incoming message
//! based on content analysis and routing rules.

use std::collections::HashMap;
use crate::types::{ContentHash, IncomingMessage};
use crate::error::GatewayError;
use super::proof::ExecutionTrace;

/// Route information describing how to handle a message.
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Target skill hash
    pub skill_hash: ContentHash,
    
    /// Confidence in this routing decision
    pub confidence: f32,
    
    /// Name of the matched route
    pub route_name: String,
    
    /// Extracted parameters from the message
    pub params: HashMap<String, String>,
}

/// A routing rule.
#[derive(Debug, Clone)]
pub struct RouteRule {
    /// Rule name
    pub name: String,
    
    /// Target skill hash
    pub skill_hash: ContentHash,
    
    /// Condition for this rule
    pub condition: RouteCondition,
    
    /// Priority (higher = checked first)
    pub priority: u32,
    
    /// Minimum confidence threshold
    pub threshold: f32,
}

/// Conditions for route matching.
#[derive(Debug, Clone)]
pub enum RouteCondition {
    /// Message starts with a prefix
    StartsWith(String),
    
    /// Message contains a substring
    Contains(String),
    
    /// Message matches a regex pattern
    Regex(String),
    
    /// Always matches (default route)
    Default,
    
    /// Message is from a specific channel
    Channel(String),
    
    /// Custom condition (evaluated by callback)
    Custom(String),
}

impl RouteCondition {
    /// Check if the condition matches a message.
    pub fn matches(&self, message: &IncomingMessage) -> Option<f32> {
        match self {
            RouteCondition::StartsWith(prefix) => {
                if message.content.starts_with(prefix) {
                    Some(0.95)
                } else {
                    None
                }
            }
            RouteCondition::Contains(substring) => {
                if message.content.contains(substring) {
                    Some(0.8)
                } else {
                    None
                }
            }
            RouteCondition::Regex(pattern) => {
                regex::Regex::new(pattern)
                    .ok()
                    .and_then(|re| {
                        if re.is_match(&message.content) {
                            Some(0.85)
                        } else {
                            None
                        }
                    })
            }
            RouteCondition::Default => Some(0.5),
            RouteCondition::Channel(channel_id) => {
                if message.channel_id == *channel_id {
                    Some(0.9)
                } else {
                    None
                }
            }
            RouteCondition::Custom(_) => {
                // Custom conditions would need external evaluation
                None
            }
        }
    }
}

/// Message router.
pub struct Router {
    /// Routing rules
    rules: Vec<RouteRule>,
    
    /// Cached routes for fast lookup
    route_cache: HashMap<ContentHash, RouteResult>,
    
    /// Default skill hash (fallback)
    default_skill: ContentHash,
    
    /// Whether to use caching
    caching_enabled: bool,
}

impl Router {
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            route_cache: HashMap::new(),
            default_skill: ContentHash::from_string("skill:default"),
            caching_enabled: true,
        }
    }

    /// Create a router with a default skill.
    pub fn with_default_skill(default_skill: ContentHash) -> Self {
        Self {
            rules: Vec::new(),
            route_cache: HashMap::new(),
            default_skill,
            caching_enabled: true,
        }
    }

    /// Add a routing rule.
    pub fn add_rule(&mut self, rule: RouteRule) {
        self.rules.push(rule);
        // Sort by priority (highest first)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Add a command route (starts with /command).
    pub fn add_command(&mut self, command: &str, skill_hash: ContentHash) {
        let prefix = if command.starts_with('/') {
            command.to_string()
        } else {
            format!("/{}", command)
        };

        self.add_rule(RouteRule {
            name: format!("command:{}", command),
            skill_hash,
            condition: RouteCondition::StartsWith(prefix),
            priority: 100,
            threshold: 0.9,
        });
    }

    /// Set the default skill.
    pub fn set_default_skill(&mut self, skill_hash: ContentHash) {
        self.default_skill = skill_hash;
    }

    /// Enable or disable caching.
    pub fn set_caching(&mut self, enabled: bool) {
        self.caching_enabled = enabled;
        if !enabled {
            self.route_cache.clear();
        }
    }

    /// Route a message to a skill.
    pub fn route(
        &mut self,
        message: &IncomingMessage,
    ) -> Result<(RouteResult, ExecutionTrace), GatewayError> {
        let mut trace = ExecutionTrace::new();
        
        // Check cache first
        if self.caching_enabled {
            let cache_key = Self::cache_key(message);
            if let Some(cached) = self.route_cache.get(&cache_key) {
                return Ok((cached.clone(), ExecutionTrace::cached()));
            }
        }

        // Try each rule in priority order
        for rule in &self.rules {
            trace.add_node(ContentHash::from_string(&rule.name));
            
            if let Some(match_confidence) = rule.condition.matches(message) {
                if match_confidence >= rule.threshold {
                    let result = RouteResult {
                        skill_hash: rule.skill_hash,
                        confidence: match_confidence,
                        route_name: rule.name.clone(),
                        params: Self::extract_params(message, &rule.condition),
                    };

                    // Cache the result
                    if self.caching_enabled {
                        let cache_key = Self::cache_key(message);
                        self.route_cache.insert(cache_key, result.clone());
                    }

                    return Ok((result, trace));
                }
            }
        }

        // Use default route
        trace.add_node(ContentHash::from_string("default_route"));
        
        let result = RouteResult {
            skill_hash: self.default_skill,
            confidence: 0.5,
            route_name: "default".to_string(),
            params: HashMap::new(),
        };

        Ok((result, trace))
    }

    /// Generate a cache key for a message.
    fn cache_key(message: &IncomingMessage) -> ContentHash {
        // Extract the pattern for caching (e.g., command prefix)
        if message.content.starts_with('/') {
            let command = message.content.split_whitespace().next().unwrap_or("");
            ContentHash::from_string(command)
        } else {
            // For non-commands, we don't cache (content is too variable)
            ContentHash::from_bytes(format!("nocache:{}", message.id.to_hex()).as_bytes())
        }
    }

    /// Extract parameters from a message based on the route condition.
    fn extract_params(
        message: &IncomingMessage,
        condition: &RouteCondition,
    ) -> HashMap<String, String> {
        let mut params = HashMap::new();

        match condition {
            RouteCondition::StartsWith(prefix) => {
                // Extract arguments after the prefix
                if message.content.len() > prefix.len() {
                    let args = message.content[prefix.len()..].trim();
                    if !args.is_empty() {
                        params.insert("args".to_string(), args.to_string());
                        
                        // Also split into individual arguments
                        for (i, arg) in args.split_whitespace().enumerate() {
                            params.insert(format!("arg{}", i), arg.to_string());
                        }
                    }
                }
            }
            RouteCondition::Regex(pattern) => {
                // Extract named capture groups
                if let Ok(re) = regex::Regex::new(pattern) {
                    if let Some(caps) = re.captures(&message.content) {
                        for name in re.capture_names().flatten() {
                            if let Some(m) = caps.name(name) {
                                params.insert(name.to_string(), m.as_str().to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        params
    }

    /// Clear the route cache.
    pub fn clear_cache(&mut self) {
        self.route_cache.clear();
    }

    /// Get the number of routing rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get the cache size.
    pub fn cache_size(&self) -> usize {
        self.route_cache.len()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating routers with a fluent API.
pub struct RouterBuilder {
    router: Router,
}

impl RouterBuilder {
    /// Create a new router builder.
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    /// Add a command route.
    pub fn command(mut self, command: &str, skill_hash: ContentHash) -> Self {
        self.router.add_command(command, skill_hash);
        self
    }

    /// Add a custom rule.
    pub fn rule(mut self, rule: RouteRule) -> Self {
        self.router.add_rule(rule);
        self
    }

    /// Set the default skill.
    pub fn default_skill(mut self, skill_hash: ContentHash) -> Self {
        self.router.set_default_skill(skill_hash);
        self
    }

    /// Enable or disable caching.
    pub fn caching(mut self, enabled: bool) -> Self {
        self.router.set_caching(enabled);
        self
    }

    /// Build the router.
    pub fn build(self) -> Router {
        self.router
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_message(content: &str) -> IncomingMessage {
        IncomingMessage::new("test", "user", content)
    }

    #[test]
    fn test_starts_with_condition() {
        let condition = RouteCondition::StartsWith("/help".to_string());
        
        assert!(condition.matches(&test_message("/help")).is_some());
        assert!(condition.matches(&test_message("/help me")).is_some());
        assert!(condition.matches(&test_message("help")).is_none());
    }

    #[test]
    fn test_contains_condition() {
        let condition = RouteCondition::Contains("hello".to_string());
        
        assert!(condition.matches(&test_message("say hello world")).is_some());
        assert!(condition.matches(&test_message("hello")).is_some());
        assert!(condition.matches(&test_message("hi there")).is_none());
    }

    #[test]
    fn test_default_condition() {
        let condition = RouteCondition::Default;
        assert!(condition.matches(&test_message("anything")).is_some());
    }

    #[test]
    fn test_router_command() {
        let mut router = Router::new();
        let help_skill = ContentHash::from_string("skill:help");
        router.add_command("help", help_skill);

        let (result, _trace) = router.route(&test_message("/help")).unwrap();
        assert_eq!(result.skill_hash, help_skill);
        assert_eq!(result.route_name, "command:help");
    }

    #[test]
    fn test_router_default() {
        let mut router = Router::new();
        let default_skill = ContentHash::from_string("skill:chat");
        router.set_default_skill(default_skill);

        let (result, _trace) = router.route(&test_message("hello there")).unwrap();
        assert_eq!(result.skill_hash, default_skill);
        assert_eq!(result.route_name, "default");
    }

    #[test]
    fn test_router_priority() {
        let mut router = Router::new();
        
        // Add low priority rule
        router.add_rule(RouteRule {
            name: "low".to_string(),
            skill_hash: ContentHash::from_string("skill:low"),
            condition: RouteCondition::StartsWith("/test".to_string()),
            priority: 10,
            threshold: 0.5,
        });
        
        // Add high priority rule
        router.add_rule(RouteRule {
            name: "high".to_string(),
            skill_hash: ContentHash::from_string("skill:high"),
            condition: RouteCondition::StartsWith("/test".to_string()),
            priority: 100,
            threshold: 0.5,
        });

        let (result, _trace) = router.route(&test_message("/test")).unwrap();
        assert_eq!(result.route_name, "high");
    }

    #[test]
    fn test_param_extraction() {
        let mut router = Router::new();
        router.add_command("remind", ContentHash::from_string("skill:remind"));

        let (result, _trace) = router.route(&test_message("/remind buy milk")).unwrap();
        
        assert_eq!(result.params.get("args"), Some(&"buy milk".to_string()));
        assert_eq!(result.params.get("arg0"), Some(&"buy".to_string()));
        assert_eq!(result.params.get("arg1"), Some(&"milk".to_string()));
    }

    #[test]
    fn test_router_builder() {
        let router = RouterBuilder::new()
            .command("help", ContentHash::from_string("skill:help"))
            .command("status", ContentHash::from_string("skill:status"))
            .default_skill(ContentHash::from_string("skill:chat"))
            .caching(true)
            .build();

        assert_eq!(router.rule_count(), 2);
    }

    #[test]
    fn test_caching() {
        let mut router = Router::new();
        router.add_command("test", ContentHash::from_string("skill:test"));

        // First call should not be cached
        let (_, trace1) = router.route(&test_message("/test")).unwrap();
        assert!(!trace1.cached);

        // Second call should be cached
        let (_, trace2) = router.route(&test_message("/test")).unwrap();
        assert!(trace2.cached);

        // Clear cache
        router.clear_cache();
        let (_, trace3) = router.route(&test_message("/test")).unwrap();
        assert!(!trace3.cached);
    }
}
