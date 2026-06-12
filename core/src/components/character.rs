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

#[derive(Debug, Clone)]
pub struct Name(pub String);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Name(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Name {
    fn default() -> Self {
        Name("Adventurer".to_string())
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for Name {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Name(s)
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Name(s.to_string())
    }
}

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

    #[test]
    fn test_name_new() {
        let n = Name::new("Alice");
        assert_eq!(n.as_str(), "Alice");
    }

    #[test]
    fn test_name_default() {
        let n = Name::default();
        assert_eq!(n.as_str(), "Adventurer");
    }

    #[test]
    fn test_name_display() {
        let n = Name::new("Bob");
        assert_eq!(format!("{n}"), "Bob");
    }

    #[test]
    fn test_name_from_string() {
        let n: Name = "Charlie".to_string().into();
        assert_eq!(n.as_str(), "Charlie");
    }

    #[test]
    fn test_name_from_str() {
        let n: Name = "Diana".into();
        assert_eq!(n.as_str(), "Diana");
    }

    #[test]
    fn test_name_deref() {
        let n = Name::new("Eve");
        assert_eq!(n.len(), 3);
        assert!(n.starts_with("E"));
    }
}
