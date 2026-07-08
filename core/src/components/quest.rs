use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestLog {
    pub active: HashMap<String, QuestProgress>,
    pub completed: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProgress {
    pub quest_id: String,
    pub started_at_epoch_ms: u64,
    pub objectives: Vec<ObjectiveProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectiveProgress {
    pub current: u32,
    pub completed: bool,
}

impl QuestLog {
    pub fn new() -> Self {
        QuestLog::default()
    }
}
