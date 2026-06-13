use crate::{Entity, EquipmentSlot};

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
    MobAttacked {
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
    ItemWorn {
        player: Entity,
        item: Entity,
        slot: EquipmentSlot,
    },
    ItemRemoved {
        player: Entity,
        item: Entity,
        slot: EquipmentSlot,
    },
    RoomEntered {
        actor: Entity,
        room: Entity,
    },
    PlayerLeveled {
        entity: Entity,
        old_level: u8,
        new_level: u8,
    },
    CorpseDecayed {
        corpse: Entity,
        room: Entity,
    },
    SetBonusChanged {
        player: Entity,
        set_id: String,
        active_tiers: Vec<u8>,
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
