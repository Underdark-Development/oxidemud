use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RestState {
    #[default]
    Standing,
    Sitting,
    Resting,
    Sleeping,
    Unconscious,
    Dead,
}

impl RestState {
    pub fn can_stand(&self) -> bool {
        matches!(
            self,
            RestState::Sitting | RestState::Resting | RestState::Sleeping
        )
    }

    pub fn can_sit(&self) -> bool {
        matches!(self, RestState::Standing)
    }

    pub fn can_rest(&self) -> bool {
        matches!(self, RestState::Sitting)
    }

    pub fn can_sleep(&self) -> bool {
        matches!(self, RestState::Resting)
    }
}

#[derive(Debug, Clone)]
pub enum PlayerState {
    Alive { rest: RestState },
    Stunned { remaining_ms: u64 },
    Casting { remaining_ms: u64 },
    Dead,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState::Alive {
            rest: RestState::Standing,
        }
    }
}

impl PlayerState {
    pub fn rest(&self) -> RestState {
        match self {
            PlayerState::Alive { rest } => *rest,
            PlayerState::Stunned { .. } => RestState::Standing,
            PlayerState::Casting { .. } => RestState::Standing,
            PlayerState::Dead => RestState::Dead,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub account_id: i64,
    pub prompt: Option<String>,
    pub screen_width: u16,
}

/// The entity's gender identity with pronoun resolution.
///
/// Common genders with their default pronouns:
/// - male   → (he, him, his)
/// - female → (she, her, hers)
/// - neutral → (they, them, their)
///
/// Custom genders (e.g. from `other` selection) store explicit pronouns.
#[derive(Debug, Clone)]
pub struct Gender {
    pub gender: String,
    pub pronoun_subject: String,
    pub pronoun_object: String,
    pub pronoun_possessive: String,
}

impl Gender {
    pub fn male() -> Self {
        Gender {
            gender: "male".into(),
            pronoun_subject: "he".into(),
            pronoun_object: "him".into(),
            pronoun_possessive: "his".into(),
        }
    }

    pub fn female() -> Self {
        Gender {
            gender: "female".into(),
            pronoun_subject: "she".into(),
            pronoun_object: "her".into(),
            pronoun_possessive: "hers".into(),
        }
    }

    pub fn neutral() -> Self {
        Gender {
            gender: "neutral".into(),
            pronoun_subject: "they".into(),
            pronoun_object: "them".into(),
            pronoun_possessive: "their".into(),
        }
    }

    pub fn new(
        gender: impl Into<String>,
        subject: impl Into<String>,
        object: impl Into<String>,
        possessive: impl Into<String>,
    ) -> Self {
        Gender {
            gender: gender.into(),
            pronoun_subject: subject.into(),
            pronoun_object: object.into(),
            pronoun_possessive: possessive.into(),
        }
    }
}

impl Default for Gender {
    fn default() -> Self {
        Gender::neutral()
    }
}

/// Structured physical appearance, bounded by race template defaults.
#[derive(Debug, Clone)]
pub struct Appearance {
    pub height: u8,  // inches
    pub weight: u16, // pounds
    pub build: String,
    pub hair_color: String,
    pub hair_style: String,
    pub eye_color: String,
    pub skin_tone: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            height: 66,
            weight: 160,
            build: "average".into(),
            hair_color: "brown".into(),
            hair_style: "straight".into(),
            eye_color: "brown".into(),
            skin_tone: "fair".into(),
        }
    }
}

/// Character age. Initial value comes from race template default.
#[derive(Debug, Clone, Copy, Default)]
pub struct Age(pub u16);

/// The entity's chosen deity (None = no deity).
#[derive(Debug, Clone)]
pub struct Deity(pub Option<String>);

/// The entity's race (e.g. "human", "elf").
#[derive(Debug, Clone)]
pub struct Race(pub String);

/// The entity's class (e.g. "warrior", "mage").
#[derive(Debug, Clone)]
pub struct Class(pub String);

/// The entity's short description (for room look display).
#[derive(Debug, Clone)]
pub struct ShortDesc(pub String);

/// Marker component for NPCs that should not be auto-attacked by guards.
#[derive(Debug, Clone, Copy)]
pub struct Friendly;

impl Player {
    pub fn new(account_id: i64) -> Self {
        Player {
            account_id,
            prompt: None,
            screen_width: 80,
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

#[derive(Debug, Clone, Default)]
pub struct Golds {
    pub copper: u64,
}

/// Multi-denomination currency wallet.
///
/// Exchange rates (all 100:1): 100 copper = 1 silver, 100 silver = 1 gold,
/// 100 gold = 1 platinum.
#[derive(Debug, Clone, Default)]
pub struct Wallet {
    pub copper: u64,
    pub silver: u64,
    pub gold: u64,
    pub platinum: u64,
}

impl Wallet {
    pub const COPPER_PER_SILVER: u64 = 100;
    pub const SILVER_PER_GOLD: u64 = 100;
    pub const GOLD_PER_PLATINUM: u64 = 100;

    pub fn new(copper: u64, silver: u64, gold: u64, platinum: u64) -> Self {
        Wallet {
            copper,
            silver,
            gold,
            platinum,
        }
    }

    /// Total value expressed in copper pieces.
    pub fn total_copper(&self) -> u64 {
        self.copper
            + self.silver * Self::COPPER_PER_SILVER
            + self.gold * Self::COPPER_PER_SILVER * Self::SILVER_PER_GOLD
            + self.platinum
                * Self::COPPER_PER_SILVER
                * Self::SILVER_PER_GOLD
                * Self::GOLD_PER_PLATINUM
    }

    /// Add another wallet's contents into this one.
    pub fn add(&mut self, other: &Wallet) {
        self.copper = self.copper.saturating_add(other.copper);
        self.silver = self.silver.saturating_add(other.silver);
        self.gold = self.gold.saturating_add(other.gold);
        self.platinum = self.platinum.saturating_add(other.platinum);
    }

    /// Deduct an amount of copper, returning true if successful.
    /// Converts higher denominations as needed.
    pub fn deduct_copper(&mut self, amount: u64) -> bool {
        let total = self.total_copper();
        if total < amount {
            return false;
        }
        let remaining = total - amount;
        self.copper = remaining % Self::COPPER_PER_SILVER;
        let remaining = remaining / Self::COPPER_PER_SILVER;
        self.silver = remaining % Self::SILVER_PER_GOLD;
        let remaining = remaining / Self::SILVER_PER_GOLD;
        self.gold = remaining % Self::GOLD_PER_PLATINUM;
        self.platinum = remaining / Self::GOLD_PER_PLATINUM;
        true
    }
}

/// The entity's alignment on the lawful–chaotic × good–evil grid.
///
/// Valid values: `lawful_good`, `neutral_good`, `chaotic_good`,
/// `lawful_neutral`, `true_neutral`, `chaotic_neutral`,
/// `lawful_evil`, `neutral_evil`, `chaotic_evil`.
#[derive(Debug, Clone)]
pub struct Alignment(pub String);

impl Alignment {
    /// All 9 valid alignment strings.
    pub const ALL: &'static [&'static str] = &[
        "lawful_good",
        "neutral_good",
        "chaotic_good",
        "lawful_neutral",
        "true_neutral",
        "chaotic_neutral",
        "lawful_evil",
        "neutral_evil",
        "chaotic_evil",
    ];

    pub fn is_valid(s: &str) -> bool {
        Self::ALL.contains(&s)
    }
}

impl Default for Alignment {
    fn default() -> Self {
        Alignment("true_neutral".to_string())
    }
}

/// A multi-line character description supporting ANSI color.
#[derive(Debug, Clone, Default)]
pub struct Description(pub String);

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

    pub fn to_next_level(&self, level: u8) -> u64 {
        Self::for_level(level + 1).saturating_sub(self.0)
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
    fn test_experience_to_next_level() {
        let xp = Experience(0);
        assert_eq!(xp.to_next_level(1), 800); // 800 - 0
        let xp = Experience(400);
        assert_eq!(xp.to_next_level(1), 400); // 800 - 400
        let xp = Experience(100);
        assert_eq!(xp.to_next_level(1), 700); // 800 - 100
                                              // Already past threshold
        let xp = Experience(900);
        assert_eq!(xp.to_next_level(1), 0); // saturated at 0
    }

    #[test]
    fn test_player_default_prompt() {
        let p = Player::new(42);
        assert_eq!(p.account_id, 42);
        assert_eq!(p.prompt, None);
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

    // ── Wallet tests ──

    #[test]
    fn test_wallet_new() {
        let w = Wallet::new(50, 2, 1, 0);
        assert_eq!(w.copper, 50);
        assert_eq!(w.silver, 2);
        assert_eq!(w.gold, 1);
        assert_eq!(w.platinum, 0);
    }

    #[test]
    fn test_wallet_total_copper() {
        let w = Wallet::new(50, 2, 1, 0);
        // 50 + 2*100 + 1*100*100 = 50 + 200 + 10_000 = 10_250
        assert_eq!(w.total_copper(), 10_250);
    }

    #[test]
    fn test_wallet_add() {
        let mut w1 = Wallet::new(50, 2, 0, 0);
        let w2 = Wallet::new(50, 0, 1, 0);
        w1.add(&w2);
        assert_eq!(w1.copper, 100);
        assert_eq!(w1.silver, 2);
        assert_eq!(w1.gold, 1);
    }

    #[test]
    fn test_wallet_deduct_simple() {
        let mut w = Wallet::new(100, 0, 0, 0);
        assert!(w.deduct_copper(30));
        assert_eq!(w.copper, 70);
    }

    #[test]
    fn test_wallet_deduct_breaks_denominations() {
        let mut w = Wallet::new(0, 1, 0, 0);
        assert!(w.deduct_copper(50));
        // 100 - 50 = 50 copper
        assert_eq!(w.copper, 50);
        assert_eq!(w.silver, 0);
    }

    #[test]
    fn test_wallet_deduct_insufficient() {
        let mut w = Wallet::new(50, 0, 0, 0);
        assert!(!w.deduct_copper(100));
        assert_eq!(w.copper, 50);
    }

    // ── Alignment tests ──

    #[test]
    fn test_alignment_is_valid() {
        assert!(Alignment::is_valid("lawful_good"));
        assert!(Alignment::is_valid("true_neutral"));
        assert!(Alignment::is_valid("chaotic_evil"));
        assert!(!Alignment::is_valid("good"));
        assert!(!Alignment::is_valid(""));
    }

    #[test]
    fn test_alignment_default() {
        let a = Alignment::default();
        assert_eq!(a.0, "true_neutral");
    }

    #[test]
    fn test_all_alignments_count() {
        assert_eq!(Alignment::ALL.len(), 9);
    }

    #[test]
    fn test_description_default() {
        let d = Description::default();
        assert_eq!(d.0, "");
    }
}
