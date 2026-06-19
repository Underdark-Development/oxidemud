use std::time::Duration;

/// Trait for resource pools that regenerate over time.
pub trait PoolRegen {
    fn current(&self) -> u16;
    fn max(&self) -> u16;
    fn set_current(&mut self, val: u16);
    fn base_regen() -> u16;

    fn regen_amount(&self, rest_mult: f32, tick_duration: Duration) -> u16 {
        let base = Self::base_regen();
        let ticks = tick_duration.as_secs_f32() / 6.0;
        let amount = (base as f32 * rest_mult * ticks).round() as u16;
        (self.max() - self.current()).min(amount.max(1))
    }

    fn apply_regen(&mut self, rest_mult: f32, tick_duration: Duration) {
        let amount = self.regen_amount(rest_mult, tick_duration);
        if amount > 0 {
            let new = self.current().saturating_add(amount).min(self.max());
            self.set_current(new);
        }
    }

    fn consume(&mut self, amount: u16) -> bool {
        if self.current() >= amount {
            self.set_current(self.current() - amount);
            true
        } else {
            false
        }
    }
}

/// Rest state multiplier for regen rates.
/// Standing: 1x, Sitting: 2x, Resting: 3x, Sleeping: 5x
pub fn rest_multiplier(rest_state: &crate::RestState) -> f32 {
    match rest_state {
        crate::RestState::Standing => 1.0,
        crate::RestState::Sitting => 2.0,
        crate::RestState::Resting => 3.0,
        crate::RestState::Sleeping => 5.0,
        crate::RestState::Unconscious => 0.5,
        crate::RestState::Dead => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPool {
        current: u16,
        max: u16,
    }

    impl PoolRegen for TestPool {
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
            10
        }
    }

    #[test]
    fn test_regen_amount() {
        let pool = TestPool {
            current: 0,
            max: 100,
        };
        // 10 base * 1.0 standing * 1.0 ticks = 10, capped at 100
        assert_eq!(pool.regen_amount(1.0, Duration::from_secs(6)), 10);
    }

    #[test]
    fn test_regen_amount_capped() {
        let pool = TestPool {
            current: 98,
            max: 100,
        };
        // 10 base * 1.0 = 10, but only 2 needed
        assert_eq!(pool.regen_amount(1.0, Duration::from_secs(6)), 2);
    }

    #[test]
    fn test_regen_amount_full() {
        let pool = TestPool {
            current: 100,
            max: 100,
        };
        assert_eq!(pool.regen_amount(1.0, Duration::from_secs(6)), 0);
    }

    #[test]
    fn test_apply_regen() {
        let mut pool = TestPool {
            current: 50,
            max: 100,
        };
        pool.apply_regen(1.0, Duration::from_secs(6));
        assert_eq!(pool.current, 60); // 50 + 10
    }

    #[test]
    fn test_consume_ok() {
        let mut pool = TestPool {
            current: 50,
            max: 100,
        };
        assert!(pool.consume(30));
        assert_eq!(pool.current, 20);
    }

    #[test]
    fn test_consume_fail() {
        let mut pool = TestPool {
            current: 5,
            max: 100,
        };
        assert!(!pool.consume(30));
        assert_eq!(pool.current, 5);
    }

    #[test]
    fn test_rest_multiplier() {
        use crate::RestState;
        assert!((rest_multiplier(&RestState::Standing) - 1.0).abs() < f32::EPSILON);
        assert!((rest_multiplier(&RestState::Sitting) - 2.0).abs() < f32::EPSILON);
        assert!((rest_multiplier(&RestState::Resting) - 3.0).abs() < f32::EPSILON);
        assert!((rest_multiplier(&RestState::Sleeping) - 5.0).abs() < f32::EPSILON);
        assert!((rest_multiplier(&RestState::Unconscious) - 0.5).abs() < f32::EPSILON);
        assert!((rest_multiplier(&RestState::Dead) - 0.0).abs() < f32::EPSILON);
    }
}
