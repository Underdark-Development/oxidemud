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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_new() {
        let h = Health::new(100);
        assert_eq!(h.current, 100);
        assert_eq!(h.max, 100);
        assert!(h.is_alive());
        assert!(!h.is_dead());
    }

    #[test]
    fn test_health_damage() {
        let mut h = Health::new(100);
        h.damage(30);
        assert_eq!(h.current, 70);
        assert!(h.is_alive());
    }

    #[test]
    fn test_health_damage_clamp() {
        let mut h = Health::new(100);
        h.damage(200);
        assert_eq!(h.current, 0);
        assert!(h.is_dead());
    }

    #[test]
    fn test_health_heal() {
        let mut h = Health::new(100);
        h.damage(50);
        h.heal(20);
        assert_eq!(h.current, 70);
    }

    #[test]
    fn test_health_heal_clamp() {
        let mut h = Health::new(100);
        h.damage(10);
        h.heal(50);
        assert_eq!(h.current, 100);
    }

    #[test]
    fn test_armor_total() {
        let a = Armor { base: 10, bonus: 5 };
        assert_eq!(a.total(), 15);
    }

    #[test]
    fn test_armor_total_no_bonus() {
        let a = Armor { base: 10, bonus: 0 };
        assert_eq!(a.total(), 10);
    }
}
