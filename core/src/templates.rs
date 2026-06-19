use crate::components::SkillDef;
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

// ---------------------------------------------------------------------------
// Gender definition — embedded in race templates
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Appearance bounds — embedded in race templates
// ---------------------------------------------------------------------------

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
    /// Gender id → pronoun definitions. Built-in genders (male/female/neutral)
    /// have default pronouns if not explicitly defined. Custom genders require
    /// explicit pronouns.
    #[serde(default)]
    pub allowed_genders: HashMap<String, GenderPronouns>,
    /// Bounds for structured appearance fields.
    #[serde(default)]
    pub appearance_bounds: AppearanceBounds,
    /// Default starting age for this race.
    #[serde(default = "default_age")]
    pub age_default: u16,
    /// Maximum natural age for this race.
    #[serde(default = "default_age_max")]
    pub age_max: u16,
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

/// Currency amounts used in template definitions (starting gold, etc.)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub allowed_alignments: Vec<String>,
    #[serde(default)]
    pub auto_skills: Vec<String>,
    #[serde(default)]
    pub skill_pool: Vec<String>,
    #[serde(default = "default_starting_skill_slots")]
    pub starting_skill_slots: u8,
    #[serde(default)]
    pub starting_items: Vec<String>,
    #[serde(default)]
    pub starting_gold: WalletAmount,
}

const fn default_starting_skill_slots() -> u8 {
    3
}

const fn default_hit_die() -> u8 {
    8
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
}

fn default_weapon_speed() -> f32 {
    2.5
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
    pub cast: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMembership {
    pub id: String,
    pub piece_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

// ---------------------------------------------------------------------------
// Passive definitions — data-driven racial/class/item passives
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTemplate {
    /// Room ID (used when serialized as a standalone room file).
    #[serde(default)]
    pub id: String,
    /// Parent area ID (used when serialized as a standalone room file).
    #[serde(default)]
    pub area: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub exits: HashMap<String, String>,
    #[serde(default)]
    pub portals: Vec<RoomPortalTemplate>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub content: RoomContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPortalTemplate {
    pub keyword: String,
    pub dest: String,
    pub description: String,
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Room reset (spawn) timer settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetInterval {
    /// How often the area's rooms reset in seconds.
    pub secs: u64,
}

/// A character spawn point within an area.
///
/// Empty constraint vectors mean "no restriction" — any race/class/alignment
/// can choose this spawn. The area's `spawn_room` is implicitly included as
/// a fallback spawn when no explicit spawns are defined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntry {
    /// Room ID within this area.
    pub room: String,
    /// Short display label shown in the spawn-selection prompt.
    pub label: String,
    /// Flavor description shown when inspecting this spawn option.
    pub description: String,
    /// Races allowed to spawn here (empty = all).
    #[serde(default)]
    pub allowed_races: Vec<String>,
    /// Classes allowed to spawn here (empty = all).
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    /// Alignments allowed to spawn here (empty = all).
    #[serde(default)]
    pub allowed_alignments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Room ID used as the fallback spawn point for this area
    /// (used when no explicit spawn matches or on recall).
    pub spawn_room: String,
    #[serde(default)]
    pub level_range: Option<[u8; 2]>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub weather_zone: Option<String>,
    #[serde(default)]
    pub reset_interval: Option<ResetInterval>,
    #[serde(default)]
    pub credits: Option<String>,
    /// Explicit spawn entries with optional race/class/alignment constraints.
    #[serde(default)]
    pub spawns: Vec<SpawnEntry>,
    /// Map of room_id → RoomTemplate
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
}

// ---------------------------------------------------------------------------
// Validation error
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

/// Error from `TemplateRegistry::resolve_skill`.
#[derive(Debug, Clone)]
pub enum SkillResolveError {
    /// No skill matched the input.
    NotFound,
    /// Multiple skills matched; each entry is `(skill_id, display_name)`.
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

// ---------------------------------------------------------------------------
// Derived indices — pre-computed lookup tables
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DerivedIndices {
    /// Set ID → item template IDs that belong to it
    pub items_by_set: HashMap<String, Vec<String>>,
    /// Equipment slot name → item template IDs for that slot
    pub items_by_slot: HashMap<String, Vec<String>>,
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
    pub passives: HashMap<String, PassiveDef>,
    pub areas: HashMap<String, AreaTemplate>,
    pub skills: HashMap<String, SkillDef>,
    pub shops: HashMap<String, ShopTemplate>,
    pub indices: DerivedIndices,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        TemplateRegistry::default()
    }

    /// Validate all templates and return any errors found.
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate race ↔ class cross-references
        for (id, race) in &self.races {
            for class_id in &race.allowed_classes {
                if !self.classes.contains_key(class_id) {
                    errors.push(ValidationError {
                        template_type: "race",
                        template_id: id.clone(),
                        field: "allowed_classes".into(),
                        message: format!("references unknown class template: {class_id}"),
                    });
                }
            }
        }

        for (id, class) in &self.classes {
            for race_id in &class.allowed_races {
                if !self.races.contains_key(race_id) {
                    errors.push(ValidationError {
                        template_type: "class",
                        template_id: id.clone(),
                        field: "allowed_races".into(),
                        message: format!("references unknown race template: {race_id}"),
                    });
                }
            }
        }

        // Validate items
        for (id, item) in &self.items {
            // Set membership
            if let Some(set) = &item.set {
                if !self.sets.contains_key(&set.id) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "set.id".into(),
                        message: format!("references unknown set: {}", set.id),
                    });
                }
            }

            // Skill requirement — validate once a master skill registry exists
            if let Some(_req) = &item.requires_skill {}

            // Allowed classes
            for class_id in &item.allowed_classes {
                if !self.classes.contains_key(class_id) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "allowed_classes".into(),
                        message: format!("references unknown class: {class_id}"),
                    });
                }
            }

            // Allowed races
            for race_id in &item.allowed_races {
                if !self.races.contains_key(race_id) {
                    errors.push(ValidationError {
                        template_type: "item",
                        template_id: id.clone(),
                        field: "allowed_races".into(),
                        message: format!("references unknown race: {race_id}"),
                    });
                }
            }
        }

        // Validate mobs
        for (id, mob) in &self.mobs {
            for eq_entry in &mob.equipment {
                if !self.items.contains_key(&eq_entry.template_id) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "equipment".into(),
                        message: format!(
                            "references unknown item template: {}",
                            eq_entry.template_id
                        ),
                    });
                }
            }

            for entry in &mob.loot.entries {
                if !entry.item.is_empty() && !self.items.contains_key(&entry.item) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "loot".into(),
                        message: format!("references unknown item template: {}", entry.item),
                    });
                }
            }

            if let Some(ref race_id) = mob.race {
                if !self.races.contains_key(race_id) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "race".into(),
                        message: format!("references unknown race: {race_id}"),
                    });
                }
            }

            if let Some(ref shop_id) = mob.shop {
                if !self.shops.contains_key(shop_id) {
                    errors.push(ValidationError {
                        template_type: "mob",
                        template_id: id.clone(),
                        field: "shop".into(),
                        message: format!("references unknown shop: {shop_id}"),
                    });
                }
            }
        }

        // Validate passives referenced by race/class/items
        for race in self.races.values() {
            for ability in &race.racial_abilities {
                if !self.passives.contains_key(ability) {
                    errors.push(ValidationError {
                        template_type: "race",
                        template_id: race.id.clone(),
                        field: "racial_abilities".into(),
                        message: format!("references unknown passive: {ability}"),
                    });
                }
            }
        }

        // Validate areas and their rooms
        for (area_id, area) in &self.areas {
            if !area.rooms.contains_key(&area.spawn_room) {
                errors.push(ValidationError {
                    template_type: "area",
                    template_id: area_id.clone(),
                    field: "spawn_room".into(),
                    message: format!(
                        "references unknown room '{}' in area '{}'",
                        area.spawn_room, area_id
                    ),
                });
            }
            for (room_id, room) in &area.rooms {
                for dest in room.exits.values() {
                    if let Some((target_area, target_room)) = dest.split_once(':') {
                        if !self.areas.contains_key(target_area) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.exits"),
                                message: format!("references unknown area '{target_area}'"),
                            });
                        } else if !self.room_exists(target_area, target_room) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.exits"),
                                message: format!(
                                    "references unknown room '{target_room}' in area '{target_area}'"
                                ),
                            });
                        }
                    } else if !area.rooms.contains_key(dest) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.exits"),
                            message: format!(
                                "references unknown room '{dest}' in area '{area_id}'"
                            ),
                        });
                    }
                }
                for portal in &room.portals {
                    if let Some((target_area, target_room)) = portal.dest.split_once(':') {
                        if !self.areas.contains_key(target_area) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.portals"),
                                message: format!("references unknown area '{target_area}'"),
                            });
                        } else if !self.room_exists(target_area, target_room) {
                            errors.push(ValidationError {
                                template_type: "area",
                                template_id: area_id.clone(),
                                field: format!("rooms.{room_id}.portals"),
                                message: format!(
                                    "references unknown room '{target_room}' in area '{target_area}'"
                                ),
                            });
                        }
                    } else if !area.rooms.contains_key(&portal.dest) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.portals"),
                            message: format!(
                                "references unknown room '{}' in area '{}'",
                                portal.dest, area_id
                            ),
                        });
                    }
                }
                for entry in &room.content.mobs {
                    if !self.mobs.contains_key(&entry.template_id) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.content.mobs"),
                            message: format!(
                                "references unknown mob template '{}'",
                                entry.template_id
                            ),
                        });
                    }
                }
                for entry in &room.content.items {
                    if !self.items.contains_key(&entry.template_id) {
                        errors.push(ValidationError {
                            template_type: "area",
                            template_id: area_id.clone(),
                            field: format!("rooms.{room_id}.content.items"),
                            message: format!(
                                "references unknown item template '{}'",
                                entry.template_id
                            ),
                        });
                    }
                }
            }
            // Validate spawn entries
            for (i, spawn) in area.spawns.iter().enumerate() {
                if !area.rooms.contains_key(&spawn.room) {
                    errors.push(ValidationError {
                        template_type: "area",
                        template_id: area_id.clone(),
                        field: format!("spawns[{i}].room"),
                        message: format!(
                            "references unknown room '{}' in area '{}'",
                            spawn.room, area_id
                        ),
                    });
                }
            }
        }

        errors
    }

    /// Build derived indices from all loaded templates.
    pub fn build_indices(&mut self) {
        let mut items_by_set: HashMap<String, Vec<String>> = HashMap::new();
        let mut items_by_slot: HashMap<String, Vec<String>> = HashMap::new();

        for (id, item) in &self.items {
            // Items by set
            if let Some(set) = &item.set {
                items_by_set
                    .entry(set.id.clone())
                    .or_default()
                    .push(id.clone());
            }

            // Items by equipment slot
            if let Some(eq) = &item.equipment {
                items_by_slot
                    .entry(eq.slot.clone())
                    .or_default()
                    .push(id.clone());
            }
        }

        self.indices = DerivedIndices {
            items_by_set,
            items_by_slot,
        };
    }

    // ── Race helpers ──

    pub fn get_race(&self, id: &str) -> Option<&RaceTemplate> {
        self.races.get(id)
    }

    pub fn get_class(&self, id: &str) -> Option<&ClassTemplate> {
        self.classes.get(id)
    }

    pub fn available_classes_for_race(&self, race_id: &str) -> Vec<&ClassTemplate> {
        let race = match self.races.get(race_id) {
            Some(r) => r,
            None => return Vec::new(),
        };
        let mut classes: Vec<&ClassTemplate> = self
            .classes
            .values()
            .filter(|c| {
                (race.allowed_classes.is_empty() || race.allowed_classes.contains(&c.id))
                    && (c.allowed_races.is_empty()
                        || c.allowed_races.contains(&race_id.to_string()))
            })
            .collect();
        classes.sort_by(|a, b| a.id.cmp(&b.id));
        classes
    }

    pub fn available_races_for_class(&self, class_id: &str) -> Vec<&RaceTemplate> {
        let class = match self.classes.get(class_id) {
            Some(c) => c,
            None => return Vec::new(),
        };
        let mut races: Vec<&RaceTemplate> = self
            .races
            .values()
            .filter(|r| {
                r.allowed_classes.is_empty()
                    || class.allowed_races.is_empty()
                    || (r.allowed_classes.contains(&class_id.to_string())
                        && class.allowed_races.contains(&r.id.to_string()))
            })
            .collect();
        races.sort_by(|a, b| a.id.cmp(&b.id));
        races
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

    // ── Passive helpers ──

    pub fn get_passive(&self, id: &str) -> Option<&PassiveDef> {
        self.passives.get(id)
    }

    pub fn passives_for_race(&self, race_id: &str) -> Vec<&PassiveDef> {
        self.races
            .get(race_id)
            .map(|race| {
                race.racial_abilities
                    .iter()
                    .filter_map(|id| self.passives.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── Skill helpers ──

    pub fn get_skill(&self, id: &str) -> Option<&SkillDef> {
        self.skills.get(id)
    }

    /// Resolve a partial or exact skill name to a unique skill ID.
    ///
    /// Matches against both `SkillDef.id` and `SkillDef.name` (case-insensitive).
    /// If `pool` is provided, only searches within the given set of skill IDs.
    ///
    /// Resolution priority:
    ///   1. Exact match on id or name
    ///   2. Unique prefix match on id
    ///   3. Unique prefix match on name
    ///   4. Multiple matches → `SkillResolveError::Multiple(candidates)`
    ///   5. No match → `SkillResolveError::NotFound`
    pub fn resolve_skill(
        &self,
        input: &str,
        pool: Option<&[String]>,
    ) -> Result<String, SkillResolveError> {
        let input_lower = input.to_lowercase();

        let candidates: Vec<&SkillDef> = if let Some(pool) = pool {
            pool.iter()
                .filter_map(|id| self.skills.get(id.as_str()))
                .collect()
        } else {
            self.skills.values().collect()
        };

        // Priority 1: exact match on id or name
        for skill in &candidates {
            if skill.id.to_lowercase() == input_lower || skill.name.to_lowercase() == input_lower {
                return Ok(skill.id.clone());
            }
        }

        // Priority 2 & 3: prefix match on id or name, deduplicated
        let mut matches: Vec<&SkillDef> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        for skill in candidates {
            if seen_ids.insert(skill.id.as_str())
                && (skill.id.to_lowercase().starts_with(&input_lower)
                    || skill.name.to_lowercase().starts_with(&input_lower))
            {
                matches.push(skill);
            }
        }

        match matches.len() {
            0 => Err(SkillResolveError::NotFound),
            1 => Ok(matches[0].id.clone()),
            _ => Err(SkillResolveError::Multiple(
                matches
                    .into_iter()
                    .map(|s| (s.id.clone(), s.name.clone()))
                    .collect(),
            )),
        }
    }

    // ── Area helpers ──

    pub fn get_area(&self, id: &str) -> Option<&AreaTemplate> {
        self.areas.get(id)
    }

    // ── Room helpers ──

    pub fn get_room(&self, area_id: &str, room_id: &str) -> Option<&RoomTemplate> {
        self.areas.get(area_id)?.rooms.get(room_id)
    }

    pub fn get_room_mut(&mut self, area_id: &str, room_id: &str) -> Option<&mut RoomTemplate> {
        self.areas.get_mut(area_id)?.rooms.get_mut(room_id)
    }

    pub fn room_exists(&self, area_id: &str, room_id: &str) -> bool {
        self.areas
            .get(area_id)
            .is_some_and(|a| a.rooms.contains_key(room_id))
    }

    /// Returns all spawn entries across all areas that match the given
    /// race, class, and alignment constraints.
    ///
    /// Each result is `(area_id, &SpawnEntry)` so the caller can construct
    /// a fully-qualified spawn key (`"area_id:room_id"`).
    pub fn available_spawns(
        &self,
        race: &str,
        class: &str,
        alignment: &str,
    ) -> Vec<(&str, &SpawnEntry)> {
        let mut result = Vec::new();
        for (area_id, area) in &self.areas {
            for spawn in &area.spawns {
                let race_ok =
                    spawn.allowed_races.is_empty() || spawn.allowed_races.iter().any(|r| r == race);
                let class_ok = spawn.allowed_classes.is_empty()
                    || spawn.allowed_classes.iter().any(|c| c == class);
                let align_ok = spawn.allowed_alignments.is_empty()
                    || spawn.allowed_alignments.iter().any(|a| a == alignment);
                if race_ok && class_ok && align_ok {
                    result.push((area_id.as_str(), spawn));
                }
            }
        }
        result
    }

    /// Find a room entity by its spawn key (`"area_id:room_id"`).
    /// Searches all spawned rooms with a matching [`SpawnKey`](crate::SpawnKey) component.
    pub fn find_room_by_key(&self, world: &crate::World, key: &str) -> Option<crate::Entity> {
        use crate::SpawnKey;
        let mut query = world.query::<(&SpawnKey,)>();
        for (e, (sk,)) in query.iter() {
            if sk.0 == key {
                return Some(crate::Entity::from(e));
            }
        }
        None
    }

    // ── Shop helpers ──

    pub fn get_shop(&self, id: &str) -> Option<&ShopTemplate> {
        self.shops.get(id)
    }

    // ── Affix helpers ──

    pub fn get_affix(&self, id: &str) -> Option<&AffixDef> {
        self.affixes.get(id)
    }

    // ── Index helpers ──

    pub fn items_for_set(&self, set_id: &str) -> Option<&[String]> {
        self.indices.items_by_set.get(set_id).map(|v| v.as_slice())
    }

    pub fn items_for_slot(&self, slot: &str) -> Option<&[String]> {
        self.indices.items_by_slot.get(slot).map(|v| v.as_slice())
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
            allowed_alignments: Vec::new(),
            racial_abilities: vec!["adaptability".into()],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
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
            allowed_alignments: Vec::new(),
            auto_skills: vec!["power_attack".into(), "shield_bash".into()],
            skill_pool: vec![
                "power_attack".into(),
                "shield_bash".into(),
                "tactics".into(),
            ],
            starting_skill_slots: 3,
            starting_items: Vec::new(),
            starting_gold: WalletAmount::default(),
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
            allowed_alignments: vec![],
            racial_abilities: vec![],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
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
