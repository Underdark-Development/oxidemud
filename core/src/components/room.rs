use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
    Northeast,
    Northwest,
    Southeast,
    Southwest,
}

impl Direction {
    pub fn all() -> &'static [Direction] {
        &[
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
            Direction::Up,
            Direction::Down,
            Direction::Northeast,
            Direction::Northwest,
            Direction::Southeast,
            Direction::Southwest,
        ]
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Direction::North => "n",
            Direction::South => "s",
            Direction::East => "e",
            Direction::West => "w",
            Direction::Up => "u",
            Direction::Down => "d",
            Direction::Northeast => "ne",
            Direction::Northwest => "nw",
            Direction::Southeast => "se",
            Direction::Southwest => "sw",
        }
    }

    pub fn long_name(&self) -> &'static str {
        match self {
            Direction::North => "north",
            Direction::South => "south",
            Direction::East => "east",
            Direction::West => "west",
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Northeast => "northeast",
            Direction::Northwest => "northwest",
            Direction::Southeast => "southeast",
            Direction::Southwest => "southwest",
        }
    }

    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Northeast => Direction::Southwest,
            Direction::Northwest => Direction::Southeast,
            Direction::Southeast => Direction::Northwest,
            Direction::Southwest => Direction::Northeast,
        }
    }

    pub fn from_short(s: &str) -> Option<Self> {
        match s {
            "n" => Some(Direction::North),
            "s" => Some(Direction::South),
            "e" => Some(Direction::East),
            "w" => Some(Direction::West),
            "u" => Some(Direction::Up),
            "d" => Some(Direction::Down),
            "ne" => Some(Direction::Northeast),
            "nw" => Some(Direction::Northwest),
            "se" => Some(Direction::Southeast),
            "sw" => Some(Direction::Southwest),
            _ => None,
        }
    }

    pub fn from_long(s: &str) -> Option<Self> {
        match s {
            "north" => Some(Direction::North),
            "south" => Some(Direction::South),
            "east" => Some(Direction::East),
            "west" => Some(Direction::West),
            "up" => Some(Direction::Up),
            "down" => Some(Direction::Down),
            "northeast" => Some(Direction::Northeast),
            "northwest" => Some(Direction::Northwest),
            "southeast" => Some(Direction::Southeast),
            "southwest" => Some(Direction::Southwest),
            _ => None,
        }
    }

    pub fn try_from(s: &str) -> Option<Self> {
        Direction::from_short(s).or_else(|| Direction::from_long(s))
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.long_name())
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    pub room: crate::Entity,
}

impl Position {
    pub fn new(room: crate::Entity) -> Self {
        Position { room }
    }
}

#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub description: String,
}

impl Room {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Room {
            name: name.into(),
            description: description.into(),
        }
    }
}

pub type ExitFlags = u8;

pub const EXIT_IS_DOOR: ExitFlags = 0x01;
pub const EXIT_IS_CLOSED: ExitFlags = 0x02;
pub const EXIT_IS_LOCKED: ExitFlags = 0x04;
pub const EXIT_HIDDEN: ExitFlags = 0x08;

#[derive(Debug, Clone)]
pub struct Exit {
    pub direction: Direction,
    pub dest: crate::Entity,
    pub flags: ExitFlags,
}

impl Exit {
    pub fn new(direction: Direction, dest: crate::Entity) -> Self {
        Exit {
            direction,
            dest,
            flags: 0,
        }
    }

    pub fn is_door(&self) -> bool {
        self.flags & EXIT_IS_DOOR != 0
    }

    pub fn is_closed(&self) -> bool {
        self.flags & EXIT_IS_CLOSED != 0
    }

    pub fn is_locked(&self) -> bool {
        self.flags & EXIT_IS_LOCKED != 0
    }

    pub fn is_hidden(&self) -> bool {
        self.flags & EXIT_HIDDEN != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_round_trip() {
        for d in Direction::all() {
            let short = d.short_name();
            assert_eq!(Direction::from_short(short), Some(*d), "short: {short}");
            let long = d.long_name();
            assert_eq!(Direction::from_long(long), Some(*d), "long: {long}");
        }
    }

    #[test]
    fn test_direction_try_from() {
        assert_eq!(Direction::try_from("n"), Some(Direction::North));
        assert_eq!(Direction::try_from("north"), Some(Direction::North));
        assert_eq!(Direction::try_from("ne"), Some(Direction::Northeast));
        assert_eq!(Direction::try_from("northeast"), Some(Direction::Northeast));
        assert_eq!(Direction::try_from("x"), None);
        assert_eq!(Direction::try_from(""), None);
    }

    #[test]
    fn test_direction_opposite() {
        assert_eq!(Direction::North.opposite(), Direction::South);
        assert_eq!(Direction::South.opposite(), Direction::North);
        assert_eq!(Direction::East.opposite(), Direction::West);
        assert_eq!(Direction::West.opposite(), Direction::East);
        assert_eq!(Direction::Up.opposite(), Direction::Down);
        assert_eq!(Direction::Down.opposite(), Direction::Up);
        assert_eq!(Direction::Northeast.opposite(), Direction::Southwest);
        assert_eq!(Direction::Southwest.opposite(), Direction::Northeast);
    }

    #[test]
    fn test_exit_flags() {
        let e = crate::Entity::from(hecs::Entity::from_bits(0x0000_0001_0000_0001).unwrap());
        let mut exit = Exit::new(Direction::North, e);
        assert!(!exit.is_door());
        assert!(!exit.is_closed());

        exit.flags = EXIT_IS_DOOR | EXIT_IS_CLOSED;
        assert!(exit.is_door());
        assert!(exit.is_closed());
        assert!(!exit.is_locked());
    }
}
