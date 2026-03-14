//! Trade skill - simulation-first Web3/trading workflow.
//!
//! This skill intentionally defaults to simulation output so the gateway can
//! request approval before any irreversible side effect.

use crate::skills::graph::{SafetyProof, SkillGraph};

pub fn create_trade_skill() -> SkillGraph {
    SkillGraph::builder("trade")
        .description("Simulate a trade and request approval before execution")
        .version("1.0.0")
        .add_input("message", "string")
        .add_constant("action_type", serde_json::json!("send_message"))
        .add_constant(
            "content",
            serde_json::json!(
                "Trade simulation ready. Risk score: medium. Reply `approve trade` to continue."
            ),
        )
        .outputs(vec!["action_type", "content"])
        .permission("network")
        .proof(SafetyProof {
            max_steps: 8,
            fuel_budget: 200,
            halting_proven: true,
            memory_bound: Some(64 * 1024),
        })
        .build()
}

