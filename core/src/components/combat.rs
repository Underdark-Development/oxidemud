use std::collections::HashMap;
use std::str::FromStr;

use crate::dice::DiceRoll;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl FromStr for DamageType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "slash" => Ok(DamageType::Slash),
            "pierce" => Ok(DamageType::Pierce),
            "bludgeon" => Ok(DamageType::Bludgeon),
            "fire" => Ok(DamageType::Fire),
            "cold" => Ok(DamageType::Cold),
            "lightning" => Ok(DamageType::Lightning),
            "acid" => Ok(DamageType::Acid),
            "poison" => Ok(DamageType::Poison),
            "magic" => Ok(DamageType::Magic),
            "true" => Ok(DamageType::True),
            _ => Err(()),
        }
    }
}

impl DamageType {
    pub fn name(&self) -> &'static str {
        match self {
            DamageType::Slash => "slash",
            DamageType::Pierce => "pierce",
            DamageType::Bludgeon => "bludgeon",
            DamageType::Fire => "fire",
            DamageType::Cold => "cold",
            DamageType::Lightning => "lightning",
            DamageType::Acid => "acid",
            DamageType::Poison => "poison",
            DamageType::Magic => "magic",
            DamageType::True => "true",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CombatStats {
    pub base_attack_bonus: i32,
    pub fort_save: i32,
    pub ref_save: i32,
    pub will_save: i32,
}

#[derive(Debug, Clone)]
pub struct ActiveStance(pub Option<String>);

/// Damage type → multiplier. 1.0 = normal, 2.0 = vulnerable,
/// 0.5 = resistant, 0.0 = immune, -1.0 = absorbed (healed).
#[derive(Debug, Clone)]
pub struct Resistance(pub HashMap<DamageType, f32>);

impl Resistance {
    pub fn multiplier(&self, dt: &DamageType) -> f32 {
        self.0.get(dt).copied().unwrap_or(1.0)
    }

    pub fn apply(&self, amount: i32, dt: &DamageType) -> i32 {
        let mult = self.multiplier(dt);
        if mult == 0.0 {
            return 0;
        }
        if mult < 0.0 {
            // Absorbed — heal instead
            return (amount as f32 * -mult) as i32;
        }
        (amount as f32 * mult).round() as i32
    }
}

#[derive(Debug, Clone)]
pub struct Corpse {
    pub owner: Option<Entity>,
    pub created_at: std::time::Instant,
    pub decay_secs: u64,
    pub lootable_by: LootRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootRule {
    Public,
    GroupOnly,
    OwnerOnly,
    Faction,
}

/// Damage dice + type — uses standard dice notation via `DiceRoll`.
#[derive(Debug, Clone)]
pub struct DamageDice {
    pub dice: DiceRoll,
    pub damage_type: DamageType,
}

impl DamageDice {
    pub fn new(dice: DiceRoll, damage_type: DamageType) -> Self {
        DamageDice { dice, damage_type }
    }

    pub fn roll(&self) -> i32 {
        self.dice.roll()
    }

    pub fn min(&self) -> i32 {
        self.dice.min()
    }

    pub fn max(&self) -> i32 {
        self.dice.max()
    }
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
