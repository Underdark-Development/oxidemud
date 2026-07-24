use std::collections::HashMap;
use std::str::FromStr;

use crate::dice::DiceRoll;
use crate::templates;
use crate::DamageType;
use crate::Entity;

#[derive(Debug, Clone)]
pub struct Item {
    pub template_id: String,
}

impl Item {
    pub fn new(template_id: impl Into<String>) -> Self {
        Item {
            template_id: template_id.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Inventory(pub Vec<Entity>);

impl Inventory {
    pub fn new() -> Self {
        Inventory(Vec::new())
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    Head,
    Neck,
    Torso,
    Arms,
    Hands,
    Finger,
    Legs,
    Feet,
    Weapon,
    Shield,
    Ammo,
    Back,
    Waist,
}

impl FromStr for EquipmentSlot {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "head" => Ok(EquipmentSlot::Head),
            "neck" => Ok(EquipmentSlot::Neck),
            "torso" => Ok(EquipmentSlot::Torso),
            "arms" => Ok(EquipmentSlot::Arms),
            "hands" => Ok(EquipmentSlot::Hands),
            "finger" => Ok(EquipmentSlot::Finger),
            "legs" => Ok(EquipmentSlot::Legs),
            "feet" => Ok(EquipmentSlot::Feet),
            "weapon" => Ok(EquipmentSlot::Weapon),
            "shield" => Ok(EquipmentSlot::Shield),
            "ammo" => Ok(EquipmentSlot::Ammo),
            "back" => Ok(EquipmentSlot::Back),
            "waist" => Ok(EquipmentSlot::Waist),
            _ => Err(()),
        }
    }
}

impl EquipmentSlot {
    pub fn all() -> &'static [EquipmentSlot] {
        &[
            EquipmentSlot::Head,
            EquipmentSlot::Neck,
            EquipmentSlot::Torso,
            EquipmentSlot::Arms,
            EquipmentSlot::Hands,
            EquipmentSlot::Finger,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
            EquipmentSlot::Weapon,
            EquipmentSlot::Shield,
            EquipmentSlot::Ammo,
            EquipmentSlot::Back,
            EquipmentSlot::Waist,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Equipment {
    pub slots: Vec<(EquipmentSlot, Entity)>,
}

impl Equipment {
    pub fn new() -> Self {
        Equipment { slots: Vec::new() }
    }

    pub fn equipped(&self, slot: &EquipmentSlot) -> Option<&Entity> {
        self.slots.iter().find(|(s, _)| s == slot).map(|(_, e)| e)
    }

    pub fn equip(&mut self, slot: EquipmentSlot, item: Entity) {
        self.slots.retain(|(s, _)| *s != slot);
        self.slots.push((slot, item));
    }

    pub fn unequip(&mut self, slot: &EquipmentSlot) -> Option<Entity> {
        let idx = self.slots.iter().position(|(s, _)| s == slot)?;
        Some(self.slots.remove(idx).1)
    }
}

impl Default for Equipment {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponRange {
    Melee,
    Ranged,
    Reach,
    Thrown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponHands {
    OneHand,
    TwoHand,
}

#[derive(Debug, Clone)]
pub struct Weapon {
    pub damage_dice: DiceRoll,
    pub damage_type: DamageType,
    pub speed: f32,
    pub range: WeaponRange,
    pub hands: WeaponHands,
}

impl Weapon {
    pub fn is_two_handed(&self) -> bool {
        self.hands == WeaponHands::TwoHand
    }

    pub fn is_ranged(&self) -> bool {
        matches!(self.range, WeaponRange::Ranged | WeaponRange::Thrown)
    }

    pub fn effective_speed(&self) -> f32 {
        if self.is_two_handed() {
            self.speed * 1.2
        } else {
            self.speed
        }
    }
}

#[derive(Debug, Clone)]
pub struct Durability {
    pub current: u16,
    pub max: u16,
    pub decay_rate: f32,
}

impl Durability {
    pub fn new(max: u16) -> Self {
        Durability {
            current: max,
            max,
            decay_rate: 1.0,
        }
    }

    pub fn is_broken(&self) -> bool {
        self.current == 0
    }

    pub fn damage(&mut self, amount: u16) {
        self.current = self.current.saturating_sub(amount);
    }

    pub fn repair(&mut self, amount: u16) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// Which item set this piece belongs to (set_id, piece_type).
/// Populated from the template when the item entity is spawned.
#[derive(Debug, Clone)]
pub struct SetMembership {
    pub set_id: String,
    pub piece_type: String,
}

impl From<templates::SetMembership> for SetMembership {
    fn from(t: templates::SetMembership) -> Self {
        SetMembership {
            set_id: t.id,
            piece_type: t.piece_type,
        }
    }
}

/// Tracks how many equipped pieces of each set the entity has.
#[derive(Debug, Clone)]
pub struct SetTracker(pub HashMap<String, u8>);

impl SetTracker {
    pub fn new() -> Self {
        SetTracker(HashMap::new())
    }

    pub fn count(&self, set_id: &str) -> u8 {
        self.0.get(set_id).copied().unwrap_or(0)
    }
}

impl Default for SetTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// An active effect on an entity (from set bonuses, spells, etc.).
#[derive(Debug, Clone)]
pub struct ActiveEffect {
    pub source: String,
    pub stat: Option<String>,
    pub amount: Option<i32>,
    pub aura_id: Option<String>,
    pub radius: Option<u32>,
}

/// Affix names attached to a looted item (prefix/suffix).
#[derive(Debug, Clone)]
pub struct AffixNames(pub Vec<String>);

/// Skill requirement for equipping this item (populated from template at spawn).
#[derive(Debug, Clone)]
pub struct ItemSkillRequirement {
    pub id: String,
    pub level: u16,
}

/// Resolved stat modifiers from affixes (parsed from AffixDef.amount).
#[derive(Debug, Clone)]
pub struct AffixModifiers(pub Vec<AffixMod>);

#[derive(Debug, Clone)]
pub struct AffixMod {
    pub stat: String,
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumableKind {
    Potion,
    Scroll,
    Wand,
    Food,
    Drink,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct Consumable {
    pub kind: ConsumableKind,
    pub charges: u16,
    pub max_charges: u16,
    pub effect_script: Option<String>,
    pub restore_health: i32,
    pub restore_mana: i32,
    pub restore_stamina: i32,
    pub depleted_template: Option<String>,
    pub replenishable: bool,
    pub liquid_type: Option<String>,
}

impl Consumable {
    pub fn is_empty(&self) -> bool {
        self.charges == 0
    }
}

#[derive(Debug, Clone)]
pub struct ItemContainer {
    pub contents: Vec<Entity>,
    pub capacity_weight: f32,
    pub max_items: u16,
    pub weight_reduction_pct: u8,
    pub is_closed: bool,
    pub is_locked: bool,
    pub key_template_id: Option<String>,
}

impl ItemContainer {
    pub fn new(
        capacity_weight: f32,
        max_items: u16,
        weight_reduction_pct: u8,
        is_closed: bool,
        is_locked: bool,
        key_template_id: Option<String>,
    ) -> Self {
        ItemContainer {
            contents: Vec::new(),
            capacity_weight,
            max_items,
            weight_reduction_pct: weight_reduction_pct.min(100),
            is_closed,
            is_locked,
            key_template_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DrinkContainer {
    pub liquid_type: String,
    pub charges: u16,
    pub max_charges: u16,
    pub replenishable: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ItemFlags {
    pub fixed: bool,
    pub flags: Vec<String>,
}

impl ItemFlags {
    pub fn is_gettable(&self) -> bool {
        !self.fixed && !self.flags.iter().any(|f| f == "fixed" || f == "!gettable")
    }
}
