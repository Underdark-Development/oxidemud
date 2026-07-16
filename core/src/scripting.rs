use crate::{DamageType, Entity, World};
use std::sync::OnceLock;

/// Context passed to and returned from combat hit scripts.
#[derive(Debug, Clone)]
pub struct HitContext {
    pub attacker: Entity,
    pub target: Entity,
    pub is_offhand: bool,
    pub is_aborted: bool,
    pub abort_reason: Option<String>,
    pub hit_modifier: i32,
    pub override_hit: Option<bool>,
}

/// Interface for executing scripts in the scripting layer.
pub trait ScriptingBridge: Send + Sync {
    /// Execute a generic trigger script.
    fn execute_trigger(
        &self,
        script: &str,
        entity: Entity,
        actor: Option<Entity>,
        target: Option<Entity>,
        world: &mut World,
    ) -> Result<bool, String>;

    /// Execute a combat hit hook to check for defenses/modifiers.
    fn execute_combat_hit_hook(
        &self,
        attacker: Entity,
        target: Entity,
        is_offhand: bool,
        world: &mut World,
    ) -> Result<HitContext, String>;

    /// Execute a combat damage hook to modify damage amount or type.
    fn execute_combat_damage_hook(
        &self,
        attacker: Entity,
        target: Entity,
        damage: i32,
        damage_type: DamageType,
        world: &mut World,
    ) -> Result<(i32, DamageType), String>;

    /// Execute a custom AI script for an NPC entity.
    fn execute_mob_ai(
        &self,
        script: &str,
        entity: Entity,
        world: &mut World,
    ) -> Result<Option<String>, String>;

    /// Execute a say script when a player speaks in a room.
    fn execute_say_hook(
        &self,
        script_entity: Entity,
        speaker: Entity,
        message: &str,
        world: &mut World,
    ) -> Result<(), String>;

    /// Execute an active skill script.
    fn execute_use_skill(
        &self,
        script: &str,
        actor: Entity,
        target: Option<Entity>,
        world: &mut World,
    ) -> Result<(), String>;

    /// Reload a script AST cache, compiling or removing it.
    fn reload_script(&self, _rel_path: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Interface for sending messages from the scripting layer to players/rooms.
pub trait MessageOutputBridge: Send + Sync {
    /// Send a direct message line to a specific connection/entity.
    fn send_to_entity(&self, entity: Entity, message: &str);

    /// Echo a message line to all occupants of a specific room.
    fn echo_to_room(&self, room: Entity, message: &str);
}

pub static SCRIPTING_BRIDGE: OnceLock<Box<dyn ScriptingBridge>> = OnceLock::new();
pub static MESSAGE_BRIDGE: OnceLock<Box<dyn MessageOutputBridge>> = OnceLock::new();

/// Register the global scripting implementation.
pub fn register_scripting_bridge(bridge: Box<dyn ScriptingBridge>) {
    let _ = SCRIPTING_BRIDGE.set(bridge);
}

/// Register the global messaging/broadcast implementation.
pub fn register_message_bridge(bridge: Box<dyn MessageOutputBridge>) {
    let _ = MESSAGE_BRIDGE.set(bridge);
}

/// Retrieve the active scripting implementation.
pub fn get_scripting_bridge() -> Option<&'static dyn ScriptingBridge> {
    SCRIPTING_BRIDGE.get().map(|b| b.as_ref())
}

/// Retrieve the active messaging implementation.
pub fn get_message_bridge() -> Option<&'static dyn MessageOutputBridge> {
    MESSAGE_BRIDGE.get().map(|b| b.as_ref())
}
