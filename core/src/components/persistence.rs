#![allow(ambiguous_glob_reexports)]

#[derive(Debug, Clone, Copy)]
pub struct Dirty;

#[derive(Debug, Clone, Copy)]
pub struct DbId(pub i64);

impl DbId {
    pub fn new(id: i64) -> Self {
        DbId(id)
    }
}

#[derive(Debug, Clone)]
pub struct EntityAttributes(pub std::collections::HashMap<String, String>);

impl EntityAttributes {
    pub fn new() -> Self {
        EntityAttributes(std::collections::HashMap::new())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.insert(key.into(), value.into());
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }
}

impl Default for EntityAttributes {
    fn default() -> Self {
        Self::new()
    }
}
