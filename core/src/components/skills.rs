use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LearnedSkills {
    pub skills: HashMap<String, u16>,
}

impl LearnedSkills {
    pub fn new() -> Self {
        LearnedSkills {
            skills: HashMap::new(),
        }
    }

    pub fn grant(&mut self, skill_id: impl Into<String>) {
        self.skills.entry(skill_id.into()).or_insert(1);
    }

    pub fn has(&self, skill_id: &str) -> bool {
        self.skills.contains_key(skill_id)
    }

    pub fn rank(&self, skill_id: &str) -> u16 {
        self.skills.get(skill_id).copied().unwrap_or(0)
    }
}

impl Default for LearnedSkills {
    fn default() -> Self {
        Self::new()
    }
}
