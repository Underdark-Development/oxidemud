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
    pub script: Option<String>,
}

impl Room {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Room {
            name: name.into(),
            description: description.into(),
            script: None,
        }
    }

    pub fn with_script(mut self, script: Option<String>) -> Self {
        self.script = script;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VoidRoom;

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
    pub key_id: Option<String>,
}

impl Exit {
    pub fn new(direction: Direction, dest: crate::Entity) -> Self {
        Exit {
            direction,
            dest,
            flags: 0,
            key_id: None,
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

    pub fn set_closed(&mut self, closed: bool) {
        if closed {
            self.flags |= EXIT_IS_CLOSED;
        } else {
            self.flags &= !EXIT_IS_CLOSED;
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        if locked {
            self.flags |= EXIT_IS_LOCKED;
        } else {
            self.flags &= !EXIT_IS_LOCKED;
        }
    }
}

/// Storage component on room entities for directional exits.
#[derive(Debug, Clone)]
pub struct RoomExits(pub Vec<Exit>);

// ---------------------------------------------------------------------------
// Portals — name-based movement points ("enter <keyword>")
// ---------------------------------------------------------------------------

pub type PortalFlags = u8;

pub const PORTAL_HIDDEN: PortalFlags = 0x01;

#[derive(Debug, Clone)]
pub struct PortalExit {
    pub keyword: String,
    pub dest: crate::Entity,
    pub description: String,
    pub flags: PortalFlags,
}

impl PortalExit {
    pub fn new(
        keyword: impl Into<String>,
        dest: crate::Entity,
        description: impl Into<String>,
    ) -> Self {
        PortalExit {
            keyword: keyword.into(),
            dest,
            description: description.into(),
            flags: 0,
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.flags & PORTAL_HIDDEN != 0
    }
}

/// Storage component on room entities for portal exits.
#[derive(Debug, Clone)]
pub struct RoomPortals(pub Vec<PortalExit>);

// ---------------------------------------------------------------------------
// Room flags — controls portal/teleport permissions
// ---------------------------------------------------------------------------

pub type RoomFlagBits = u16;

/// Room has `portal_in` — temp portals can target this room (opt-in).
pub const ROOM_PORTAL_IN: RoomFlagBits = 0x0001;
/// Room has `portal_out` — temp portals can originate from this room (opt-in).
pub const ROOM_PORTAL_OUT: RoomFlagBits = 0x0002;
/// Room has `no_teleport_in` — teleport spells cannot land here (opt-out).
pub const ROOM_NO_TELEPORT_IN: RoomFlagBits = 0x0004;
/// Room has `no_teleport_out` — teleport spells cannot leave here (opt-out).
pub const ROOM_NO_TELEPORT_OUT: RoomFlagBits = 0x0008;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RoomFlags(pub RoomFlagBits);

impl RoomFlags {
    pub fn can_portal_in(&self) -> bool {
        self.0 & ROOM_PORTAL_IN != 0
    }

    pub fn can_portal_out(&self) -> bool {
        self.0 & ROOM_PORTAL_OUT != 0
    }

    pub fn no_teleport_in(&self) -> bool {
        self.0 & ROOM_NO_TELEPORT_IN != 0
    }

    pub fn no_teleport_out(&self) -> bool {
        self.0 & ROOM_NO_TELEPORT_OUT != 0
    }
}

// ---------------------------------------------------------------------------
// Teleportable — whether a non-owner player can teleport this entity
// ---------------------------------------------------------------------------

/// Present = entity can be teleported by other players.
#[derive(Debug, Clone, Copy)]
pub struct Teleportable(pub bool);

/// Items lying on the room floor.
#[derive(Debug, Clone, Default)]
pub struct FloorItems(pub Vec<crate::Entity>);

/// Marker component: ghost players can revive in this room without their corpse.
#[derive(Debug, Clone, Copy)]
pub struct RoomAllowRevive;

/// Maps a room entity to its content-defined spawn key (`"area_id:room_id"`).
/// Used at login to resolve a player's saved spawn point to an entity.
#[derive(Debug, Clone)]
pub struct SpawnKey(pub String);

impl Default for Teleportable {
    fn default() -> Self {
        Teleportable(true)
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

    #[test]
    fn test_portal_exit() {
        let e = crate::Entity::from(hecs::Entity::from_bits(0x0000_0001_0000_0002).unwrap());
        let portal = PortalExit::new(
            "sewer grate",
            e,
            "A rusty iron grate leads into darkness below.",
        );
        assert_eq!(portal.keyword, "sewer grate");
        assert!(!portal.is_hidden());

        let mut hidden = portal.clone();
        hidden.flags = PORTAL_HIDDEN;
        assert!(hidden.is_hidden());
    }

    #[test]
    fn test_room_exits_component() {
        let e = crate::Entity::from(hecs::Entity::from_bits(0x0000_0001_0000_0003).unwrap());
        let exits = RoomExits(vec![
            Exit::new(Direction::North, e),
            Exit::new(Direction::South, e),
        ]);
        assert_eq!(exits.0.len(), 2);
        assert_eq!(exits.0[0].direction, Direction::North);
    }

    #[test]
    fn test_room_portals_component() {
        let e = crate::Entity::from(hecs::Entity::from_bits(0x0000_0001_0000_0004).unwrap());
        let portals = RoomPortals(vec![
            PortalExit::new("sewer grate", e, "A grate."),
            PortalExit::new("painting", e, "A painting."),
        ]);
        assert_eq!(portals.0.len(), 2);
        assert_eq!(portals.0[0].keyword, "sewer grate");
        assert_eq!(portals.0[1].keyword, "painting");
    }

    #[test]
    fn test_room_flags_default() {
        let f = RoomFlags::default();
        assert!(f.0 == 0);
        assert!(!f.can_portal_in());
        assert!(!f.can_portal_out());
        assert!(!f.no_teleport_in());
        assert!(!f.no_teleport_out());
    }

    #[test]
    fn test_room_flags_portal_opt_in() {
        let f = RoomFlags(ROOM_PORTAL_IN | ROOM_PORTAL_OUT);
        assert!(f.can_portal_in());
        assert!(f.can_portal_out());
        assert!(!f.no_teleport_in());
        assert!(!f.no_teleport_out());
    }

    #[test]
    fn test_room_flags_teleport_opt_out() {
        let f = RoomFlags(ROOM_NO_TELEPORT_IN | ROOM_NO_TELEPORT_OUT);
        assert!(!f.can_portal_in());
        assert!(!f.can_portal_out());
        assert!(f.no_teleport_in());
        assert!(f.no_teleport_out());
    }

    #[test]
    fn test_teleportable_default() {
        let t = Teleportable::default();
        assert!(t.0);
    }

    #[test]
    fn test_room_flags_default_new() {
        let f = RoomFlags::default();
        assert!(f.0 == 0);
    }
}
