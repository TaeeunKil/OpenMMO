//! Shared monster AI behavior tree runtime — used by both WASM (client) and native Rust (agent-client).
//!
//! The runtime is stateful per-monster via [`MonsterBrain`]. Each tick receives
//! external inputs (delta time, nearby players) and returns a list of
//! [`AiCommand`]s that the caller translates into network messages.
//!
//! The module is split into:
//! - [`tree`] — behavior tree data model and JSON loading
//! - [`command`] — AI state and behavior input/output types
//! - [`path`] — pathfinding abstraction ([`PathProvider`])
//! - [`brain`] — the per-monster [`MonsterBrain`] (struct, tick, event handlers)
//! - [`behavior`] — behavior tree execution (condition/action evaluation)
//! - [`movement`] — state transitions and path-following helpers

use std::collections::HashMap;

mod behavior;
mod brain;
mod command;
mod movement;
mod path;
mod tree;

#[cfg(test)]
mod tests;

pub use brain::MonsterBrain;
pub use command::{AiCommand, AiState, NearbyPlayer, TickResult};
pub use path::{CachePathProvider, PathProvider};
pub use tree::{behavior_tree_for, load_behavior_trees, BehaviorNode, BehaviorTree};

// ---------------------------------------------------------------------------
// Shared tuning constants. Public ones are part of the module's API; private
// ones are visible to all submodules as descendants of `monster_ai`.
// ---------------------------------------------------------------------------

const DEFAULT_IDLE_CHECK_MS: f32 = 1000.0;
const DEFAULT_MIN_MOVE_DIST: f32 = 2.0;
const DEFAULT_MAX_MOVE_DIST: f32 = 10.0;
pub const DEFAULT_WALK_SPEED: f32 = 1.0;
pub const DEFAULT_RUN_SPEED: f32 = 8.0;
pub const DEFAULT_ATTACK_RANGE: f32 = 2.0;
pub const DEFAULT_CHASE_RANGE: f32 = 25.0;
pub const DEFAULT_ATTACK_COOLDOWN_MS: f32 = 1500.0;
const DEFAULT_LEASH_RANGE: f32 = 50.0;
const DEFAULT_HIT_STAGGER_MS: f32 = 800.0;
const DEFAULT_FLEE_HEALTH_RATIO: f32 = 0.0;
const DEFAULT_FLEE_MAX_DURATION_MS: f32 = 15000.0;
const FLEE_SAFE_DIST_MARGIN: f32 = 5.0;
const DEFAULT_RETURN_ARRIVE_DIST: f32 = 5.0;
const DEFAULT_PATH_RECALC_MS: f32 = 500.0;
const DEFAULT_TARGET_MOVE_THRESHOLD: f32 = 3.0;
pub const DEFAULT_BEHAVIOR: &str = "brave";
/// Behavior tree used by proactive (선공형) monsters that acquire and attack
/// targets on sight. Selected when `Monster::aggressive` is set, overriding the
/// monster type's configured behavior.
pub const AGGRESSIVE_BEHAVIOR: &str = "aggressive";

fn param(params: &HashMap<String, f32>, name: &str, default: f32) -> f32 {
    params.get(name).copied().unwrap_or(default)
}
