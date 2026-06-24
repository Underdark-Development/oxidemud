use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct DiceRoll {
    pub count: u8,
    pub sides: u8,
    pub bonus: i16,
}

impl DiceRoll {
    pub fn new(count: u8, sides: u8, bonus: i16) -> Self {
        DiceRoll {
            count,
            sides,
            bonus,
        }
    }

    pub fn roll(&self) -> i32 {
        let mut total: i32 = 0;
        for _ in 0..self.count {
            total += fastrand::i32(1..=self.sides as i32);
        }
        (total + self.bonus as i32).max(0)
    }

    pub fn min(&self) -> i32 {
        (self.count as i32 + self.bonus as i32).max(0)
    }

    pub fn max(&self) -> i32 {
        (self.count as i32 * self.sides as i32 + self.bonus as i32).max(0)
    }

    /// Average value of this dice roll, rounded to nearest integer.
    /// Formula: count * (sides + 1) / 2 + bonus, rounded.
    pub fn average_rounded(&self) -> i32 {
        let avg = self.count as f64 * (self.sides as f64 + 1.0) / 2.0 + self.bonus as f64;
        (avg.round() as i32).max(0)
    }
}

impl fmt::Display for DiceRoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}d{}", self.count, self.sides)?;
        if self.bonus > 0 {
            write!(f, "+{}", self.bonus)?;
        } else if self.bonus < 0 {
            write!(f, "{}", self.bonus)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseDiceError;

impl fmt::Display for ParseDiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid dice notation")
    }
}

impl FromStr for DiceRoll {
    type Err = ParseDiceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        let (dice_part, bonus) = if let Some(plus) = s.find('+') {
            let bonus: i16 = s[plus + 1..].trim().parse().map_err(|_| ParseDiceError)?;
            (&s[..plus], bonus)
        } else if let Some(minus) = s.find('-') {
            if minus == 0 {
                return Err(ParseDiceError);
            }
            let bonus: i16 = -s[minus + 1..]
                .trim()
                .parse::<i16>()
                .map_err(|_| ParseDiceError)?;
            (&s[..minus], bonus)
        } else {
            (s, 0)
        };

        let dice_part = dice_part.trim();
        let parts: Vec<&str> = dice_part.split('d').collect();
        if parts.len() != 2 {
            return Err(ParseDiceError);
        }

        let count: u8 = if parts[0].is_empty() {
            1
        } else {
            parts[0].parse().map_err(|_| ParseDiceError)?
        };

        let sides: u8 = parts[1].parse().map_err(|_| ParseDiceError)?;

        if count == 0 || sides == 0 {
            return Err(ParseDiceError);
        }

        Ok(DiceRoll {
            count,
            sides,
            bonus,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_d6() {
        let d: DiceRoll = "d6".parse().unwrap();
        assert_eq!(
            d,
            DiceRoll {
                count: 1,
                sides: 6,
                bonus: 0
            }
        );
    }

    #[test]
    fn test_parse_2d8() {
        let d: DiceRoll = "2d8".parse().unwrap();
        assert_eq!(
            d,
            DiceRoll {
                count: 2,
                sides: 8,
                bonus: 0
            }
        );
    }

    #[test]
    fn test_parse_3d6_plus_3() {
        let d: DiceRoll = "3d6+3".parse().unwrap();
        assert_eq!(
            d,
            DiceRoll {
                count: 3,
                sides: 6,
                bonus: 3
            }
        );
    }

    #[test]
    fn test_parse_1d4_minus_1() {
        let d: DiceRoll = "1d4-1".parse().unwrap();
        assert_eq!(
            d,
            DiceRoll {
                count: 1,
                sides: 4,
                bonus: -1
            }
        );
    }

    #[test]
    fn test_parse_invalid() {
        assert!("abc".parse::<DiceRoll>().is_err());
        assert!("d".parse::<DiceRoll>().is_err());
        assert!("-1d6".parse::<DiceRoll>().is_err());
        assert!("0d6".parse::<DiceRoll>().is_err());
        assert!("2d0".parse::<DiceRoll>().is_err());
    }

    #[test]
    fn test_min_max() {
        let d: DiceRoll = "2d8+3".parse().unwrap();
        assert_eq!(d.min(), 5);
        assert_eq!(d.max(), 19);
    }

    #[test]
    fn test_display() {
        assert_eq!(DiceRoll::new(1, 6, 0).to_string(), "1d6");
        assert_eq!(DiceRoll::new(2, 8, 3).to_string(), "2d8+3");
        assert_eq!(DiceRoll::new(1, 4, -1).to_string(), "1d4-1");
    }

    #[test]
    fn test_roll_in_range() {
        let d: DiceRoll = "2d6".parse().unwrap();
        for _ in 0..100 {
            let r = d.roll();
            assert!(r >= 2, "roll {r} below min 2");
            assert!(r <= 12, "roll {r} above max 12");
        }
    }

    #[test]
    fn test_roll_with_bonus() {
        let d: DiceRoll = "1d10+5".parse().unwrap();
        for _ in 0..100 {
            let r = d.roll();
            assert!(r >= 6, "roll {r} below min 6");
            assert!(r <= 15, "roll {r} above max 15");
        }
    }
}
