use crate::components::CombatStats;
use crate::dice::DiceRoll;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Dice roll helper for TOML — stored as string, parsed at use time
// ---------------------------------------------------------------------------

/// Custom deserialize for dice notation strings like "2d6" or "2d8+3".
/// Accepts a plain string in TOML: `dice = "2d6+3"`
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenderPronouns {
    pub subject: String,
    pub object: String,
    pub possessive: String,
}

impl Default for GenderPronouns {
    fn default() -> Self {
        GenderPronouns {
            subject: "they".into(),
            object: "them".into(),
            possessive: "their".into(),
        }
    }
}

pub fn default_genders() -> Vec<String> {
    vec!["male".into(), "female".into(), "neutral".into()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceBounds {
    pub height_min: u8,
    pub height_max: u8,
    pub weight_min: u16,
    pub weight_max: u16,
    pub allowed_builds: Vec<String>,
    pub allowed_hair_colors: Vec<String>,
    pub allowed_eye_colors: Vec<String>,
    pub allowed_skin_tones: Vec<String>,
}

impl Default for AppearanceBounds {
    fn default() -> Self {
        AppearanceBounds {
            height_min: 54,
            height_max: 80,
            weight_min: 90,
            weight_max: 350,
            allowed_builds: vec![
                "slim".into(),
                "average".into(),
                "athletic".into(),
                "stocky".into(),
            ],
            allowed_hair_colors: vec![
                "black".into(),
                "brown".into(),
                "blonde".into(),
                "red".into(),
                "white".into(),
                "gray".into(),
            ],
            allowed_eye_colors: vec![
                "brown".into(),
                "blue".into(),
                "green".into(),
                "hazel".into(),
                "gray".into(),
            ],
            allowed_skin_tones: vec![
                "fair".into(),
                "light".into(),
                "olive".into(),
                "tan".into(),
                "brown".into(),
                "dark".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub attributes: RaceAttributes,
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    #[serde(default)]
    pub allowed_alignments: Vec<String>,
    #[serde(default)]
    pub racial_abilities: Vec<String>,
    #[serde(default)]
    pub allowed_genders: HashMap<String, GenderPronouns>,
    #[serde(default)]
    pub appearance_bounds: AppearanceBounds,
    #[serde(default = "default_age")]
    pub age_default: u16,
    #[serde(default = "default_age_max")]
    pub age_max: u16,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

const fn default_age() -> u16 {
    20
}

const fn default_age_max() -> u16 {
    100
}

// ---------------------------------------------------------------------------
// Class template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WalletAmount {
    #[serde(default)]
    pub copper: u64,
    #[serde(default)]
    pub silver: u64,
    #[serde(default)]
    pub gold: u64,
    #[serde(default)]
    pub platinum: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrestigeGate {
    #[serde(default)]
    pub requires_class: HashMap<String, u8>,
    #[serde(default)]
    pub requires_skills: HashMap<String, u16>,
    pub requires_race: Option<String>,
    pub requires_alignment: Option<String>,
    pub requires_quest: Option<String>,
    pub requires_faction: Option<String>,
    pub requires_level: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub prestige: bool,
    #[serde(default)]
    pub prestige_gate: Option<PrestigeGate>,
    #[serde(default = "default_hit_die")]
    pub hit_die: u8,
    #[serde(default)]
    pub attribute_mods: ClassAttributeMods,
    #[serde(default = "default_bab")]
    pub bab: String,
    #[serde(default = "default_save_progression")]
    pub fort_save: String,
    #[serde(default = "default_save_progression")]
    pub ref_save: String,
    #[serde(default = "default_save_progression")]
    pub will_save: String,
    #[serde(default)]
    pub allowed_races: Vec<String>,
    #[serde(default)]
    pub allowed_alignments: Vec<String>,
    #[serde(default)]
    pub auto_skills: Vec<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
    #[serde(default)]
    pub skill_pool: Vec<String>,
    #[serde(default = "default_starting_skill_slots")]
    pub starting_skill_slots: u8,
    #[serde(default)]
    pub starting_items: Vec<String>,
    #[serde(default)]
    pub starting_gold: WalletAmount,
    #[serde(default = "default_deity_policy")]
    pub deity_policy: DeityPolicy,
}

impl ClassTemplate {
    pub fn calculate_combat_stats(&self, level: u8) -> CombatStats {
        let bab = match self.bab.as_str() {
            "full" => level as i32,
            "medium" => ((level as i32) * 3) / 4,
            "poor" => (level as i32) / 2,
            _ => 0,
        };
        let fort = match self.fort_save.as_str() {
            "good" => 2 + (level as i32) / 2,
            "poor" => (level as i32) / 3,
            _ => 0,
        };
        let reflex = match self.ref_save.as_str() {
            "good" => 2 + (level as i32) / 2,
            "poor" => (level as i32) / 3,
            _ => 0,
        };
        let will = match self.will_save.as_str() {
            "good" => 2 + (level as i32) / 2,
            "poor" => (level as i32) / 3,
            _ => 0,
        };
        CombatStats {
            base_attack_bonus: bab,
            fort_save: fort,
            ref_save: reflex,
            will_save: will,
        }
    }
}

const fn default_starting_skill_slots() -> u8 {
    3
}

const fn default_hit_die() -> u8 {
    8
}

fn default_bab() -> String {
    "poor".to_string()
}

fn default_save_progression() -> String {
    "poor".to_string()
}

fn default_deity_policy() -> DeityPolicy {
    DeityPolicy::Any
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeityPolicy {
    Any,
    None,
    Required,
    Subset(Vec<String>),
}

impl<'de> Deserialize<'de> for DeityPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DeityPolicyVisitor;
        impl<'de> serde::de::Visitor<'de> for DeityPolicyVisitor {
            type Value = DeityPolicy;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("a string ('any', 'none', 'required') or a map with a 'subset' key")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "any" => Ok(DeityPolicy::Any),
                    "none" => Ok(DeityPolicy::None),
                    "required" => Ok(DeityPolicy::Required),
                    _ => Err(serde::de::Error::custom(format!(
                        "invalid deity policy string: {}",
                        value
                    ))),
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut subset = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "subset" {
                        subset = Some(map.next_value::<Vec<String>>()?);
                    } else {
                        let _ = map.next_value::<serde::de::IgnoredAny>()?;
                    }
                }
                if let Some(list) = subset {
                    Ok(DeityPolicy::Subset(list))
                } else {
                    Err(serde::de::Error::custom(
                        "missing 'subset' key for deity policy",
                    ))
                }
            }
        }

        deserializer.deserialize_any(DeityPolicyVisitor)
    }
}

impl Serialize for DeityPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            DeityPolicy::Any => serializer.serialize_str("any"),
            DeityPolicy::None => serializer.serialize_str("none"),
            DeityPolicy::Required => serializer.serialize_str("required"),
            DeityPolicy::Subset(list) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("subset", list)?;
                map.end()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrayerEffect {
    pub buff_id: String,
    pub duration_secs: u64,
    pub cooldown_secs: u64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeityTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub alignment: Option<String>,
    pub symbol: String,
    pub favored_weapon: Option<String>,
    #[serde(default)]
    pub tenets: Vec<String>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub allowed_races: Vec<String>,
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    #[serde(default)]
    pub allowed_alignments: Vec<String>,
    pub prayer_effect: Option<PrayerEffect>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub level_requirement: u8,
    #[serde(default)]
    pub repeatable: bool,
    #[serde(default)]
    pub auto_complete: bool,
    pub giver_npc: Option<String>,
    pub turn_in_npc: Option<String>,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    pub objectives: Vec<QuestObjective>,
    pub rewards: QuestRewards,
    #[serde(default)]
    pub scripts: Option<QuestScripts>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum QuestObjective {
    Kill { mob: String, count: u32 },
    Gather { item: String, count: u32 },
    Deliver { item: String, npc: String },
    Explore { room: String },
    Talk { npc: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuestRewards {
    #[serde(default)]
    pub xp: u64,
    #[serde(default)]
    pub gold: u64,
    #[serde(default)]
    pub items: Vec<QuestRewardItem>,
    #[serde(default)]
    pub faction: Vec<QuestRewardFaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestRewardItem {
    pub item_template_id: String,
    #[serde(default = "default_reward_count")]
    pub count: u32,
}

fn default_reward_count() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestRewardFaction {
    pub faction_id: String,
    pub amount: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuestScripts {
    pub on_accept: Option<String>,
    pub on_update: Option<String>,
    pub on_complete: Option<String>,
}

// ---------------------------------------------------------------------------
// Faction template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub starting_standing: i32,
    #[serde(default = "default_min_standing")]
    pub min_standing: i32,
    #[serde(default = "default_max_standing")]
    pub max_standing: i32,
    pub ranks: Vec<FactionRank>,
    #[serde(default)]
    pub relationships: HashMap<String, f32>,
    pub aggro_below: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionRank {
    pub name: String,
    pub threshold: i32,
}

fn default_min_standing() -> i32 {
    -10000
}

fn default_max_standing() -> i32 {
    10000
}

impl FactionDef {
    pub fn get_rank(&self, standing: i32) -> String {
        let mut best_rank = "Neutral".to_string();
        let mut max_threshold = i32::MIN;

        for rank in &self.ranks {
            if standing >= rank.threshold && rank.threshold >= max_threshold {
                best_rank = rank.name.clone();
                max_threshold = rank.threshold;
            }
        }
        best_rank
    }
}

// ---------------------------------------------------------------------------
// Recipe template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub station: Option<String>,
    pub skill_requirement: Option<RecipeSkillReq>,
    pub difficulty: u32,
    pub materials: Vec<RecipeMaterial>,
    pub result: RecipeResult,
    pub success_chance: u8,
    #[serde(default)]
    pub quality_scaling: bool,
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSkillReq {
    pub id: String,
    pub rank: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMaterial {
    pub template_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeResult {
    pub template_id: String,
    pub quantity: u32,
}

// ---------------------------------------------------------------------------
// Stance template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub params: HashMap<String, String>,
}

const fn default_min_level() -> u8 {
    1
}

// ---------------------------------------------------------------------------
// Item template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponDef {
    pub damage: DiceString,
    pub damage_type: String,
    #[serde(default = "default_weapon_speed")]
    pub speed: f32,
    #[serde(default = "default_weapon_range")]
    pub range: String,
    #[serde(default = "default_weapon_hands")]
    pub hands: String,
}

fn default_weapon_speed() -> f32 {
    2.5
}

fn default_weapon_hands() -> String {
    "one_hand".to_string()
}

fn default_weapon_range() -> String {
    "melee".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentDef {
    pub slot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirement {
    pub id: String,
    #[serde(default)]
    pub level: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDef {
    pub event: String,
    #[serde(default)]
    pub chance: u8,
    /// Skill/spell to cast when the trigger fires. Optional for script-driven
    /// triggers (`script` set) where the script performs its own effects.
    #[serde(default)]
    pub cast: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMembership {
    pub id: String,
    pub piece_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumableDef {
    #[serde(default)]
    pub kind: String,
    #[serde(default = "default_one_u16")]
    pub charges: u16,
    #[serde(default = "default_one_u16")]
    pub max_charges: u16,
    #[serde(default)]
    pub effect_script: Option<String>,
    #[serde(default)]
    pub restore_health: i32,
    #[serde(default)]
    pub restore_mana: i32,
    #[serde(default)]
    pub restore_stamina: i32,
    #[serde(default)]
    pub depleted_template: Option<String>,
    #[serde(default)]
    pub replenishable: bool,
    #[serde(default)]
    pub liquid_type: Option<String>,
}

fn default_one_u16() -> u16 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDef {
    #[serde(default)]
    pub capacity_weight: f32,
    #[serde(default)]
    pub max_items: u16,
    #[serde(default)]
    pub weight_reduction_pct: u8,
    #[serde(default)]
    pub is_drink_container: bool,
    #[serde(default)]
    pub liquid_type: Option<String>,
    #[serde(default)]
    pub liquid_charges: u16,
    #[serde(default)]
    pub max_liquid_charges: u16,
    #[serde(default)]
    pub is_closed: bool,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub key_template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurabilityDef {
    pub max: u16,
    #[serde(default = "default_decay_rate")]
    pub decay_rate: f32,
}

fn default_decay_rate() -> f32 {
    1.0
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub item_type: String,
    #[serde(default)]
    pub subtype: String,
    /// Rarity tier (drop rates, affix budget, display color). Values:
    /// common, uncommon, rare, epic, legendary, artifact.
    #[serde(default = "default_rarity")]
    pub rarity: String,
    /// Craftsmanship quality (flavor/price modifier). Values: poor, standard,
    /// fine, masterwork. Distinct from `rarity` — a masterwork common item and
    /// a shoddy rare item are both valid.
    #[serde(default = "default_item_quality")]
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
    pub consumable: Option<ConsumableDef>,
    #[serde(default)]
    pub container: Option<ContainerDef>,
    #[serde(default)]
    pub durability: Option<DurabilityDef>,
    #[serde(default)]
    pub triggers: Vec<TriggerDef>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

fn default_rarity() -> String {
    "common".to_string()
}

fn default_item_quality() -> String {
    "standard".to_string()
}

// ---------------------------------------------------------------------------
// Mob template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthBounds {
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobEquipmentEntry {
    pub template_id: String,
    pub slot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LootTable {
    #[serde(default)]
    pub entries: Vec<LootEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobSkillEntry {
    pub id: String,
    #[serde(default)]
    pub level: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptHookEntry {
    pub event: String,
    pub script: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub short_desc: String,
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
    pub patrol_route: Vec<String>,
    #[serde(default)]
    pub wander_rooms: Vec<String>,
    #[serde(default)]
    pub wander_area: bool,
    #[serde(default)]
    pub aggro_range: u32,
    #[serde(default)]
    pub aggro_players: bool,
    #[serde(default)]
    pub aggro_mobs: bool,
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
    pub shop: Option<String>,
    #[serde(default)]
    pub friendly: bool,
    #[serde(default)]
    pub banker: bool,
    #[serde(default)]
    pub scripts: Vec<ScriptHookEntry>,
    #[serde(default)]
    pub params: HashMap<String, String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCondition {
    pub piece_type: String,
    #[serde(default = "default_condition_min")]
    pub min: u8,
}

const fn default_condition_min() -> u8 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBonusEntry {
    #[serde(default)]
    pub min_pieces: u8,
    #[serde(default)]
    pub conditions: Vec<SetCondition>,
    #[serde(default)]
    pub effects: Vec<SetEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub bonuses: Vec<SetBonusEntry>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Passive definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveEffect {
    pub effect_type: String,
    pub target: String,
    #[serde(default)]
    pub amount: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub effects: Vec<PassiveEffect>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Affix definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub params: HashMap<String, String>,
}

const fn default_weight() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Area / Room template types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobSpawnEntry {
    pub template_id: String,
    #[serde(default = "default_mob_count")]
    pub count: u8,
    #[serde(default)]
    pub respawn_secs: Option<u64>,
}

const fn default_mob_count() -> u8 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSpawnEntry {
    pub template_id: String,
    #[serde(default = "default_item_count")]
    pub count: u8,
}

const fn default_item_count() -> u8 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomContent {
    #[serde(default)]
    pub mobs: Vec<MobSpawnEntry>,
    #[serde(default)]
    pub items: Vec<ItemSpawnEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ExitTemplate {
    Simple(String),
    Detailed {
        dest: String,
        #[serde(default)]
        door: bool,
        #[serde(default)]
        closed: bool,
        #[serde(default)]
        locked: bool,
        key_id: Option<String>,
    },
}

impl ExitTemplate {
    pub fn dest(&self) -> &str {
        match self {
            ExitTemplate::Simple(d) => d.as_str(),
            ExitTemplate::Detailed { dest, .. } => dest.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTemplate {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub area: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub exits: HashMap<String, ExitTemplate>,
    #[serde(default)]
    pub portals: Vec<RoomPortalTemplate>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub content: RoomContent,
    #[serde(default)]
    pub allow_revive: bool,
    #[serde(default)]
    pub no_weather: bool,
    #[serde(default)]
    pub exclude_weather: Vec<String>,
    #[serde(default)]
    pub additional_weather: HashMap<String, u32>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPortalTemplate {
    pub keyword: String,
    pub dest: String,
    pub description: String,
    #[serde(default)]
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetInterval {
    pub secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntry {
    pub room: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub allowed_races: Vec<String>,
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    #[serde(default)]
    pub allowed_alignments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub level_range: Option<[u8; 2]>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub weather_zone: Option<String>,
    #[serde(default)]
    pub no_weather: bool,
    #[serde(default)]
    pub weather_matrix: HashMap<String, HashMap<String, u32>>,
    #[serde(default)]
    pub reset_interval: Option<ResetInterval>,
    #[serde(default)]
    pub credits: Option<String>,
    #[serde(default)]
    pub spawns: Vec<SpawnEntry>,
    #[serde(default)]
    pub rooms: HashMap<String, RoomTemplate>,
}

// ---------------------------------------------------------------------------
// Shop template types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopInventoryCount {
    pub min: u64,
    pub max: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopInventoryEntry {
    pub item: String,
    pub count: ShopInventoryCount,
    pub price: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopTemplate {
    pub id: String,
    pub name: String,
    pub buy_rate: f64,
    pub sell_rate: f64,
    pub restock_secs: u64,
    #[serde(default)]
    pub inventory: Vec<ShopInventoryEntry>,
    #[serde(default)]
    pub buy_types: Vec<String>,
    #[serde(default)]
    pub price_mods: HashMap<String, f64>,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Validation error & Skill resolve error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub template_type: &'static str,
    pub template_id: String,
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {} — {}",
            self.template_type, self.template_id, self.field, self.message
        )
    }
}

// ---------------------------------------------------------------------------
// Channel scope — how far a channel's message propagates
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelScope {
    /// Sender's room only.
    Room,
    /// Room plus up to N hops via the exit graph (respects doors + silent rooms).
    Adjacent(u32),
    /// All rooms within the same area (respects doors + silent rooms).
    Area,
    /// All connected players regardless of location.
    #[default]
    Global,
}

// ---------------------------------------------------------------------------
// Channel definition — runtime config for chat channels
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDef {
    pub id: String,
    pub name: String,
    /// Single-character prefix for shortcut dispatch, e.g. "=", ";", "[".
    /// Empty string means no shortcut.
    pub shortcut: String,
    /// Command aliases (e.g. "goss", "newb", "auc").
    pub aliases: Vec<String>,
    /// Format template with {player} and {message} interpolation.
    /// {player} renders as "You" for sender, player name for others.
    /// Include color tags directly, e.g. "{yellow}[OOC]{/} {player}: {message}"
    pub format: String,
    /// If true, skip ghost text formatting (OOC channels).
    pub is_ooc: bool,
    /// Minimum level allowed to send on this channel (1 = everyone).
    pub min_level_send: u8,
    /// Maximum level allowed to send on this channel (0 = unlimited).
    pub max_level_send: u8,
    /// Whether the channel is enabled by default for new characters.
    pub default_enabled: bool,
    /// Cooldown between sends in seconds (0 = no cooldown).
    pub cooldown_secs: u64,
    /// Scope/range of the channel (default: Global).
    pub scope: ChannelScope,
}

impl ChannelDef {
    pub fn render(&self, player_name: &str, message: &str, for_sender: bool) -> String {
        let p = if for_sender { "You" } else { player_name };
        self.format
            .replace("{player}", p)
            .replace("{message}", message)
    }
}

pub fn default_channel_defs() -> Vec<ChannelDef> {
    vec![
        ChannelDef {
            id: "ooc".into(),
            name: "OOC".into(),
            shortcut: "=".into(),
            aliases: vec![],
            format: "{yellow}[OOC]{/} {player}: {message}".into(),
            is_ooc: true,
            min_level_send: 1,
            max_level_send: 0,
            default_enabled: true,
            cooldown_secs: 0,
            scope: ChannelScope::Global,
        },
        ChannelDef {
            id: "gossip".into(),
            name: "Gossip".into(),
            shortcut: ";".into(),
            aliases: vec!["goss".into()],
            format: "{green}[Gossip]{/} {player}: {message}".into(),
            is_ooc: true,
            min_level_send: 1,
            max_level_send: 0,
            default_enabled: true,
            cooldown_secs: 0,
            scope: ChannelScope::Global,
        },
        ChannelDef {
            id: "newbie".into(),
            name: "Newbie".into(),
            shortcut: "[".into(),
            aliases: vec!["newb".into()],
            format: "{cyan}[Newbie]{/} {player}: {message}".into(),
            is_ooc: true,
            min_level_send: 1,
            max_level_send: 5,
            default_enabled: true,
            cooldown_secs: 0,
            scope: ChannelScope::Global,
        },
        ChannelDef {
            id: "auction".into(),
            name: "Auction".into(),
            shortcut: String::new(),
            aliases: vec!["auc".into()],
            format: "{magenta}[Auction]{/} {player}: {message}".into(),
            is_ooc: true,
            min_level_send: 1,
            max_level_send: 0,
            default_enabled: true,
            cooldown_secs: 0,
            scope: ChannelScope::Global,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialDef {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub char_no_target: Option<String>,
    #[serde(default)]
    pub room_no_target: Option<String>,
    #[serde(default)]
    pub char_self: Option<String>,
    #[serde(default)]
    pub room_self: Option<String>,
    #[serde(default)]
    pub char_target: Option<String>,
    #[serde(default)]
    pub room_target: Option<String>,
    #[serde(default)]
    pub target_char: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SkillResolveError {
    NotFound,
    Multiple(Vec<(String, String)>),
}

impl std::fmt::Display for SkillResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillResolveError::NotFound => write!(f, "no matching skill found"),
            SkillResolveError::Multiple(candidates) => {
                let names: Vec<&str> = candidates.iter().map(|(id, _)| id.as_str()).collect();
                write!(f, "multiple skills match: {}", names.join(", "))
            }
        }
    }
}
