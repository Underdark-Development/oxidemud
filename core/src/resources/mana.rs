use crate::regen::PoolRegen;

#[derive(Debug, Clone, Copy, Default)]
pub struct Mana {
    pub current: u16,
    pub max: u16,
}

impl Mana {
    pub fn new(max: u16) -> Self {
        Mana { current: max, max }
    }

    pub fn from_formula(level: u16, int: u16, wis: u16) -> Self {
        let max = level * 4 + int * 2 + wis * 2;
        Mana { current: max, max }
    }

    pub fn fraction(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }
}

impl PoolRegen for Mana {
    fn current(&self) -> u16 {
        self.current
    }
    fn max(&self) -> u16 {
        self.max
    }
    fn set_current(&mut self, val: u16) {
        self.current = val;
    }
    fn base_regen() -> u16 {
        2
    }
}
