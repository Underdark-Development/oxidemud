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

    /// Execute a quest event hook (e.g. on_accept, on_complete, on_update).
    fn execute_quest_hook(
        &self,
        script: &str,
        player: Entity,
        quest_id: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _ = script;
        let _ = player;
        let _ = quest_id;
        let _ = world;
        Ok(())
    }

    /// Execute a dynamic skill/spell script.
    fn execute_script_skill(
        &self,
        script: &str,
        actor: Entity,
        args: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _ = script;
        let _ = actor;
        let _ = args;
        let _ = world;
        Ok(())
    }

    /// Execute an entity command script.
    fn execute_entity_command(
        &self,
        entity: Entity,
        script: &str,
        actor: Entity,
        args: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _ = entity;
        let _ = script;
        let _ = actor;
        let _ = args;
        let _ = world;
        Ok(())
    }

    /// Evaluate a script predicate returning boolean result.
    fn evaluate_script_predicate(
        &self,
        script: &str,
        actor: Entity,
        target_entity: Option<Entity>,
        world: &mut World,
    ) -> Result<bool, String> {
        let _ = script;
        let _ = actor;
        let _ = target_entity;
        let _ = world;
        Ok(true)
    }

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

    /// Echo a message line to occupants of a specific room, excluding specified entities.
    fn echo_to_room_except(&self, room: Entity, message: &str, exclude: &[Entity]);
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

use crate::components::CommandRestrictions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Dynamic skill or spell registered by a script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptSkill {
    pub id: String,
    pub name: String,
    pub command: Option<String>,
    pub is_spell: bool,
    pub topic: String,
    pub help_text: String,
    pub script: String,
    #[serde(default)]
    pub restrictions: CommandRestrictions,
}

/// Thread-safe registry for dynamically registered skills and spells.
#[derive(Debug, Clone, Default)]
pub struct DynamicSkillRegistry {
    pub skills: HashMap<String, ScriptSkill>,
    pub direct_commands: HashMap<String, String>, // lowercase command name -> skill id
    pub spells: HashMap<String, String>,          // lowercase spell name -> skill id
}

impl DynamicSkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, skill: ScriptSkill) {
        let id = skill.id.clone();
        if let Some(cmd) = &skill.command {
            let cmd_lower = cmd.to_lowercase();
            if skill.is_spell {
                self.spells.insert(cmd_lower, id.clone());
            } else {
                self.direct_commands.insert(cmd_lower, id.clone());
            }
        }
        self.skills.insert(id, skill);
    }

    pub fn find_direct_command(&self, name: &str) -> Option<&ScriptSkill> {
        let name_lower = name.to_lowercase();
        if let Some(id) = self.direct_commands.get(&name_lower) {
            return self.skills.get(id);
        }
        for (cmd, id) in &self.direct_commands {
            if cmd.starts_with(&name_lower) {
                return self.skills.get(id);
            }
        }
        None
    }

    pub fn find_spell(&self, name: &str) -> Option<&ScriptSkill> {
        let name_lower = name.to_lowercase();
        if let Some(id) = self.spells.get(&name_lower) {
            return self.skills.get(id);
        }
        for (spell, id) in &self.spells {
            if spell.starts_with(&name_lower) {
                return self.skills.get(id);
            }
        }
        None
    }

    pub fn find_by_name_or_command(&self, query: &str) -> Option<&ScriptSkill> {
        let q_lower = query.to_lowercase();
        if let Some(s) = self.find_direct_command(&q_lower) {
            return Some(s);
        }
        if let Some(s) = self.find_spell(&q_lower) {
            return Some(s);
        }
        self.skills
            .values()
            .find(|s| s.id.to_lowercase() == q_lower || s.name.to_lowercase() == q_lower)
    }

    pub fn topics(&self) -> Vec<String> {
        let mut t: Vec<String> = self.skills.values().map(|s| s.topic.clone()).collect();
        t.sort();
        t.dedup();
        t
    }

    pub fn skills_for_topic(&self, topic: &str) -> Vec<&ScriptSkill> {
        self.skills
            .values()
            .filter(|s| s.topic.eq_ignore_ascii_case(topic))
            .collect()
    }
}

static DYNAMIC_SKILLS: OnceLock<RwLock<DynamicSkillRegistry>> = OnceLock::new();

pub fn register_dynamic_skill(skill: ScriptSkill) {
    let registry = DYNAMIC_SKILLS.get_or_init(|| RwLock::new(DynamicSkillRegistry::new()));
    if let Ok(mut writer) = registry.write() {
        writer.register(skill);
    }
}

pub fn with_dynamic_skills<R>(f: impl FnOnce(&DynamicSkillRegistry) -> R) -> R {
    let registry = DYNAMIC_SKILLS.get_or_init(|| RwLock::new(DynamicSkillRegistry::new()));
    let reader = registry.read().unwrap();
    f(&reader)
}
