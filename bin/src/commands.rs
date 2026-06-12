use mud_core as core;
use mud_core::{Direction, Name, Position, Room, RoomExits, VoidRoom, World};
use mud_server::{Connection, ConnectionFlag, ConnectionRegistry};

fn send_formatted(conn: &mut dyn Connection, text: &core::format::RichText) {
    let ansi = conn.flags().has(ConnectionFlag::Ansi);
    let blink = conn.flags().has(ConnectionFlag::Blink);
    conn.send_line(&text.render(ansi, blink));
}

fn section_label(text: &str) -> core::format::Segment {
    core::format::Segment::colored(text, core::format::Color::BrightBlack)
}

fn get_pos_room(world: &World, entity: core::Entity) -> Option<core::Entity> {
    world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
}

fn get_room_name(world: &World, room: core::Entity) -> Option<String> {
    world
        .query_one::<&Room>(room)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.name.clone()))
}

fn get_room_desc(world: &World, room: core::Entity) -> Option<String> {
    world
        .query_one::<&Room>(room)
        .ok()
        .and_then(|mut q| q.get().map(|r| r.description.clone()))
}

fn get_name(world: &World, entity: core::Entity) -> Option<Name> {
    world
        .query_one::<&Name>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
}

fn is_void_room(world: &World, room: core::Entity) -> bool {
    world
        .query_one::<&VoidRoom>(room)
        .is_ok_and(|mut q| q.get().is_some())
}

fn get_exits(world: &World, room: core::Entity) -> Vec<&'static str> {
    let mut exits = Vec::new();
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(room_exits) = q.get() {
            for exit in &room_exits.0 {
                if !exit.is_hidden() {
                    exits.push(exit.direction.short_name());
                }
            }
        }
    }
    exits
}

pub fn cmd_look(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if is_void_room(world, room) {
        conn.send_line("");
        send_formatted(conn, &core::format::conventions::room_name("The Void"));
        send_formatted(conn, &core::format::conventions::separator("-".repeat(9)));
        conn.send_line("You are floating in an endless, featureless void.");
        conn.send_line("There is nothing here and no way out.");
        conn.send_line("");
        return;
    }

    let room_name = match get_room_name(world, room) {
        Some(n) => n,
        None => {
            conn.send_line("The void stares back.");
            return;
        }
    };

    let room_desc = get_room_desc(world, room).unwrap_or_default();

    conn.send_line("");
    send_formatted(conn, &core::format::conventions::room_name(&room_name));
    send_formatted(
        conn,
        &core::format::conventions::separator("-".repeat(room_name.len().min(40))),
    );
    conn.send_line(&room_desc);

    // Exits
    let exits = get_exits(world, room);
    if !exits.is_empty() {
        send_formatted(
            conn,
            &core::format::conventions::exit_dir(format!("[Exits: {}]", exits.join(" "))),
        );
    }

    // Occupants
    let others: Vec<_> = registry
        .occupants(world, room)
        .into_iter()
        .filter(|&e| e != entity)
        .collect();

    if !others.is_empty() {
        let mut t = core::format::RichText::new();
        t.push(section_label("Players here: "));
        for (i, &other) in others.iter().enumerate() {
            if i > 0 {
                t.push(core::format::Segment::new(", "));
            }
            if let Some(name) = get_name(world, other) {
                t.push(core::format::conventions::player_name_segment(
                    name.as_str(),
                ));
            }
        }
        send_formatted(conn, &t);
    }

    conn.send_line("");
}

pub fn cmd_say(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    registry: &ConnectionRegistry,
) {
    if args.is_empty() {
        conn.send_line("Say what?");
        return;
    }

    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if is_void_room(world, room) {
        conn.send_line("Your words echo in the void with no one to hear.");
        return;
    }

    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));

    // Speaker message
    let speaker_msg = core::format::conventions::say_text(format!("You say, \"{args}\""));
    send_formatted(conn, &speaker_msg);

    // Room broadcast
    let mut room_msg = core::format::RichText::new();
    room_msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    room_msg.push(core::format::Segment::new(format!(" says, \"{args}\"")));

    let rendered = room_msg.render(true, true);
    let bytes = format!("{}\r\n", rendered).into_bytes();

    let occupants = registry.occupants(world, room);
    for &other in &occupants {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
        }
    }
}

fn send_leave_broadcast(
    world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    from_room: core::Entity,
    dir_long: &str,
) {
    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));
    let mut msg = core::format::RichText::new();
    msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    msg.push(core::format::Segment::new(format!(" leaves {dir_long}.")));
    let rendered = msg.render(true, true);
    let bytes = format!("{}\r\n", rendered).into_bytes();
    for &other in &registry.occupants(world, from_room) {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
        }
    }
}

fn send_enter_broadcast(
    world: &World,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    dest_room: core::Entity,
    dir_long: &str,
) {
    let name = get_name(world, entity).unwrap_or(Name::new("Someone"));
    let mut msg = core::format::RichText::new();
    msg.push(core::format::conventions::player_name_segment(
        name.as_str(),
    ));
    msg.push(core::format::Segment::new(format!(
        " arrives from the {dir_long}."
    )));
    let rendered = msg.render(true, true);
    let bytes = format!("{}\r\n", rendered).into_bytes();
    for &other in &registry.occupants(world, dest_room) {
        if other == entity {
            continue;
        }
        if let Some(tx) = registry.sender(other) {
            let _ = tx.send(bytes.clone());
        }
    }
}

fn direction_from_name(name: &str) -> Option<Direction> {
    Direction::from_short(name).or_else(|| Direction::from_long(name))
}

fn move_player(
    world: &mut World,
    conn: &mut dyn Connection,
    registry: &ConnectionRegistry,
    entity: core::Entity,
    direction: Direction,
) {
    let room = match get_pos_room(world, entity) {
        Some(r) => r,
        None => {
            conn.send_line("You are nowhere.");
            return;
        }
    };

    if is_void_room(world, room) {
        conn.send_line("You cannot move in the void.");
        return;
    }

    // Find the exit
    let dest = match world.query_one::<&RoomExits>(room) {
        Ok(mut q) => q.get().and_then(|exits| {
            exits
                .0
                .iter()
                .find(|e| e.direction == direction)
                .map(|e| e.dest)
        }),
        Err(_) => None,
    };

    let dest = match dest {
        Some(d) => d,
        None => {
            conn.send_line("You cannot go that way.");
            return;
        }
    };

    // Check exit flags
    if let Ok(mut q) = world.query_one::<&RoomExits>(room) {
        if let Some(exits) = q.get() {
            if let Some(exit) = exits.0.iter().find(|e| e.direction == direction) {
                if exit.is_closed() {
                    conn.send_line("That exit is closed.");
                    return;
                }
                if exit.is_locked() {
                    conn.send_line("That exit is locked.");
                    return;
                }
            }
        }
    }

    // Move the player
    let _ = world.insert(entity, (Position::new(dest),));

    // Broadcast leave
    let dir_long = direction.long_name();
    let opposite = direction.opposite();
    let opp_long = opposite.long_name();
    send_leave_broadcast(world, registry, entity, room, dir_long);

    // Broadcast enter
    send_enter_broadcast(world, registry, entity, dest, opp_long);

    // Auto-look
    cmd_look(world, conn, "", "", registry);
}

pub fn cmd_move(
    world: &mut World,
    conn: &mut dyn Connection,
    name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let direction = match direction_from_name(name) {
        Some(d) => d,
        None => {
            conn.send_line("Huh?");
            return;
        }
    };

    move_player(world, conn, registry, entity, direction);
}

pub fn cmd_help(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("");
    conn.send_line("Available commands:");
    conn.send_line("  look/l         — examine your surroundings");
    conn.send_line("  say <text>     — speak in the room");
    conn.send_line("  north/n        — move north");
    conn.send_line("  south/s        — move south");
    conn.send_line("  east/e         — move east");
    conn.send_line("  west/w         — move west");
    conn.send_line("  up/u           — move up");
    conn.send_line("  down/d         — move down");
    conn.send_line("  help           — this help");
    conn.send_line("  quit           — disconnect");
    conn.send_line("");
}

pub fn cmd_quit(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("Goodbye!");
    conn.disconnect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use mud_core as core;
    use mud_core::Exit;
    use mud_server::{CharacterCreateBuffer, ConnectionFlags, ConnectionState};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// A mock connection that records all send_line calls.
    struct MockConnection {
        lines: RefCell<VecDeque<String>>,
        entity: RefCell<Option<core::Entity>>,
        disconnected: RefCell<bool>,
        flags: RefCell<ConnectionFlags>,
    }

    impl MockConnection {
        fn new() -> Self {
            MockConnection {
                lines: RefCell::new(VecDeque::new()),
                entity: RefCell::new(None),
                disconnected: RefCell::new(false),
                flags: RefCell::new(ConnectionFlags::new()),
            }
        }

        fn take_lines(&self) -> Vec<String> {
            self.lines.borrow_mut().drain(..).collect()
        }

        fn was_disconnected(&self) -> bool {
            *self.disconnected.borrow()
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
        fn id(&self) -> u64 {
            0
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
        fn flags(&self) -> ConnectionFlags {
            *self.flags.borrow()
        }
        fn set_flags(&mut self, flags: ConnectionFlags) {
            self.flags.borrow_mut().clone_from(&flags);
        }
        fn state(&self) -> ConnectionState {
            ConnectionState::Playing
        }
        fn set_state(&mut self, _state: ConnectionState) {}
        fn create_buffer(&mut self) -> &mut CharacterCreateBuffer {
            todo!()
        }
        fn account_id(&self) -> Option<i64> {
            None
        }
        fn set_account_id(&mut self, _id: i64) {}
        fn strikes(&self) -> u8 {
            0
        }
        fn set_strikes(&mut self, _n: u8) {}
        fn output_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
            None
        }
    }

    /// Build a test world with two rooms connected by exits.
    fn test_world() -> (World, core::Entity, core::Entity, core::Entity) {
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

    fn test_player(
        world: &mut World,
        room: core::Entity,
    ) -> (core::Entity, MockConnection, ConnectionRegistry) {
        let player = world.spawn((Position::new(room), Name::new("TestPlayer")));
        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let mut registry = ConnectionRegistry::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        registry.register(player, tx);
        (player, conn, registry)
    }

    // ── cmd_look ────────────────────────────────────────────

    #[test]
    fn test_look_in_room() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        let all = lines.join("|");
        assert!(
            all.contains("Room A"),
            "Expected 'Room A' in lines: {lines:?}"
        );
        assert!(lines.len() > 1, "expected more than one line: {lines:?}");
    }

    #[test]
    fn test_look_in_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("The Void")));
        assert!(lines
            .iter()
            .any(|l| l.contains("endless, featureless void")));
    }

    #[test]
    fn test_look_no_entity() {
        let (mut world, _void, _room_a, _room_b) = test_world();
        let mut conn = MockConnection::new(); // no entity set
        let registry = ConnectionRegistry::new();

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You have no form")));
    }

    #[test]
    fn test_look_no_position() {
        let (mut world, _void, _room_a, _room_b) = test_world();
        let player = world.spawn((Name::new("Ghost"),));
        let mut conn = MockConnection::new();
        conn.set_entity(player);
        let registry = ConnectionRegistry::new();

        cmd_look(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You are nowhere")));
    }

    // ── cmd_say ─────────────────────────────────────────────

    #[test]
    fn test_say_empty_args() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_say(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Say what")));
    }

    #[test]
    fn test_say_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_say(&mut world, &mut conn, "", "hello", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("echo in the void")));
    }

    #[test]
    fn test_say_broadcasts_to_room() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_say(&mut world, &mut conn, "", "Hi there!", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("You say")));
        assert!(lines.iter().any(|l| l.contains("Hi there")));
    }

    #[test]
    fn test_say_echoes_to_other_occupants() {
        let (mut world, _void, room_a, _room_b) = test_world();

        let speaker = world.spawn((Position::new(room_a), Name::new("Speaker")));
        let mut conn_speaker = MockConnection::new();
        conn_speaker.set_entity(speaker);

        let listener = world.spawn((Position::new(room_a), Name::new("Listener")));
        let (tx_speaker, _rx_speaker) = tokio::sync::mpsc::unbounded_channel();
        let (tx_listener, mut rx_listener) = tokio::sync::mpsc::unbounded_channel();

        let mut registry = ConnectionRegistry::new();
        registry.register(speaker, tx_speaker);
        registry.register(listener, tx_listener);

        cmd_say(&mut world, &mut conn_speaker, "", "Hello room!", &registry);

        // Listener should have received the broadcast
        let received = rx_listener.try_recv().ok();
        assert!(received.is_some(), "listener should receive broadcast");
        if let Some(bytes) = received {
            let msg = String::from_utf8_lossy(&bytes);
            assert!(msg.contains("Speaker"));
            assert!(msg.contains("Hello room"));
        }
    }

    // ── Movement ────────────────────────────────────────────

    #[test]
    fn test_move_valid_direction() {
        let (mut world, _void, room_a, room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        let player_room = pos.get().unwrap().room;
        assert_eq!(player_room, room_b);
    }

    #[test]
    fn test_move_invalid_direction() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "north", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("cannot go that way")));
    }

    #[test]
    fn test_move_in_void() {
        let (mut world, void_room, _room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, void_room);

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("cannot move in the void")));
    }

    #[test]
    fn test_move_no_entity() {
        let (mut world, _void, _room_a, _room_b) = test_world();
        let mut conn = MockConnection::new();
        let registry = ConnectionRegistry::new();

        cmd_move(&mut world, &mut conn, "east", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("have no form")));
    }

    #[test]
    fn test_move_closed_exit() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        // Close the east exit from room_a
        let mut q = world.query_one::<&mut RoomExits>(room_a).unwrap();
        if let Some(exits) = q.get() {
            exits.0[0].set_closed(true);
        }
        drop(q);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("closed")));
        // Position should not have changed
        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_a);
    }

    #[test]
    fn test_move_locked_exit() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        // Lock the east exit from room_a
        let mut q = world.query_one::<&mut RoomExits>(room_a).unwrap();
        if let Some(exits) = q.get() {
            exits.0[0].set_locked(true);
        }
        drop(q);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("locked")));
    }

    #[test]
    fn test_move_uses_short_name() {
        let (mut world, _void, room_a, room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_b);
    }

    #[test]
    fn test_move_broadcasts_leave_and_enter() {
        let (mut world, _void, room_a, room_b) = test_world();

        let mover = world.spawn((Position::new(room_a), Name::new("Mover")));
        let mut conn_mover = MockConnection::new();
        conn_mover.set_entity(mover);

        // Another player in room_a to receive leave broadcast
        let observer_a = world.spawn((Position::new(room_a), Name::new("ObserverA")));
        let (tx_obs_a, mut rx_obs_a) = tokio::sync::mpsc::unbounded_channel();

        // Another player in room_b to receive enter broadcast
        let observer_b = world.spawn((Position::new(room_b), Name::new("ObserverB")));
        let (tx_obs_b, mut rx_obs_b) = tokio::sync::mpsc::unbounded_channel();

        let mut registry = ConnectionRegistry::new();
        let (tx_mover, _rx_mover) = tokio::sync::mpsc::unbounded_channel();
        registry.register(mover, tx_mover);
        registry.register(observer_a, tx_obs_a);
        registry.register(observer_b, tx_obs_b);

        cmd_move(&mut world, &mut conn_mover, "east", "", &registry);

        // Observer A should see "Mover leaves east."
        let msg_a = rx_obs_a.try_recv().ok();
        assert!(msg_a.is_some(), "observer A should receive leave broadcast");
        if let Some(bytes) = msg_a {
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("Mover"));
            assert!(text.contains("leaves east"));
        }

        // Observer B should see "Mover arrives from the west."
        let msg_b = rx_obs_b.try_recv().ok();
        assert!(msg_b.is_some(), "observer B should receive enter broadcast");
        if let Some(bytes) = msg_b {
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("Mover"));
            assert!(text.contains("arrives"));
            assert!(text.contains("west"));
        }

        // Mover should see auto-look (room B info)
        let mover_lines = conn_mover.take_lines();
        assert!(mover_lines.iter().any(|l| l.contains("Room B")));
    }

    // ── cmd_help ────────────────────────────────────────────

    #[test]
    fn test_help_shows_commands() {
        let mut conn = MockConnection::new();
        let registry = ConnectionRegistry::new();
        let mut world = World::new();

        cmd_help(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Available commands")));
        assert!(lines.iter().any(|l| l.contains("look")));
        assert!(lines.iter().any(|l| l.contains("say")));
    }

    // ── cmd_quit ────────────────────────────────────────────

    #[test]
    fn test_quit_says_goodbye_and_disconnects() {
        let mut conn = MockConnection::new();
        let registry = ConnectionRegistry::new();
        let mut world = World::new();

        cmd_quit(&mut world, &mut conn, "", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Goodbye")));
        assert!(conn.was_disconnected());
    }

    // ── direction_from_name ─────────────────────────────────

    #[test]
    fn test_direction_from_short_name() {
        assert_eq!(direction_from_name("n"), Some(Direction::North));
        assert_eq!(direction_from_name("s"), Some(Direction::South));
        assert_eq!(direction_from_name("e"), Some(Direction::East));
        assert_eq!(direction_from_name("w"), Some(Direction::West));
        assert_eq!(direction_from_name("u"), Some(Direction::Up));
        assert_eq!(direction_from_name("d"), Some(Direction::Down));
        assert_eq!(direction_from_name("ne"), Some(Direction::Northeast));
        assert_eq!(direction_from_name("nw"), Some(Direction::Northwest));
        assert_eq!(direction_from_name("se"), Some(Direction::Southeast));
        assert_eq!(direction_from_name("sw"), Some(Direction::Southwest));
    }

    #[test]
    fn test_direction_from_long_name() {
        assert_eq!(direction_from_name("north"), Some(Direction::North));
        assert_eq!(direction_from_name("south"), Some(Direction::South));
        assert_eq!(direction_from_name("east"), Some(Direction::East));
    }

    #[test]
    fn test_direction_from_unknown_name() {
        assert_eq!(direction_from_name("sideways"), None);
        assert_eq!(direction_from_name(""), None);
    }

    // ── cmd_move with direction_from_name ───────────────────

    #[test]
    fn test_move_with_short_name() {
        let (mut world, _void, room_a, room_b) = test_world();
        let (player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "e", "", &registry);

        let mut pos = world.query_one::<&Position>(player).unwrap();
        assert_eq!(pos.get().unwrap().room, room_b);
    }

    #[test]
    fn test_move_huh_for_bad_name() {
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);

        cmd_move(&mut world, &mut conn, "sideways", "", &registry);

        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("Huh")));
    }
}
