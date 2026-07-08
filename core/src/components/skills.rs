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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Targeting {
    #[default]
    SelfTarget,
    Single {
        range: u8,
    },
    Room,
    Area {
        radius: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum ResourceCost {
    #[default]
    None,
    Stamina(u16),
    Mana(u16),
    Energy(u16),
    Psi(u16),
    Gold(u64),
    Xp(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EffectTemplate {
    Damage {
        dice: String,
    },
    Heal {
        dice: String,
    },
    Buff {
        stat: String,
        amount: i32,
        duration: u32,
    },
    Debuff {
        stat: String,
        amount: i32,
        duration: u32,
    },
    Teleport {
        room: String,
    },
    Script {
        id: String,
    },
    Spawn {
        mob_id: String,
        count: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryEffect {
    pub effect: EffectTemplate,
    pub remaining_secs: u32,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct SkillCooldowns {
    pub cooldowns: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub max_rank: u16,

    #[serde(default)]
    pub level_requirement: u8,
    #[serde(default)]
    pub cooldown_secs: u32,
    #[serde(default)]
    pub targeting: Targeting,
    #[serde(default)]
    pub cost: ResourceCost,
    #[serde(default)]
    pub effect: Option<EffectTemplate>,
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    #[serde(default)]
    pub allowed_races: Vec<String>,
    #[serde(default)]
    pub requires_skill: Option<String>,
    #[serde(default)]
    pub must_train: bool,
    #[serde(default)]
    pub trainer_types: Vec<String>,
    #[serde(default)]
    pub use_while_fighting: bool,
    #[serde(default)]
    pub use_while_sitting: bool,

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
            level_requirement: 1,
            cooldown_secs: 0,
            targeting: Targeting::SelfTarget,
            cost: ResourceCost::None,
            effect: None,
            allowed_classes: Vec::new(),
            allowed_races: Vec::new(),
            requires_skill: None,
            must_train: false,
            trainer_types: Vec::new(),
            use_while_fighting: true,
            use_while_sitting: false,
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
