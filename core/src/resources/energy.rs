use crate::regen::PoolRegen;

#[derive(Debug, Clone, Copy, Default)]
pub struct Energy {
    pub current: u16,
    pub max: u16,
}

impl Energy {
    pub fn new(max: u16) -> Self {
        Energy { current: max, max }
    }

    pub fn fraction(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }
}

impl PoolRegen for Energy {
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
