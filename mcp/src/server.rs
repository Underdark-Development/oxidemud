use std::path::PathBuf;

use crate::params::*;
use crate::simulator::{
    SimulateCharacterCreationParams, SimulateCombatParams, SimulateSkillUseParams,
};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router, transport::stdio, ServiceExt};

#[derive(Clone)]
pub struct OxideMcpServer {
    content_path: PathBuf,
    #[allow(dead_code)]
    api_url: Option<String>,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl OxideMcpServer {
    pub fn new(content_path: PathBuf, api_url: Option<String>, api_key: Option<String>) -> Self {
        OxideMcpServer {
            content_path,
            api_url,
            api_key,
        }
    }

    fn handler_context(&self) -> crate::context::HandlerContext<'_> {
        crate::context::HandlerContext::new(&self.content_path, &self.api_url, &self.api_key)
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let service = self.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
}

#[tool_router(server_handler)]
impl OxideMcpServer {
    #[tool(description = "List all areas")]
    fn list_areas(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::list_areas(&ctx)
    }
    #[tool(description = "Get area details")]
    fn get_area(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::get_area(&ctx, params)
    }
    #[tool(description = "Create a new area")]
    fn create_area(&self, params: Parameters<CreateAreaParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::create_area(&ctx, params)
    }
    #[tool(description = "Delete an area and its file")]
    fn delete_area(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::delete_area(&ctx, params)
    }
    #[tool(description = "List rooms in an area")]
    fn list_rooms(&self, params: Parameters<AreaIdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::list_rooms(&ctx, params)
    }
    #[tool(description = "Get room details")]
    fn get_room(&self, params: Parameters<RoomIdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::get_room(&ctx, params)
    }
    #[tool(description = "Create a new room in an area")]
    fn create_room(&self, params: Parameters<CreateRoomParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::create_room(&ctx, params)
    }
    #[tool(description = "Delete a room from an area")]
    fn delete_room(&self, params: Parameters<RoomIdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::delete_room(&ctx, params)
    }
    #[tool(description = "Link two rooms together by adding an exit")]
    fn link_rooms(&self, params: Parameters<LinkRoomsParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::link_rooms(&ctx, params)
    }
    #[tool(description = "Add a portal (keyword-based exit) to a room")]
    fn add_portal(&self, params: Parameters<UpdateRoomFieldsParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::add_portal(&ctx, params)
    }
    #[tool(description = "Remove a portal from a room (set keyword to empty)")]
    fn remove_portal(&self, params: Parameters<UpdateRoomFieldsParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::remove_portal(&ctx, params)
    }
    #[tool(description = "Update a room's fields inline")]
    fn update_room(&self, params: Parameters<UpdateRoomFieldsParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::update_room(&ctx, params)
    }
    #[tool(
        description = "Update fields on any template type except rooms (use update_room for rooms)"
    )]
    fn update_template(&self, params: Parameters<UpdateFieldsParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::areas::update_template(&ctx, params)
    }
    #[tool(description = "List all mob templates")]
    fn list_mobs(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_mobs(&ctx)
    }
    #[tool(description = "Get mob template details")]
    fn get_mob(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_mob(&ctx, params)
    }
    #[tool(description = "Create a new mob template")]
    async fn create_mob(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_mob(&ctx, params).await
    }
    #[tool(description = "Delete a mob template")]
    async fn delete_mob(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_mob(&ctx, params).await
    }
    #[tool(description = "List all item templates")]
    fn list_items(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_items(&ctx)
    }
    #[tool(description = "Get item template details")]
    fn get_item(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_item(&ctx, params)
    }
    #[tool(description = "Create a new item template")]
    async fn create_item(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_item(&ctx, params).await
    }
    #[tool(description = "Delete an item template")]
    async fn delete_item(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_item(&ctx, params).await
    }
    #[tool(description = "List all quest templates")]
    fn list_quests(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_quests(&ctx)
    }
    #[tool(description = "Get quest template details")]
    fn get_quest(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_quest(&ctx, params)
    }
    #[tool(description = "Create a new quest template")]
    async fn create_quest(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_quest(&ctx, params).await
    }
    #[tool(description = "Delete a quest template")]
    async fn delete_quest(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_quest(&ctx, params).await
    }
    #[tool(description = "List all faction templates")]
    fn list_factions(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_factions(&ctx)
    }
    #[tool(description = "Get faction template details")]
    fn get_faction(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_faction(&ctx, params)
    }
    #[tool(description = "Create a new faction template")]
    async fn create_faction(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_faction(&ctx, params).await
    }
    #[tool(description = "Delete a faction template")]
    async fn delete_faction(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_faction(&ctx, params).await
    }
    #[tool(description = "List all recipe templates")]
    fn list_recipes(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_recipes(&ctx)
    }
    #[tool(description = "Get recipe template details")]
    fn get_recipe(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_recipe(&ctx, params)
    }
    #[tool(description = "Create a new recipe template")]
    async fn create_recipe(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_recipe(&ctx, params).await
    }
    #[tool(description = "Delete a recipe template")]
    async fn delete_recipe(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_recipe(&ctx, params).await
    }
    #[tool(description = "List all shop templates")]
    fn list_shops(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_shops(&ctx)
    }
    #[tool(description = "Get shop template details")]
    fn get_shop(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_shop(&ctx, params)
    }
    #[tool(description = "Create a new shop template")]
    async fn create_shop(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_shop(&ctx, params).await
    }
    #[tool(description = "Delete a shop template")]
    async fn delete_shop(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_shop(&ctx, params).await
    }
    #[tool(description = "List all deity templates")]
    fn list_deities(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_deities(&ctx)
    }
    #[tool(description = "Get deity template details")]
    fn get_deity(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_deity(&ctx, params)
    }
    #[tool(description = "Create a new deity template")]
    async fn create_deity(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_deity(&ctx, params).await
    }
    #[tool(description = "Delete a deity template")]
    async fn delete_deity(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_deity(&ctx, params).await
    }
    #[tool(description = "List all stance templates")]
    fn list_stances(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_stances(&ctx)
    }
    #[tool(description = "Get stance template details")]
    fn get_stance(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_stance(&ctx, params)
    }
    #[tool(description = "Create a new stance template")]
    async fn create_stance(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_stance(&ctx, params).await
    }
    #[tool(description = "Delete a stance template")]
    async fn delete_stance(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_stance(&ctx, params).await
    }
    #[tool(description = "List all item set templates")]
    fn list_sets(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_sets(&ctx)
    }
    #[tool(description = "Get item set template details")]
    fn get_set(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_set(&ctx, params)
    }
    #[tool(description = "Create a new item set template")]
    async fn create_set(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_set(&ctx, params).await
    }
    #[tool(description = "Delete an item set template")]
    async fn delete_set(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_set(&ctx, params).await
    }
    #[tool(description = "List all affix templates")]
    fn list_affixes(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_affixes(&ctx)
    }
    #[tool(description = "Get affix template details")]
    fn get_affix(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_affix(&ctx, params)
    }
    #[tool(description = "Create a new affix template")]
    async fn create_affix(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_affix(&ctx, params).await
    }
    #[tool(description = "Delete an affix template")]
    async fn delete_affix(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_affix(&ctx, params).await
    }
    #[tool(description = "List all passive templates")]
    fn list_passives(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_passives(&ctx)
    }
    #[tool(description = "Get passive template details")]
    fn get_passive(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_passive(&ctx, params)
    }
    #[tool(description = "Create a new passive template")]
    async fn create_passive(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_passive(&ctx, params).await
    }
    #[tool(description = "Delete a passive template")]
    async fn delete_passive(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_passive(&ctx, params).await
    }
    #[tool(description = "List all skill templates")]
    fn list_skills(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_skills(&ctx)
    }
    #[tool(description = "Get skill template details")]
    fn get_skill(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_skill(&ctx, params)
    }
    #[tool(description = "Create a new skill template")]
    async fn create_skill(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_skill(&ctx, params).await
    }
    #[tool(description = "Delete a skill template")]
    async fn delete_skill(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_skill(&ctx, params).await
    }
    #[tool(description = "List all race templates")]
    fn list_races(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_races(&ctx)
    }
    #[tool(description = "Get race template details")]
    fn get_race(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_race(&ctx, params)
    }
    #[tool(description = "Create a new race template")]
    async fn create_race(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_race(&ctx, params).await
    }
    #[tool(description = "Delete a race template")]
    async fn delete_race(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_race(&ctx, params).await
    }
    #[tool(description = "List all class templates")]
    fn list_classes(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::list_classes(&ctx)
    }
    #[tool(description = "Get class template details")]
    fn get_class(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::get_class(&ctx, params)
    }
    #[tool(description = "Create a new class template")]
    async fn create_class(&self, params: Parameters<CreateEntityParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::create_class(&ctx, params).await
    }
    #[tool(description = "Delete a class template")]
    async fn delete_class(&self, params: Parameters<IdParam>) -> String {
        let ctx = self.handler_context();
        crate::handlers::entities::delete_class(&ctx, params).await
    }
    #[tool(description = "Get template content as TOML for any type")]
    fn get_template_raw(&self, params: Parameters<UpdateFieldsParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::tools::get_template_raw(&ctx, params)
    }
    #[tool(description = "Validate all templates for cross-reference errors")]
    fn validate(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::tools::validate(&ctx)
    }
    #[tool(description = "Get content statistics summary")]
    fn get_stats(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::tools::get_stats(&ctx)
    }
    #[tool(description = "Fuzzy search all template names and descriptions")]
    fn search(&self, params: Parameters<SearchParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::tools::search(&ctx, params)
    }
    #[tool(description = "Simulate loot drops from a mob template")]
    fn simulate_loot(&self, params: Parameters<SimulateLootParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_loot(&ctx, params)
    }
    #[tool(description = "Simulate combat rounds between two characters (based on templates)")]
    fn simulate_combat(&self, params: Parameters<SimulateCombatParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_combat(&ctx, params)
    }
    #[tool(description = "Simulate character progression stats level-by-level")]
    fn simulate_progression(&self, params: Parameters<SimulateProgressionParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_progression(&ctx, params)
    }
    #[tool(description = "Simulate a gear loadout on a mock character and show final stats")]
    fn simulate_gear_loadout(&self, params: Parameters<SimulateGearLoadoutParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_gear_loadout(&ctx, params)
    }
    #[tool(description = "Simulate AI random wander paths and room visit frequencies")]
    fn simulate_ai_wander(&self, params: Parameters<SimulateAiWanderParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_ai_wander(&ctx, params)
    }
    #[tool(
        description = "Simulate shop buying/selling transaction pricing across reputation levels"
    )]
    fn simulate_shop_transaction(
        &self,
        params: Parameters<SimulateShopTransactionParams>,
    ) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_shop_transaction(&ctx, params)
    }
    #[tool(description = "Validate skill prerequisites for circular dependency loops")]
    fn validate_content_dag(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::validate_content_dag(&ctx)
    }
    #[tool(description = "Simulate recipe crafting outcomes based on character stats")]
    async fn simulate_crafting(&self, params: Parameters<SimulateCraftingParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_crafting(&ctx, params).await
    }
    #[tool(description = "Simulate casting spells or using active abilities")]
    async fn simulate_skill_use(&self, params: Parameters<SimulateSkillUseParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_skill_use(&ctx, params).await
    }
    #[tool(description = "Simulate adoption constraints and prayer buff effects for a deity")]
    async fn simulate_prayer(&self, params: Parameters<SimulatePrayerParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_prayer(&ctx, params).await
    }
    #[tool(description = "Check if a character satisfies requirements for a prestige class")]
    async fn simulate_prestige_eligibility(
        &self,
        params: Parameters<SimulatePrestigeParams>,
    ) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_prestige_eligibility(&ctx, params).await
    }
    #[tool(
        description = "Evaluate stat and AC modifiers applied to a group based on party layout and formation"
    )]
    fn simulate_group_formation(&self, params: Parameters<SimulateGroupParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_group_formation(&ctx, params)
    }
    #[tool(
        description = "Calculate XP loss penalties, corpse decay, and ghost parameters when a player dies"
    )]
    async fn simulate_death_penalty(&self, params: Parameters<SimulateDeathParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_death_penalty(&ctx, params).await
    }
    #[tool(
        description = "Simulate character creation (online if MUD server is connected, otherwise offline fallback)"
    )]
    async fn simulate_character_creation(
        &self,
        params: Parameters<SimulateCharacterCreationParams>,
    ) -> String {
        let ctx = self.handler_context();
        crate::handlers::simulation::simulate_character_creation(&ctx, params).await
    }
    #[tool(description = "List all currently connected players in the MUD (Online Only)")]
    async fn list_connected_players(&self) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::list_connected_players(&ctx).await
    }
    #[tool(description = "Put an item from a template into a player's inventory (Online Only)")]
    async fn imm_put_item(&self, params: Parameters<PutItemParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_put_item(&ctx, params).await
    }
    #[tool(description = "Teleport a player to a specific room by its key (Online Only)")]
    async fn imm_teleport(&self, params: Parameters<TeleportParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_teleport(&ctx, params).await
    }
    #[tool(description = "Force a player to execute a command as if they typed it (Online Only)")]
    async fn imm_force_command(&self, params: Parameters<ForceCommandParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_force_command(&ctx, params).await
    }
    #[tool(description = "Set character attributes, pools, level, or XP (Online Only)")]
    async fn imm_set_stat(&self, params: Parameters<SetStatParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_set_stat(&ctx, params).await
    }
    #[tool(description = "Spawn an NPC from a template into a specific room (Online Only)")]
    async fn imm_load_mob(&self, params: Parameters<LoadMobParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_load_mob(&ctx, params).await
    }
    #[tool(description = "Spawn an item from a template into a specific room (Online Only)")]
    async fn imm_load_item(&self, params: Parameters<LoadItemParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_load_item(&ctx, params).await
    }
    #[tool(description = "Broadcast a global echo message to all players (Online Only)")]
    async fn imm_gecho(&self, params: Parameters<GechoParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_gecho(&ctx, params).await
    }
    #[tool(description = "Advance a player to a specific level (Online Only)")]
    async fn imm_advance(&self, params: Parameters<AdvanceParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_advance(&ctx, params).await
    }
    #[tool(
        description = "Inspect ECS stats and components of a target character or NPC (Online Only)"
    )]
    async fn imm_stat(&self, params: Parameters<StatParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_stat(&ctx, params).await
    }
    #[tool(description = "Fully heal a target's HP, mana, and stamina (Online Only)")]
    async fn imm_heal(&self, params: Parameters<HealParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_heal(&ctx, params).await
    }
    #[tool(description = "Deal direct damage to a target entity (Online Only)")]
    async fn imm_damage(&self, params: Parameters<DamageParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_damage(&ctx, params).await
    }
    #[tool(description = "Instantly kill a target entity (Online Only)")]
    async fn imm_kill(&self, params: Parameters<KillParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_kill(&ctx, params).await
    }
    #[tool(description = "Revive a dead or ghost target entity (Online Only)")]
    async fn imm_revive(&self, params: Parameters<ReviveParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_revive(&ctx, params).await
    }
    #[tool(description = "Set character alignment (Online Only)")]
    async fn imm_set_alignment(&self, params: Parameters<SetAlignmentParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_set_alignment(&ctx, params).await
    }
    #[tool(description = "Adjust character faction standing (Online Only)")]
    async fn imm_set_faction(&self, params: Parameters<SetFactionParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_set_faction(&ctx, params).await
    }
    #[tool(description = "Purge all NPCs and items from a room (Online Only)")]
    async fn imm_purge_room(&self, params: Parameters<PurgeRoomParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_purge_room(&ctx, params).await
    }
    #[tool(description = "Initiate a graceful server reboot (Online Only)")]
    async fn imm_reboot(&self, params: Parameters<RebootParams>) -> String {
        let ctx = self.handler_context();
        crate::handlers::immortal::imm_reboot(&ctx, params).await
    }
}
