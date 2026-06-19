use crate::regen::PoolRegen;

#[derive(Debug, Clone, Copy, Default)]
pub struct Stamina {
    pub current: u16,
    pub max: u16,
}

impl Stamina {
    pub fn new(max: u16) -> Self {
        Stamina { current: max, max }
    }

    pub fn from_formula(level: u16, strength: u16, dexterity: u16) -> Self {
        let max = level * 12 + strength * 2 + dexterity * 2;
        Stamina { current: max, max }
    }

    pub fn fraction(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }
}

impl PoolRegen for Stamina {
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
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stamina_new() {
        let s = Stamina::new(100);
        assert_eq!(s.current, 100);
        assert_eq!(s.max, 100);
    }

    #[test]
    fn test_stamina_fraction() {
        let s = Stamina {
            current: 50,
            max: 100,
        };
        assert!((s.fraction() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stamina_zero_max() {
        let s = Stamina::new(0);
        assert_eq!(s.fraction(), 0.0);
    }
}
