use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Player {
    pub account_id: i64,
    pub prompt: String,
}

impl Player {
    pub fn new(account_id: i64) -> Self {
        Player {
            account_id,
            prompt: "<%hhp %hmhp> ".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Npc {
    pub template_id: String,
}

impl Npc {
    pub fn new(template_id: impl Into<String>) -> Self {
        Npc {
            template_id: template_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attributes {
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub constitution: u8,
    pub charisma: u8,
}

impl Attributes {
    pub const MAX: u8 = 50;
    pub const MIN: u8 = 3;

    pub fn new(
        strength: u8,
        dexterity: u8,
        intelligence: u8,
        wisdom: u8,
        constitution: u8,
        charisma: u8,
    ) -> Self {
        Attributes {
            strength: strength.clamp(Self::MIN, Self::MAX),
            dexterity: dexterity.clamp(Self::MIN, Self::MAX),
            intelligence: intelligence.clamp(Self::MIN, Self::MAX),
            wisdom: wisdom.clamp(Self::MIN, Self::MAX),
            constitution: constitution.clamp(Self::MIN, Self::MAX),
            charisma: charisma.clamp(Self::MIN, Self::MAX),
        }
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes::new(10, 10, 10, 10, 10, 10)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level(pub u8);

impl Default for Level {
    fn default() -> Self {
        Level(1)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Experience(pub u64);

impl Experience {
    pub fn for_level(level: u8) -> u64 {
        (level as u64).saturating_pow(3) * 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attributes_default() {
        let a = Attributes::default();
        assert_eq!(a.strength, 10);
        assert_eq!(a.dexterity, 10);
        assert_eq!(a.intelligence, 10);
        assert_eq!(a.wisdom, 10);
        assert_eq!(a.constitution, 10);
        assert_eq!(a.charisma, 10);
    }

    #[test]
    fn test_attributes_clamp() {
        let a = Attributes::new(100, 1, 10, 10, 10, 10);
        assert_eq!(a.strength, Attributes::MAX);
        assert_eq!(a.dexterity, Attributes::MIN);
    }

    #[test]
    fn test_experience_for_level() {
        assert_eq!(Experience::for_level(1), 100);
        assert_eq!(Experience::for_level(2), 800);
        assert_eq!(Experience::for_level(10), 100_000);
    }

    #[test]
    fn test_player_default_prompt() {
        let p = Player::new(42);
        assert_eq!(p.account_id, 42);
        assert_eq!(p.prompt, "<%hhp %hmhp> ");
    }
}
