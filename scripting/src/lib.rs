pub mod bindings;
pub mod bridge;
pub mod context;
pub mod engine;
#[cfg(test)]
mod tests;

pub use context::{push_script_context, with_current_world, ScriptContextGuard, ScriptExecContext};
pub use engine::{construct_test_script, parse_test_blocks, strip_tests, ScriptEngine, TestBlock};

use oxide_core::{Entity, World};
use std::sync::OnceLock;

type AwardXpCallback = Box<dyn Fn(&mut World, Entity) -> Vec<String> + Send + Sync + 'static>;

static AWARD_XP_CALLBACK: OnceLock<AwardXpCallback> = OnceLock::new();

pub fn register_award_xp_callback(
    cb: impl Fn(&mut World, Entity) -> Vec<String> + Send + Sync + 'static,
) {
    let _ = AWARD_XP_CALLBACK.set(Box::new(cb));
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}
