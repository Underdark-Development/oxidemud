pub mod components;
pub mod content;
pub mod dice;
pub mod format;
pub mod prompt;
pub mod regen;
pub mod resources;
pub mod script_dispatch;
pub mod scripting;
pub mod systems;
pub mod templates;
pub mod trie;
pub mod util;

pub use _hecs::Entity;

pub use components::{
    AccessLevel, ActiveEffect, ActiveScriptEffect, ActiveScriptEffects, ActiveStance, AffixMod,
    AffixModifiers, AffixNames, Age, Alignment, Appearance, Armor, Attributes, Class, CombatState,
    CombatStats, CommandRestrictions, Corpse, DamageType, DbId, Deity, Description, Direction,
    Dirty, Durability, EffectExpireCondition, EffectTemplate, EntityCommands, Equipment,
    EquipmentSlot, Exit, ExitFlags, Experience, FactionMember, FactionStanding, FloorItems,
    Following, Formation, Friendly, Gender, Group, GroupInvite, GroupManager, GroupMember,
    GroupMemberInfo, GroupRole, Health, HolyLight, Immortal, Inventory, Item, ItemSkillRequirement,
    LastMessenger, LearnedRecipes, LearnedSkills, Level, LootMode, LootRule, MultiClassInfo, Name,
    Npc, ObjectiveProgress, PatrolRoute, PermanentItemAffects, Player, PlayerState, PortalExit,
    Position, PracticePoints, PrayerCooldown, QuestLog, QuestProgress, Race, RecallRoom,
    Resistance, ResourceCost, RestState, Room, RoomAllowRevive, RoomExits, RoomFlagBits, RoomFlags,
    RoomKey, RoomPortals, RoomTags, ScriptParams, SetMembership, SetTracker, ShortDesc,
    SkillCooldowns, SkillDef, SkillType, Switched, Targeting, TemporaryEffect, Trainer, VoidRoom,
    Wallet, WanderBounds, Weapon, WeaponHands, WeaponRange, Wizin, EXIT_IS_CLOSED, EXIT_IS_DOOR,
    EXIT_IS_LOCKED, PORTAL_HIDDEN, ROOM_NO_TELEPORT_IN, ROOM_NO_TELEPORT_OUT, ROOM_PORTAL_IN,
    ROOM_PORTAL_OUT,
};
pub use prompt::PromptVars;
pub use resources::{Energy, Mana, Psi, Stamina, WorldName};
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
pub use systems::time::{advance_time, GameTime, Season, TimeConfig, TimeEvent};
pub use systems::trigger::{ItemTriggers, TriggeredEffect};
pub use systems::weather::{
    get_effective_weather_effects, resolve_weather_weights, roll_modifier, roll_weather,
    ResolutionParams, WeatherState,
};
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

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            inner: _hecs::World::new(),
        }
    }

    pub fn spawn(&mut self, components: impl _hecs::DynamicBundle) -> Entity {
        self.inner.spawn(components)
    }

    pub fn deserialized_spawn(
        &mut self,
        entity_id: u32,
        components: impl _hecs::DynamicBundle,
    ) -> Entity {
        let entity = _hecs::Entity::from_bits(entity_id as u64 | (1 << 32)).unwrap();
        self.inner.spawn_at(entity, components);
        entity
    }

    pub fn deserialized_spawn_at(&mut self, entity: Entity, components: impl _hecs::DynamicBundle) {
        self.inner.spawn_at(entity, components);
    }

    pub fn reserve_entity(&self) -> Entity {
        self.inner.reserve_entity()
    }

    pub fn spawn_at(&mut self, entity: Entity, components: impl _hecs::DynamicBundle) {
        self.inner.spawn_at(entity, components);
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<(), _hecs::NoSuchEntity> {
        self.inner.despawn(entity)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.inner.contains(entity)
    }

    pub fn insert(
        &mut self,
        entity: Entity,
        components: impl _hecs::DynamicBundle,
    ) -> Result<(), _hecs::NoSuchEntity> {
        self.inner.insert(entity, components)
    }

    pub fn remove_one<T: _hecs::Component>(
        &mut self,
        entity: Entity,
    ) -> Result<T, _hecs::ComponentError> {
        self.inner.remove_one::<T>(entity)
    }

    pub fn query<Q: _hecs::Query>(&self) -> _hecs::QueryBorrow<'_, Q> {
        self.inner.query::<Q>()
    }

    pub fn query_one<Q: _hecs::Query>(
        &self,
        entity: Entity,
    ) -> Result<_hecs::QueryOne<'_, Q>, _hecs::NoSuchEntity> {
        self.inner.query_one::<Q>(entity)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn len(&self) -> u32 {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
