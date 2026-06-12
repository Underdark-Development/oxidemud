#[derive(Debug, Clone, Copy, Default)]
pub struct Mana {
    pub current: u16,
    pub max: u16,
}

impl Mana {
    pub fn new(max: u16) -> Self {
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
