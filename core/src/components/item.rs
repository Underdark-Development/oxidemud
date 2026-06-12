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
