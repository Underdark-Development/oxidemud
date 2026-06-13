use crate::dice::DiceRoll;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Dice roll helper for TOML — stored as string, parsed at use time
// ---------------------------------------------------------------------------

/// Custom deserialize for dice notation strings like "2d6" or "2d8+3".
/// Accepts a plain string in TOML: `dice = "2d6+3"`
#[derive(Debug, Clone)]
pub struct DiceString(pub String);

impl DiceString {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DiceString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Validate at parse time using DiceRoll parser
        if DiceRoll::from_str(&s).is_err() {
            return Err(serde::de::Error::custom(format!(
                "invalid dice notation: {s:?} — expected format like \"2d6\" or \"2d8+3\""
            )));
        }
        Ok(DiceString(s))
    }
}

// ---------------------------------------------------------------------------
// Race template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RaceAttributes {
    #[serde(default = "default_stat")]
    pub strength: u8,
    #[serde(default = "default_stat")]
    pub dexterity: u8,
    #[serde(default = "default_stat")]
    pub intelligence: u8,
    #[serde(default = "default_stat")]
    pub wisdom: u8,
    #[serde(default = "default_stat")]
    pub constitution: u8,
    #[serde(default = "default_stat")]
    pub charisma: u8,
}

impl Default for RaceAttributes {
    fn default() -> Self {
        RaceAttributes {
            strength: 10,
            dexterity: 10,
            intelligence: 10,
            wisdom: 10,
            constitution: 10,
            charisma: 10,
        }
    }
}

const fn default_stat() -> u8 {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaceTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub attributes: RaceAttributes,
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    #[serde(default)]
    pub racial_abilities: Vec<String>,
}

// ---------------------------------------------------------------------------
// Class template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ClassAttributeMods {
    #[serde(default)]
    pub strength: i8,
    #[serde(default)]
    pub dexterity: i8,
    #[serde(default)]
    pub intelligence: i8,
    #[serde(default)]
    pub wisdom: i8,
    #[serde(default)]
    pub constitution: i8,
    #[serde(default)]
    pub charisma: i8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_hit_die")]
    pub hit_die: u8,
    #[serde(default)]
    pub attribute_mods: ClassAttributeMods,
    #[serde(default)]
    pub allowed_races: Vec<String>,
    #[serde(default)]
    pub auto_skills: Vec<String>,
}

const fn default_hit_die() -> u8 {
    8
}

// ---------------------------------------------------------------------------
// Stance template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct StanceDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub ac_bonus: i8,
    #[serde(default)]
    pub attack_penalty: i8,
    #[serde(default)]
    pub damage_bonus: i8,
    #[serde(default)]
    pub ac_penalty: i8,
    #[serde(default = "default_min_level")]
    pub min_level: u8,
}

const fn default_min_level() -> u8 {
    1
}

// ---------------------------------------------------------------------------
// Item template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponDef {
    pub damage: DiceString,
    pub damage_type: String,
    #[serde(default = "default_weapon_speed")]
    pub speed: f32,
    #[serde(default = "default_weapon_range")]
    pub range: String,
}

fn default_weapon_speed() -> f32 {
    2.5
}

fn default_weapon_range() -> String {
    "melee".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct EquipmentDef {
    pub slot: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillRequirement {
    pub id: String,
    #[serde(default)]
    pub level: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerDef {
    pub event: String,
    #[serde(default)]
    pub chance: u8,
    pub cast: String,
    pub target: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetMembership {
    pub id: String,
    pub piece_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub item_type: String,
    #[serde(default)]
    pub subtype: String,
    #[serde(default = "default_quality")]
    pub quality: String,
    #[serde(default)]
    pub level_requirement: u8,
    #[serde(default)]
    pub weight: f32,
    #[serde(default)]
    pub value: u64,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    #[serde(default)]
    pub allowed_races: Vec<String>,
    #[serde(default)]
    pub allowed_alignments: Vec<String>,
    #[serde(default)]
    pub requires_skill: Option<SkillRequirement>,
    #[serde(default)]
    pub weapon: Option<WeaponDef>,
    #[serde(default)]
    pub equipment: Option<EquipmentDef>,
    #[serde(default)]
    pub set: Option<SetMembership>,
    #[serde(default)]
    pub triggers: Vec<TriggerDef>,
}

fn default_quality() -> String {
    "common".to_string()
}

// ---------------------------------------------------------------------------
// Mob template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct HealthBounds {
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MobEquipmentEntry {
    pub template_id: String,
    pub slot: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CountRange {
    #[serde(default = "default_count_min")]
    pub min: u8,
    #[serde(default = "default_count_max")]
    pub max: u8,
}

const fn default_count_min() -> u8 {
    1
}

const fn default_count_max() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct LootEntry {
    #[serde(default)]
    pub item: String,
    #[serde(default)]
    pub treasure_class: Option<String>,
    #[serde(default)]
    pub count: Option<CountRange>,
    #[serde(default = "default_chance")]
    pub chance: u8,
}

const fn default_chance() -> u8 {
    100
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LootTable {
    #[serde(default)]
    pub entries: Vec<LootEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MobSkillEntry {
    pub id: String,
    #[serde(default)]
    pub level: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptHookEntry {
    pub event: String,
    pub script: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MobTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub attributes: RaceAttributes,
    pub health: HealthBounds,
    #[serde(default)]
    pub armor: i32,
    #[serde(default)]
    pub damage: Option<String>,
    #[serde(default)]
    pub damage_type: Option<String>,
    #[serde(default)]
    pub race: Option<String>,
    #[serde(default = "default_size")]
    pub size: String,
    #[serde(default)]
    pub equipment: Vec<MobEquipmentEntry>,
    #[serde(default)]
    pub xp_value: u64,
    #[serde(default)]
    pub loot: LootTable,
    #[serde(default = "default_ai_mode")]
    pub ai_mode: String,
    #[serde(default)]
    pub aggro_range: u32,
    #[serde(default)]
    pub aggro_players: bool,
    #[serde(default)]
    pub aggro_race: Vec<String>,
    #[serde(default)]
    pub faction: Option<String>,
    #[serde(default)]
    pub faction_standing: i32,
    #[serde(default)]
    pub trainer_types: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub skills: Vec<MobSkillEntry>,
    #[serde(default)]
    pub scripts: Vec<ScriptHookEntry>,
}

fn default_size() -> String {
    "medium".to_string()
}

fn default_ai_mode() -> String {
    "idle".to_string()
}

// ---------------------------------------------------------------------------
// Item set definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct SetCondition {
    pub piece_type: String,
    #[serde(default = "default_condition_min")]
    pub min: u8,
}

const fn default_condition_min() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetEffect {
    pub effect_type: String,
    #[serde(default)]
    pub stat: Option<String>,
    #[serde(default)]
    pub amount: Option<i32>,
    #[serde(default)]
    pub aura_id: Option<String>,
    #[serde(default)]
    pub radius: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetBonusEntry {
    #[serde(default)]
    pub min_pieces: u8,
    #[serde(default)]
    pub conditions: Vec<SetCondition>,
    #[serde(default)]
    pub effects: Vec<SetEffect>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub bonuses: Vec<SetBonusEntry>,
}

// ---------------------------------------------------------------------------
// Affix definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct AffixDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub affix_type: String,
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub stat: Option<String>,
    #[serde(default)]
    pub quality_min: String,
    #[serde(default)]
    pub slot: Vec<String>,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

const fn default_weight() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Registry — holds all template types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    pub races: HashMap<String, RaceTemplate>,
    pub classes: HashMap<String, ClassTemplate>,
    pub items: HashMap<String, ItemTemplate>,
    pub mobs: HashMap<String, MobTemplate>,
    pub stances: HashMap<String, StanceDef>,
    pub sets: HashMap<String, SetDef>,
    pub affixes: HashMap<String, AffixDef>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        TemplateRegistry::default()
    }

    // ── Race helpers ──

    pub fn get_race(&self, id: &str) -> Option<&RaceTemplate> {
        self.races.get(id)
    }

    pub fn get_class(&self, id: &str) -> Option<&ClassTemplate> {
        self.classes.get(id)
    }

    pub fn available_classes_for_race(&self, race_id: &str) -> Vec<&ClassTemplate> {
        self.classes
            .values()
            .filter(|c| {
                c.allowed_races.is_empty() || c.allowed_races.contains(&race_id.to_string())
            })
            .collect()
    }

    pub fn available_races_for_class(&self, class_id: &str) -> Vec<&RaceTemplate> {
        let class = match self.classes.get(class_id) {
            Some(c) => c,
            None => return Vec::new(),
        };
        self.races
            .values()
            .filter(|r| {
                r.allowed_classes.is_empty()
                    || class.allowed_races.is_empty()
                    || (r.allowed_classes.contains(&class_id.to_string())
                        && class.allowed_races.contains(&r.id.to_string()))
            })
            .collect()
    }

    // ── Item helpers ──

    pub fn get_item(&self, id: &str) -> Option<&ItemTemplate> {
        self.items.get(id)
    }

    // ── Mob helpers ──

    pub fn get_mob(&self, id: &str) -> Option<&MobTemplate> {
        self.mobs.get(id)
    }

    // ── Stance helpers ──

    pub fn get_stance(&self, id: &str) -> Option<&StanceDef> {
        self.stances.get(id)
    }

    // ── Set helpers ──

    pub fn get_set(&self, id: &str) -> Option<&SetDef> {
        self.sets.get(id)
    }

    // ── Affix helpers ──

    pub fn get_affix(&self, id: &str) -> Option<&AffixDef> {
        self.affixes.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_race() -> RaceTemplate {
        RaceTemplate {
            id: "human".into(),
            name: "Human".into(),
            description: "A versatile race.".into(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec!["warrior".into(), "mage".into()],
            racial_abilities: vec!["adaptability".into()],
        }
    }

    fn test_class() -> ClassTemplate {
        ClassTemplate {
            id: "warrior".into(),
            name: "Warrior".into(),
            description: "A master of arms.".into(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods {
                strength: 2,
                constitution: 1,
                ..Default::default()
            },
            allowed_races: vec!["human".into()],
            auto_skills: vec!["power_attack".into(), "shield_bash".into()],
        }
    }

    #[test]
    fn test_get_race() {
        let reg = TemplateRegistry::new();
        let reg = TemplateRegistry {
            races: vec![("human".into(), test_race())].into_iter().collect(),
            ..reg
        };
        assert!(reg.get_race("human").is_some());
        assert!(reg.get_race("elf").is_none());
    }

    #[test]
    fn test_get_class() {
        let mut reg = TemplateRegistry::new();
        reg.classes.insert("warrior".into(), test_class());
        assert!(reg.get_class("warrior").is_some());
        assert!(reg.get_class("mage").is_none());
    }

    #[test]
    fn test_available_classes_for_race() {
        let mut reg = TemplateRegistry::new();
        reg.classes.insert("warrior".into(), test_class());
        let mut mage = test_class();
        mage.id = "mage".into();
        mage.allowed_races = vec!["human".into()];
        reg.classes.insert("mage".into(), mage);
        let mut elf_class = test_class();
        elf_class.id = "elf_only".into();
        elf_class.allowed_races = vec!["elf".into()];
        reg.classes.insert("elf_only".into(), elf_class);

        reg.races.insert("human".into(), test_race());
        let available = reg.available_classes_for_race("human");
        assert_eq!(available.len(), 2);
        assert!(available.iter().any(|c| c.id == "warrior"));
        assert!(available.iter().any(|c| c.id == "mage"));
    }

    #[test]
    fn test_dice_string_deserialize() {
        let toml_str = r#"dice = "2d6+3""#;
        #[derive(Deserialize)]
        struct Wrapper {
            dice: DiceString,
        }
        let w: Wrapper = toml::from_str(toml_str).unwrap();
        assert_eq!(w.dice.as_str(), "2d6+3");
    }

    #[test]
    fn test_dice_string_invalid() {
        let toml_str = r#"dice = "not_dice""#;
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Wrapper {
            dice: DiceString,
        }
        assert!(toml::from_str::<Wrapper>(toml_str).is_err());
    }

    #[test]
    fn test_item_template_defaults() {
        let toml_str = r#"
id = "test_sword"
name = "Test Sword"
description = "A test blade."
item_type = "weapon"
"#;
        let item: ItemTemplate = toml::from_str(toml_str).unwrap();
        assert_eq!(item.quality, "common");
        assert_eq!(item.level_requirement, 0);
        assert!(item.weapon.is_none());
        assert!(item.equipment.is_none());
    }

    #[test]
    fn test_mob_template_parse() {
        let toml_str = r#"
id = "goblin"
name = "goblin"
description = "A goblin."
health = { current = 20, max = 20 }
"#;
        let mob: MobTemplate = toml::from_str(toml_str).unwrap();
        assert_eq!(mob.id, "goblin");
        assert_eq!(mob.health.max, 20);
        assert_eq!(mob.ai_mode, "idle");
        assert_eq!(mob.size, "medium");
    }

    #[test]
    fn test_stance_def_parse() {
        let toml_str = r#"
id = "defensive"
name = "Defensive Stance"
ac_bonus = 2
attack_penalty = -2
"#;
        let stance: StanceDef = toml::from_str(toml_str).unwrap();
        assert_eq!(stance.ac_bonus, 2);
        assert_eq!(stance.attack_penalty, -2);
        assert_eq!(stance.min_level, 1);
    }

    #[test]
    fn test_set_def_parse() {
        let toml_str = r#"
id = "templar_armor"
name = "Templar Armor Set"
[[bonuses]]
min_pieces = 2
effects = [{ effect_type = "stat", stat = "constitution", amount = 2 }]
"#;
        let set: SetDef = toml::from_str(toml_str).unwrap();
        assert_eq!(set.id, "templar_armor");
        assert_eq!(set.bonuses.len(), 1);
        assert_eq!(set.bonuses[0].min_pieces, 2);
    }

    #[test]
    fn test_class_attribute_mods_default() {
        let m = ClassAttributeMods::default();
        assert_eq!(m.strength, 0);
    }

    #[test]
    fn test_race_defaults() {
        let r = RaceTemplate {
            id: "test".into(),
            name: "Test".into(),
            description: "Desc.".into(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec![],
            racial_abilities: vec![],
        };
        assert_eq!(r.attributes.strength, 10);
        assert!(r.allowed_classes.is_empty());
    }

    #[test]
    fn test_available_races_for_class() {
        let mut reg = TemplateRegistry::new();
        let mut elf_race = test_race();
        elf_race.id = "elf".into();
        elf_race.allowed_classes = vec!["mage".into()];
        reg.races.insert("elf".into(), elf_race);
        reg.races.insert("human".into(), test_race());

        reg.classes.insert("warrior".into(), test_class());
        let available = reg.available_races_for_class("warrior");
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id, "human");
    }
}
