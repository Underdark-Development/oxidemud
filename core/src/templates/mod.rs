use serde::Deserialize;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Race template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct RaceTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub attributes: RaceAttributes,
    #[serde(default)]
    pub allowed_classes: Vec<String>,
    #[serde(default)]
    pub racial_abilities: Vec<String>,
}

// ---------------------------------------------------------------------------
// Class template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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
    pub auto_skills: Vec<String>,
}

const fn default_hit_die() -> u8 {
    8
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    pub races: HashMap<String, RaceTemplate>,
    pub classes: HashMap<String, ClassTemplate>,
}

impl TemplateRegistry {
    pub fn new(
        races: HashMap<String, RaceTemplate>,
        classes: HashMap<String, ClassTemplate>,
    ) -> Self {
        TemplateRegistry { races, classes }
    }

    pub fn get_race(&self, id: &str) -> Option<&RaceTemplate> {
        self.races.get(id)
    }

    pub fn get_class(&self, id: &str) -> Option<&ClassTemplate> {
        self.classes.get(id)
    }

    pub fn available_classes_for_race(&self, race_id: &str) -> Vec<&ClassTemplate> {
        self.classes
            .values()
            .filter(|c| {
                c.allowed_races.is_empty() || c.allowed_races.contains(&race_id.to_string())
            })
            .collect()
    }

    pub fn available_races_for_class(&self, class_id: &str) -> Vec<&RaceTemplate> {
        let class = match self.classes.get(class_id) {
            Some(c) => c,
            None => return Vec::new(),
        };
        self.races
            .values()
            .filter(|r| {
                r.allowed_classes.is_empty()
                    || class.allowed_races.is_empty()
                    || (r.allowed_classes.contains(&class_id.to_string())
                        && class.allowed_races.contains(&r.id.to_string()))
            })
            .collect()
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
            racial_abilities: vec!["adaptability".into()],
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
            auto_skills: vec!["power_attack".into(), "shield_bash".into()],
        }
    }

    #[test]
    fn test_get_race() {
        let reg = TemplateRegistry::new(
            vec![("human".into(), test_race())].into_iter().collect(),
            HashMap::new(),
        );
        assert!(reg.get_race("human").is_some());
        assert!(reg.get_race("elf").is_none());
    }

    #[test]
    fn test_get_class() {
        let reg = TemplateRegistry::new(
            HashMap::new(),
            vec![("warrior".into(), test_class())].into_iter().collect(),
        );
        assert!(reg.get_class("warrior").is_some());
        assert!(reg.get_class("mage").is_none());
    }

    #[test]
    fn test_available_classes_for_race() {
        let mut classes = HashMap::new();
        classes.insert("warrior".into(), test_class());
        let mut mage = test_class();
        mage.id = "mage".into();
        mage.allowed_races = vec!["human".into()];
        classes.insert("mage".into(), mage);
        let mut elf_class = test_class();
        elf_class.id = "elf_only".into();
        elf_class.allowed_races = vec!["elf".into()];
        classes.insert("elf_only".into(), elf_class);

        let races = vec![("human".into(), test_race())].into_iter().collect();
        let reg = TemplateRegistry::new(races, classes);
        let available = reg.available_classes_for_race("human");
        assert_eq!(available.len(), 2);
        assert!(available.iter().any(|c| c.id == "warrior"));
        assert!(available.iter().any(|c| c.id == "mage"));
    }

    #[test]
    fn test_race_defaults() {
        let r = RaceTemplate {
            id: "test".into(),
            name: "Test".into(),
            description: "Desc.".into(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec![],
            racial_abilities: vec![],
        };
        assert_eq!(r.attributes.strength, 10);
        assert!(r.allowed_classes.is_empty());
    }

    #[test]
    fn test_class_attribute_mods_default() {
        let m = ClassAttributeMods::default();
        assert_eq!(m.strength, 0);
    }

    #[test]
    fn test_available_races_for_class() {
        let mut races = HashMap::new();
        races.insert("human".into(), test_race());
        let mut elf_race = test_race();
        elf_race.id = "elf".into();
        elf_race.allowed_classes = vec!["mage".into()];
        races.insert("elf".into(), elf_race);

        let classes = vec![("warrior".into(), test_class())].into_iter().collect();
        let reg = TemplateRegistry::new(races, classes);
        let available = reg.available_races_for_class("warrior");
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id, "human");
    }

    #[test]
    fn test_auto_skills_defaults_empty() {
        let c = test_class();
        assert!(!c.auto_skills.is_empty());
        let c2 = ClassTemplate {
            id: "empty".into(),
            name: "Empty".into(),
            description: ".".into(),
            hit_die: 8,
            attribute_mods: ClassAttributeMods::default(),
            allowed_races: vec![],
            auto_skills: vec![],
        };
        assert!(c2.auto_skills.is_empty());
    }
}
