use serde::{Deserialize, Serialize};

/// Configuration for the in-game time system, loaded from `[time]` in server.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConfig {
    /// Real-world minutes per in-game hour (default: 24).
    #[serde(default = "default_real_minutes_per_game_hour")]
    pub real_minutes_per_game_hour: u64,
    /// Game days per season (default: 30).
    #[serde(default = "default_days_per_season")]
    pub days_per_season: u32,
    /// Season on first boot (default: "spring").
    #[serde(default = "default_start_season")]
    pub start_season: String,
    /// Hour on first boot, 0–23 (default: 6).
    #[serde(default = "default_start_hour")]
    pub start_hour: u8,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            real_minutes_per_game_hour: default_real_minutes_per_game_hour(),
            days_per_season: default_days_per_season(),
            start_season: default_start_season(),
            start_hour: default_start_hour(),
        }
    }
}

fn default_real_minutes_per_game_hour() -> u64 {
    24
}
fn default_days_per_season() -> u32 {
    30
}
fn default_start_season() -> String {
    "spring".to_string()
}
fn default_start_hour() -> u8 {
    6
}

/// The four seasons of the in-game world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn name(&self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Autumn => "Autumn",
            Season::Winter => "Winter",
        }
    }

    /// Returns the next season and whether a year boundary was crossed.
    pub fn next(&self) -> (Season, bool) {
        match self {
            Season::Spring => (Season::Summer, false),
            Season::Summer => (Season::Autumn, false),
            Season::Autumn => (Season::Winter, false),
            Season::Winter => (Season::Spring, true),
        }
    }
}

impl std::str::FromStr for Season {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "spring" => Ok(Season::Spring),
            "summer" => Ok(Season::Summer),
            "autumn" | "fall" => Ok(Season::Autumn),
            "winter" => Ok(Season::Winter),
            _ => Err(format!("Unknown season: {}", s)),
        }
    }
}

impl std::fmt::Display for Season {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Named time periods during a 24-hour day cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePeriod {
    Midnight,
    Dawn,
    Morning,
    Noon,
    Afternoon,
    Dusk,
    Evening,
    Night,
}

impl TimePeriod {
    pub fn name(&self) -> &'static str {
        match self {
            TimePeriod::Midnight => "Midnight",
            TimePeriod::Dawn => "Dawn",
            TimePeriod::Morning => "Morning",
            TimePeriod::Noon => "Noon",
            TimePeriod::Afternoon => "Afternoon",
            TimePeriod::Dusk => "Dusk",
            TimePeriod::Evening => "Evening",
            TimePeriod::Night => "Night",
        }
    }
}

/// Maps an hour (0..23) to its corresponding named TimePeriod.
pub fn period_from_hour(hour: u8) -> TimePeriod {
    match hour % 24 {
        1..=4 => TimePeriod::Midnight,
        5..=6 => TimePeriod::Dawn,
        7..=9 => TimePeriod::Morning,
        10..=13 => TimePeriod::Noon,
        14..=16 => TimePeriod::Afternoon,
        17..=18 => TimePeriod::Dusk,
        19..=21 => TimePeriod::Evening,
        _ => TimePeriod::Night, // 22, 23, and 0
    }
}

/// Component representing the current in-game time state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    pub hour: u8,
    pub minute: u8,
    pub day: u32,
    pub season: Season,
    pub year: u32,
}

impl GameTime {
    pub fn new(hour: u8, day: u32, season: Season, year: u32) -> Self {
        Self {
            hour: hour % 24,
            minute: 0,
            day: day.max(1),
            season,
            year: year.max(1),
        }
    }

    pub fn period(&self) -> TimePeriod {
        period_from_hour(self.hour)
    }

    pub fn format_time_cmd(&self) -> String {
        format!(
            "It is {} on the {} day of {}, Year {}.",
            self.period().name(),
            ordinal_suffix(self.day),
            self.season.name(),
            self.year
        )
    }
}

pub fn ordinal_suffix(n: u32) -> String {
    let tens = (n / 10) % 10;
    let units = n % 10;
    let suffix = if tens == 1 {
        "th"
    } else {
        match units {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{}{}", n, suffix)
}

/// Events emitted during time advancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeEvent {
    HourPassed {
        new_hour: u8,
    },
    PeriodChanged {
        old_period: TimePeriod,
        new_period: TimePeriod,
    },
    DayPassed {
        new_day: u32,
    },
    SeasonChanged {
        old_season: Season,
        new_season: Season,
    },
}

/// Advances in-game time by `minutes` and returns all time events that occurred.
pub fn advance_time(time: &mut GameTime, minutes: u8, config: &TimeConfig) -> Vec<TimeEvent> {
    let mut events = Vec::new();
    let old_period = time.period();

    time.minute += minutes;
    if time.minute >= 60 {
        let hours_passed = time.minute / 60;
        time.minute %= 60;

        for _ in 0..hours_passed {
            time.hour = (time.hour + 1) % 24;
            events.push(TimeEvent::HourPassed {
                new_hour: time.hour,
            });

            if time.hour == 0 {
                time.day += 1;

                let days_per_season = if config.days_per_season == 0 {
                    30
                } else {
                    config.days_per_season
                };
                if time.day > days_per_season {
                    time.day = 1;
                    let (next_season, year_wrapped) = time.season.next();
                    let old_season = time.season;
                    time.season = next_season;
                    events.push(TimeEvent::SeasonChanged {
                        old_season,
                        new_season: time.season,
                    });
                    if year_wrapped {
                        time.year += 1;
                    }
                }

                events.push(TimeEvent::DayPassed { new_day: time.day });
            }
        }

        let new_period = time.period();
        if old_period != new_period {
            events.push(TimeEvent::PeriodChanged {
                old_period,
                new_period,
            });
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_from_hour() {
        assert_eq!(period_from_hour(0), TimePeriod::Night);
        assert_eq!(period_from_hour(1), TimePeriod::Midnight);
        assert_eq!(period_from_hour(4), TimePeriod::Midnight);
        assert_eq!(period_from_hour(5), TimePeriod::Dawn);
        assert_eq!(period_from_hour(6), TimePeriod::Dawn);
        assert_eq!(period_from_hour(7), TimePeriod::Morning);
        assert_eq!(period_from_hour(9), TimePeriod::Morning);
        assert_eq!(period_from_hour(10), TimePeriod::Noon);
        assert_eq!(period_from_hour(13), TimePeriod::Noon);
        assert_eq!(period_from_hour(14), TimePeriod::Afternoon);
        assert_eq!(period_from_hour(16), TimePeriod::Afternoon);
        assert_eq!(period_from_hour(17), TimePeriod::Dusk);
        assert_eq!(period_from_hour(18), TimePeriod::Dusk);
        assert_eq!(period_from_hour(19), TimePeriod::Evening);
        assert_eq!(period_from_hour(21), TimePeriod::Evening);
        assert_eq!(period_from_hour(22), TimePeriod::Night);
        assert_eq!(period_from_hour(23), TimePeriod::Night);
    }

    #[test]
    fn test_advance_time_hour_rollover() {
        let mut gt = GameTime::new(5, 1, Season::Spring, 1);
        let config = TimeConfig::default();
        let events = advance_time(&mut gt, 60, &config);

        assert_eq!(gt.hour, 6);
        assert_eq!(gt.minute, 0);
        assert!(events.contains(&TimeEvent::HourPassed { new_hour: 6 }));
    }

    #[test]
    fn test_advance_time_day_season_year_rollover() {
        let mut gt = GameTime::new(23, 30, Season::Winter, 1);
        let config = TimeConfig {
            days_per_season: 30,
            ..Default::default()
        };

        let events = advance_time(&mut gt, 60, &config);

        assert_eq!(gt.hour, 0);
        assert_eq!(gt.day, 1);
        assert_eq!(gt.season, Season::Spring);
        assert_eq!(gt.year, 2);

        assert!(events.contains(&TimeEvent::DayPassed { new_day: 1 }));
        assert!(events.contains(&TimeEvent::SeasonChanged {
            old_season: Season::Winter,
            new_season: Season::Spring,
        }));
    }

    #[test]
    fn test_format_time_cmd() {
        let gt = GameTime::new(6, 14, Season::Spring, 1);
        assert_eq!(
            gt.format_time_cmd(),
            "It is Dawn on the 14th day of Spring, Year 1."
        );
    }
}
