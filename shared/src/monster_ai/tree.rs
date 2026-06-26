//! Behavior tree data model (loaded from data-src/behavior_trees.json) and the
//! node evaluation status type.

use super::DEFAULT_BEHAVIOR;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BehaviorTreeFile {
    #[serde(default)]
    pub schema_version: u32,
    pub trees: HashMap<String, BehaviorTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorTree {
    #[serde(default)]
    pub description: Option<String>,
    pub root: BehaviorNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorNode {
    Selector {
        children: Vec<BehaviorNode>,
    },
    Sequence {
        children: Vec<BehaviorNode>,
    },
    Condition {
        name: String,
        #[serde(default)]
        params: HashMap<String, f32>,
    },
    Action {
        name: String,
        #[serde(default)]
        params: HashMap<String, f32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BehaviorStatus {
    Success,
    Failure,
    Running,
}

impl From<bool> for BehaviorStatus {
    fn from(passed: bool) -> Self {
        if passed {
            BehaviorStatus::Success
        } else {
            BehaviorStatus::Failure
        }
    }
}

/// Load behavior trees from JSON string (data-src/behavior_trees.json).
pub fn load_behavior_trees(json: &str) -> Result<HashMap<String, BehaviorTree>, serde_json::Error> {
    let file: BehaviorTreeFile = serde_json::from_str(json)?;
    Ok(file.trees)
}

pub fn behavior_tree_for<'a>(
    trees: &'a HashMap<String, BehaviorTree>,
    behavior: &str,
) -> Option<&'a BehaviorTree> {
    trees.get(behavior).or_else(|| trees.get(DEFAULT_BEHAVIOR))
}
