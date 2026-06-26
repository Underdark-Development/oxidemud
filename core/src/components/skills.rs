use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Marker component for NPC mobiles that can train skills.
/// Attached to NPC entities spawned from mob templates with `trainer_types`.
#[derive(Debug, Clone)]
pub struct Trainer {
    pub trainer_types: Vec<String>,
}

impl Trainer {
    pub fn new(trainer_types: Vec<String>) -> Self {
        Trainer { trainer_types }
    }

    pub fn can_train(&self, skill_type: &str) -> bool {
        self.trainer_types.is_empty() || self.trainer_types.iter().any(|t| t == skill_type)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillType {
    Combat,
    Magic,
    Craft,
    Lore,
    Physical,
    Social,
}

impl SkillType {
    pub fn all() -> &'static [SkillType] {
        &[
            SkillType::Combat,
            SkillType::Magic,
            SkillType::Craft,
            SkillType::Lore,
            SkillType::Physical,
            SkillType::Social,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub max_rank: u16,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

impl SkillDef {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        skill_type: SkillType,
    ) -> Self {
        SkillDef {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            skill_type,
            max_rank: 100,
            script: None,
            params: HashMap::new(),
        }
    }
}

/// Tracks per-entity skill ranks.
#[derive(Debug, Clone)]
pub struct LearnedSkills {
    pub skills: HashMap<String, u16>,
}

impl LearnedSkills {
    pub fn new() -> Self {
        LearnedSkills {
            skills: HashMap::new(),
        }
    }

    pub fn grant(&mut self, skill_id: impl Into<String>) {
        self.skills.entry(skill_id.into()).or_insert(1);
    }

    pub fn has(&self, skill_id: &str) -> bool {
        self.skills.contains_key(skill_id)
    }

    pub fn rank(&self, skill_id: &str) -> u16 {
        self.skills.get(skill_id).copied().unwrap_or(0)
    }

    pub fn set_rank(&mut self, skill_id: &str, rank: u16) {
        self.skills.insert(skill_id.to_string(), rank);
    }
}

impl Default for LearnedSkills {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource holding max skill rank per level for a given skill type.
#[derive(Debug, Clone)]
pub struct SkillCap {
    pub skill_type: SkillType,
    pub base_cap: u16,
    pub per_level: u16,
}

impl SkillCap {
    pub fn new(skill_type: SkillType) -> Self {
        SkillCap {
            skill_type,
            base_cap: 5,
            per_level: 5,
        }
    }

    pub fn for_level(&self, level: u8) -> u16 {
        self.base_cap + self.per_level * level as u16
    }

    pub fn defaults() -> Vec<SkillCap> {
        SkillType::all()
            .iter()
            .map(|&st| SkillCap::new(st))
            .collect()
    }
}
