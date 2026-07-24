#![allow(unused)]

use oxide_core as core;
use oxide_core::{Direction, Exit, Name, Position, Room, RoomExits, VoidRoom, World};
use oxide_server::{Connection, ConnectionFlags, ConnectionRegistry};
use std::cell::RefCell;
use std::collections::VecDeque;

/// A mock connection that records all send_line calls.
pub struct MockConnection {
    pub lines: RefCell<VecDeque<String>>,
    pub entity: RefCell<Option<core::Entity>>,
    pub disconnected: RefCell<bool>,
    pub flags: RefCell<ConnectionFlags>,
    pub screen_width: RefCell<u16>,
    pub access_level: RefCell<core::AccessLevel>,
}

impl MockConnection {
    pub fn new() -> Self {
        MockConnection {
            lines: RefCell::new(VecDeque::new()),
            entity: RefCell::new(None),
            disconnected: RefCell::new(false),
            flags: RefCell::new(ConnectionFlags::new()),
            screen_width: RefCell::new(0),
            access_level: RefCell::new(core::AccessLevel::Player),
        }
    }

    pub fn take_lines(&self) -> Vec<String> {
        self.lines.borrow_mut().drain(..).collect()
    }

    pub fn was_disconnected(&self) -> bool {
        *self.disconnected.borrow()
    }
}

impl Default for MockConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection for MockConnection {
    fn send_line(&mut self, text: &str) {
        self.lines.borrow_mut().push_back(text.to_string());
    }
    fn send(&mut self, text: &str) {
        self.lines
            .borrow_mut()
            .push_back(format!("[inline] {text}"));
    }
    fn send_raw(&mut self, _bytes: &[u8]) {}
    fn id(&self) -> &str {
        "0"
    }
    fn entity(&self) -> Option<core::Entity> {
        *self.entity.borrow()
    }
    fn set_entity(&mut self, entity: core::Entity) {
        self.entity.borrow_mut().replace(entity);
    }
    fn disconnect(&mut self) {
        self.disconnected.borrow_mut().clone_from(&true);
    }
    fn is_disconnected(&self) -> bool {
        *self.disconnected.borrow()
    }
    fn flags(&self) -> ConnectionFlags {
        *self.flags.borrow()
    }
    fn set_flags(&mut self, flags: ConnectionFlags) {
        self.flags.borrow_mut().clone_from(&flags);
    }
    fn screen_width(&self) -> u16 {
        *self.screen_width.borrow()
    }
    fn set_screen_width(&mut self, width: u16) {
        *self.screen_width.borrow_mut() = width;
    }
    fn access_level(&self) -> core::AccessLevel {
        *self.access_level.borrow()
    }
    fn set_access_level(&mut self, level: core::AccessLevel) {
        *self.access_level.borrow_mut() = level;
    }
    fn output_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
        None
    }
}

/// Build a test world with two rooms connected by exits.
pub fn test_world() -> (World, core::Entity, core::Entity, core::Entity) {
    let mut world = World::new();

    let void_room = world.spawn((Room::new("The Void", "Empty void."), VoidRoom));

    let room_a = world.spawn((
        Room::new("Room A", "This is room A."),
        RoomExits(vec![Exit::new(Direction::East, void_room)]), // placeholder
    ));
    let room_b = world.spawn((
        Room::new("Room B", "This is room B."),
        RoomExits(vec![Exit::new(Direction::West, void_room)]), // placeholder
    ));

    // Fix exits with real dests
    let mut q_a = world.query_one::<&mut RoomExits>(room_a).unwrap();
    if let Some(exits) = q_a.get() {
        exits.0[0] = Exit::new(Direction::East, room_b);
    }
    drop(q_a);

    let mut q_b = world.query_one::<&mut RoomExits>(room_b).unwrap();
    if let Some(exits) = q_b.get() {
        exits.0[0] = Exit::new(Direction::West, room_a);
    }
    drop(q_b);

    (world, void_room, room_a, room_b)
}

pub fn test_player(
    world: &mut World,
    room: core::Entity,
) -> (core::Entity, MockConnection, ConnectionRegistry) {
    let player = world.spawn((
        Position::new(room),
        Name::new("TestPlayer"),
        core::Player::new(1),
    ));
    let mut conn = MockConnection::new();
    conn.set_entity(player);
    let mut registry = ConnectionRegistry::new();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    registry.register(player, tx);
    (player, conn, registry)
}

pub static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn init_test_templates() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    oxide_server::config::init(std::path::Path::new(""));
    let mut registry = core::templates::TemplateRegistry::new();

    let solaris = core::templates::DeityTemplate {
        id: "solaris".into(),
        name: "Solaris".into(),
        description: "The sun god.".into(),
        alignment: Some("lawful_good".into()),
        symbol: "Sunburst".into(),
        favored_weapon: None,
        tenets: vec![],
        domains: vec![],
        allowed_races: vec![],
        allowed_classes: vec![],
        allowed_alignments: vec![],
        prayer_effect: Some(core::templates::PrayerEffect {
            buff_id: "sun_blessing".into(),
            duration_secs: 60,
            cooldown_secs: 2,
            description: "Solar blessing".into(),
        }),
        params: std::collections::HashMap::new(),
    };
    registry.deities.insert("solaris".into(), solaris);

    // Two-handed weapon template
    let two_handed_tmpl = core::templates::ItemTemplate {
        id: "two_handed_sword".to_string(),
        name: "Greatsword".to_string(),
        description: "A heavy two-handed sword.".to_string(),
        item_type: "weapon".to_string(),
        subtype: "sword".to_string(),
        quality: "common".to_string(),
        level_requirement: 1,
        weight: 8.0,
        value: 100,
        flags: vec![],
        allowed_classes: vec![],
        allowed_races: vec![],
        allowed_alignments: vec![],
        requires_skill: None,
        weapon: Some(core::templates::WeaponDef {
            damage: core::templates::DiceString("2d6".to_string()),
            damage_type: "slash".to_string(),
            speed: 2.0,
            range: "melee".to_string(),
            hands: "TwoHand".to_string(),
        }),
        equipment: None,
        set: None,
        triggers: vec![],
        params: std::collections::HashMap::new(),
        ..Default::default()
    };
    registry
        .items
        .insert("two_handed_sword".to_string(), two_handed_tmpl);

    // Shield template
    let shield_tmpl = core::templates::ItemTemplate {
        id: "wooden_shield".to_string(),
        name: "Wooden Shield".to_string(),
        description: "A simple wooden shield.".to_string(),
        item_type: "armor".to_string(),
        subtype: "shield".to_string(),
        quality: "common".to_string(),
        level_requirement: 1,
        weight: 5.0,
        value: 10,
        flags: vec![],
        allowed_classes: vec![],
        allowed_races: vec![],
        allowed_alignments: vec![],
        requires_skill: None,
        weapon: None,
        equipment: Some(core::templates::EquipmentDef {
            slot: "shield".to_string(),
        }),
        set: None,
        triggers: vec![],
        params: std::collections::HashMap::new(),
        ..Default::default()
    };
    registry
        .items
        .insert("wooden_shield".to_string(), shield_tmpl);

    // Skill templates for testing
    let heal_skill = core::SkillDef {
        id: "potion_minor_heal".into(),
        name: "Minor Healing Potion".into(),
        description: "Heals target.".into(),
        skill_type: core::SkillType::Craft,
        max_rank: 1,
        level_requirement: 1,
        cooldown_secs: 0,
        targeting: core::Targeting::SelfTarget,
        cost: core::ResourceCost::None,
        effect: Some(core::EffectTemplate::Heal { dice: "2d6".into() }),
        allowed_classes: vec![],
        allowed_races: vec![],
        requires_skill: None,
        must_train: false,
        trainer_types: vec![],
        use_while_fighting: true,
        use_while_sitting: true,
        script: None,
        params: std::collections::HashMap::new(),
    };
    registry
        .skills
        .insert("potion_minor_heal".into(), heal_skill);

    let scroll_fireball = core::SkillDef {
        id: "scroll_fireball".into(),
        name: "Scroll of Fireball".into(),
        description: "Hurls a fireball.".into(),
        skill_type: core::SkillType::Magic,
        max_rank: 1,
        level_requirement: 1,
        cooldown_secs: 0,
        targeting: core::Targeting::Single { range: 1 },
        cost: core::ResourceCost::None,
        effect: Some(core::EffectTemplate::Damage { dice: "3d6".into() }),
        allowed_classes: vec![],
        allowed_races: vec![],
        requires_skill: None,
        must_train: false,
        trainer_types: vec![],
        use_while_fighting: true,
        use_while_sitting: false,
        script: None,
        params: std::collections::HashMap::new(),
    };
    registry
        .skills
        .insert("scroll_fireball".into(), scroll_fireball);

    let spell_fireball = core::SkillDef {
        id: "spell_fireball".into(),
        name: "Fireball".into(),
        description: "Hurls a fireball.".into(),
        skill_type: core::SkillType::Magic,
        max_rank: 1,
        level_requirement: 1,
        cooldown_secs: 0,
        targeting: core::Targeting::Single { range: 1 },
        cost: core::ResourceCost::None,
        effect: Some(core::EffectTemplate::Damage { dice: "3d6".into() }),
        allowed_classes: vec![],
        allowed_races: vec![],
        requires_skill: None,
        must_train: false,
        trainer_types: vec![],
        use_while_fighting: true,
        use_while_sitting: false,
        script: None,
        params: std::collections::HashMap::new(),
    };
    registry
        .skills
        .insert("spell_fireball".into(), spell_fireball);

    let world = World::new();
    let mut server = oxide_server::Server::new("127.0.0.1:0", world).with_templates(registry);
    super::register_all_commands(&mut server);
    guard
}
