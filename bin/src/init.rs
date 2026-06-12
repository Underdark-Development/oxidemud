use mud_core::{Entity, Position, Room, VoidRoom, World};

pub fn init_world() -> (World, Entity) {
    let mut world = World::new();

    let void_room = world.spawn((
        Room::new("The Void", "You are floating in a void"),
        VoidRoom,
    ));

    world
        .insert(void_room, (Position::new(void_room),))
        .expect("void room should exist");

    (world, void_room)
}
