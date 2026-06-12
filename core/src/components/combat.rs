use crate::Entity;

#[derive(Debug, Clone)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Health { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0
    }

    pub fn damage(&mut self, amount: i32) {
        self.current = (self.current - amount).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
}

#[derive(Debug, Clone)]
pub struct Damage(pub i32);

#[derive(Debug, Clone)]
pub struct Armor {
    pub base: i32,
    pub bonus: i32,
}

impl Armor {
    pub fn total(&self) -> i32 {
        self.base + self.bonus
    }
}

#[derive(Debug, Clone)]
pub struct CombatTarget(pub Entity);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageType {
    Slash,
    Pierce,
    Bludgeon,
    Fire,
    Cold,
    Lightning,
    Acid,
    Poison,
    Magic,
    True,
}
