use std::collections::HashMap;

use tokio::sync::mpsc;

use mud_core::{Entity, World};

/// Maps player entities to their output channel senders for room broadcasts.
pub struct ConnectionRegistry {
    map: HashMap<Entity, mpsc::UnboundedSender<Vec<u8>>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        ConnectionRegistry {
            map: HashMap::new(),
        }
    }

    pub fn register(&mut self, entity: Entity, tx: mpsc::UnboundedSender<Vec<u8>>) {
        self.map.insert(entity, tx);
    }

    pub fn unregister(&mut self, entity: Entity) {
        self.map.remove(&entity);
    }

    pub fn is_connected(&self, entity: Entity) -> bool {
        self.map.contains_key(&entity)
    }

    /// Get the sender for a specific entity, if registered.
    pub fn sender(&self, entity: Entity) -> Option<&mpsc::UnboundedSender<Vec<u8>>> {
        self.map.get(&entity)
    }

    /// Broadcast a message to all players in the given room, optionally excluding one entity.
    pub fn broadcast_to_room(
        &self,
        world: &World,
        room: Entity,
        message: &str,
        exclude: Option<Entity>,
    ) {
        let mut q = world.query::<(&mud_core::Position,)>();
        let bytes = message.as_bytes().to_vec();
        for (raw, (pos,)) in q.iter() {
            let entity = Entity::from(raw);
            if pos.room != room {
                continue;
            }
            if exclude == Some(entity) {
                continue;
            }
            if let Some(tx) = self.map.get(&entity) {
                let _ = tx.send(bytes.clone());
            }
        }
    }

    /// Return all connected player entities.
    pub fn connected_entities(&self) -> Vec<Entity> {
        self.map.keys().copied().collect()
    }

    /// Number of currently connected players.
    pub fn player_count(&self) -> usize {
        self.map.len()
    }

    /// Return all connected player entities in the given room.
    pub fn occupants(&self, world: &World, room: Entity) -> Vec<Entity> {
        let mut q = world.query::<(&mud_core::Position,)>();
        q.iter()
            .map(|(raw, (pos,))| (Entity::from(raw), pos))
            .filter(|(entity, pos)| pos.room == room && self.map.contains_key(entity))
            .map(|(entity, _)| entity)
            .collect()
    }
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mud_core::{Name, Position, Room, VoidRoom};

    fn setup_world() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();

        let void_room = world.spawn((Room::new("The Void", "Empty"), VoidRoom));
        let room_a = world.spawn(());
        let room_b = world.spawn(());

        (world, void_room, room_a, room_b)
    }

    #[test]
    fn test_register_and_unregister() {
        let mut registry = ConnectionRegistry::new();
        let mut world = World::new();
        let dummy_room = world.spawn(());
        let player = world.spawn((Position::new(dummy_room), Name::new("Test")));
        let (tx, _rx) = mpsc::unbounded_channel();

        registry.register(player, tx);
        assert!(registry.is_connected(player));

        registry.unregister(player);
        assert!(!registry.is_connected(player));
    }

    #[test]
    fn test_occupants_in_room() {
        let (mut world, _void, room_a, room_b) = setup_world();
        let mut registry = ConnectionRegistry::new();

        let player_a = world.spawn((Position::new(room_a), Name::new("A")));
        let player_b = world.spawn((Position::new(room_b), Name::new("B")));

        let (tx_a, _) = mpsc::unbounded_channel();
        let (tx_b, _) = mpsc::unbounded_channel();
        registry.register(player_a, tx_a);
        registry.register(player_b, tx_b);

        let occ = registry.occupants(&world, room_a);
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0], player_a);

        let occ_b = registry.occupants(&world, room_b);
        assert_eq!(occ_b.len(), 1);
        assert_eq!(occ_b[0], player_b);
    }

    #[test]
    fn test_broadcast_to_room() {
        let (mut world, _void, room_a, _room_b) = setup_world();
        let mut registry = ConnectionRegistry::new();

        let player = world.spawn((Position::new(room_a), Name::new("P")));
        let (tx, mut rx) = mpsc::unbounded_channel();
        registry.register(player, tx);

        registry.broadcast_to_room(&world, room_a, "hello", None);

        if let Ok(bytes) = rx.try_recv() {
            let msg = String::from_utf8_lossy(&bytes).to_string();
            assert_eq!(msg, "hello");
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn test_broadcast_excludes_sender() {
        let (mut world, _void, room_a, _room_b) = setup_world();
        let mut registry = ConnectionRegistry::new();

        let sender = world.spawn((Position::new(room_a), Name::new("Sender")));
        let (tx_s, mut rx_s) = mpsc::unbounded_channel();
        registry.register(sender, tx_s);

        world.spawn((Position::new(room_a), Name::new("Other")));

        registry.broadcast_to_room(&world, room_a, "hello", Some(sender));

        assert!(rx_s.try_recv().is_err());
    }
}
