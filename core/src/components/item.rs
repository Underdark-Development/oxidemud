use std::str::FromStr;

use crate::dice::DiceRoll;
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

#[derive(Debug, Clone)]
pub struct Weapon {
    pub damage_dice: DiceRoll,
    pub damage_type: DamageType,
    pub speed: f32,
    pub range: WeaponRange,
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
