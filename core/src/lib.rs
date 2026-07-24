mod components;
pub mod content;
pub mod dice;
pub mod format;
pub mod prompt;
pub mod regen;
mod resources;
pub mod script_dispatch;
pub mod scripting;
pub mod systems;
pub mod templates;
pub mod trie;
pub mod util;

pub use components::*;
pub use prompt::PromptVars;
pub use resources::*;
pub use scripting::{
    get_message_bridge, get_scripting_bridge, register_dynamic_skill, with_dynamic_skills,
    DynamicSkillRegistry, HitContext, MessageOutputBridge, ScriptSkill, ScriptingBridge,
};
pub use systems::ai::AiState;
pub use systems::combat::{transition_combat_state, HitResult};
pub use systems::crafting::{can_craft_recipe, craft_recipe};
pub use systems::faction::{adjust_faction_standing, handle_faction_kill};
pub use systems::group::{
    handle_group_accept, handle_group_disband, handle_group_formation, handle_group_invite,
    handle_group_kick, handle_group_leader, handle_group_leave, handle_group_loot,
    handle_player_disconnect_group, handle_player_login_group, run_formation_effects,
    run_group_cleanup,
};
pub use systems::multi_class::{calculate_multiclass_combat_stats, satisfies_prestige_gate};
pub use systems::player_state::{
    run_player_state_decay, transition_player_state, try_transition_player_state,
    PlayerStateTrigger,
};
pub use systems::quest::{
    abandon_quest, accept_quest, complete_quest, handle_explore_event, handle_kill_event,
    handle_talk_event, reconcile_gather_objectives,
};
pub use systems::skill_gate::run_skill_gate_pulse;
pub use systems::skill_use::{
    apply_skill_effect, can_use_skill, deduct_resource_cost, get_modified_attributes,
    run_cooldown_decay, run_temporary_effect_decay,
};
pub use systems::time::TimeConfig;
pub use systems::trigger::{ItemTriggers, TriggeredEffect};
pub use templates::{
    ExitTemplate, FactionDef, FactionRank, PrestigeGate, RecipeDef, RecipeMaterial, RecipeResult,
    RecipeSkillReq,
};
pub use util::{
    entities_in_room, get_entity_name, get_exits, get_name, get_pos_room, get_room_desc,
    get_room_name, get_short_desc, is_void_room,
};

use hecs as _hecs;

pub struct World {
    inner: _hecs::World,
}

impl World {
    pub fn new() -> Self {
        World {
            inner: _hecs::World::new(),
        }
    }

    pub fn spawn(&mut self, bundle: impl _hecs::DynamicBundle) -> Entity {
        Entity(self.inner.spawn(bundle))
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<(), _hecs::NoSuchEntity> {
        self.inner.despawn(entity.0)
    }

    pub fn query<T: _hecs::Query>(&self) -> _hecs::QueryBorrow<'_, T> {
        self.inner.query::<T>()
    }

    pub fn query_one<T: _hecs::Query>(
        &self,
        entity: Entity,
    ) -> Result<_hecs::QueryOne<'_, T>, _hecs::NoSuchEntity> {
        self.inner.query_one::<T>(entity.0)
    }

    pub fn insert<T: _hecs::DynamicBundle>(
        &mut self,
        entity: Entity,
        bundle: T,
    ) -> Result<(), _hecs::NoSuchEntity> {
        self.inner.insert(entity.0, bundle)
    }

    pub fn remove_one<T: _hecs::Component>(
        &mut self,
        entity: Entity,
    ) -> Result<T, _hecs::ComponentError> {
        self.inner.remove_one::<T>(entity.0)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(_hecs::Entity);

impl Entity {
    pub fn id(&self) -> u32 {
        self.0.id()
    }
}

impl From<_hecs::Entity> for Entity {
    fn from(e: _hecs::Entity) -> Self {
        Entity(e)
    }
}
