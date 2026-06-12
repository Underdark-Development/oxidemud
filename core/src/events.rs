use crate::Entity;

#[derive(Debug, Clone)]
pub enum GameEvent {
    PlayerConnected {
        entity: Entity,
    },
    PlayerDisconnected {
        entity: Entity,
    },
    PlayerSaid {
        speaker: Entity,
        message: String,
    },
    PlayerMoved {
        player: Entity,
        from: Entity,
        to: Entity,
    },
    PlayerAttacked {
        attacker: Entity,
        target: Entity,
    },
    PlayerDied {
        victim: Entity,
        killer: Option<Entity>,
    },
    MobDied {
        mob: Entity,
        killer: Entity,
    },
    ItemPickedUp {
        player: Entity,
        item: Entity,
    },
    ItemDropped {
        player: Entity,
        item: Entity,
    },
    RoomEntered {
        actor: Entity,
        room: Entity,
    },
    ScriptTrigger {
        entity: Entity,
        trigger: TriggerType,
    },
}

#[derive(Debug, Clone)]
pub enum TriggerType {
    Death,
    Enter,
    Leave,
    Say,
    Combat,
    Timer,
    Custom(String),
}
