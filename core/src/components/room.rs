use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
