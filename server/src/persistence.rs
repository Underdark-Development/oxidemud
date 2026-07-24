use oxide_core::{
    Alignment, Attributes, DbId, Description, Entity, Equipment, Experience, Health, Inventory,
    LearnedSkills, Level, Player, Position, PracticePoints, RoomKey, Wallet, World,
};

pub fn save_online_players(world: &mut World, db: &oxide_data::Database, force: bool) {
    save_player_positions(world, db);

    let players: Vec<Entity> = if force {
        world
            .query::<(&Player, &DbId)>()
            .iter()
            .map(|(raw, _)| Entity::from(raw))
            .collect()
    } else {
        world
            .query::<(&Player, &DbId, &oxide_core::Dirty)>()
            .iter()
            .map(|(raw, _)| Entity::from(raw))
            .collect()
    };

    for player in players {
        save_player_progress(world, player, db);
        let _ = world.remove_one::<oxide_core::Dirty>(player);
    }
}

pub fn save_player_progress(world: &mut World, player: Entity, db: &oxide_data::Database) {
    let Some(db_id) = world
        .query_one::<&DbId>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|db_id| db_id.0)
    else {
        return;
    };

    let conn = db.conn();

    // 1. Level & Experience
    if let Some(level) = world
        .query_one::<&Level>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
    {
        let _ = oxide_data::save_level_component(conn, db_id, level.0 as i64);
        let _ = oxide_data::update_character_level(
            conn,
            db_id,
            level.0.into(),
            current_xp(world, player) as i64,
        );
    }

    if let Some(xp) = world
        .query_one::<&Experience>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
    {
        let _ = oxide_data::save_experience_component(conn, db_id, xp.0 as i64);
    }

    // 2. Health
    if let Some(health) = world
        .query_one::<&Health>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_health_component(conn, db_id, health.current, health.max);
    }

    // 3. Mana
    if let Some(mana) = world
        .query_one::<&oxide_core::Mana>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_mana_component(conn, db_id, mana.current as i32);
    }

    // 4. Stamina
    if let Some(stamina) = world
        .query_one::<&oxide_core::Stamina>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_stamina_component(conn, db_id, stamina.current as i32);
    }

    // 5. Wallet / Golds
    if let Some(wallet) = world
        .query_one::<&Wallet>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_golds_component(
            conn,
            db_id,
            wallet.copper as i64,
            wallet.silver as i64,
            wallet.gold as i64,
            wallet.platinum as i64,
        );
    }

    // 6. LearnedSkills
    if let Some(skills) = world
        .query_one::<&LearnedSkills>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Err(e) = oxide_data::save_skills(conn, db_id, &skills.skills) {
            tracing::error!(entity_id = db_id, error = %e, "Save player progress: failed to save skills");
        }
    }

    // 7. PracticePoints
    if let Some(pp) = world
        .query_one::<&PracticePoints>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
    {
        let _ = oxide_data::save_practice_points(conn, db_id, pp.0 as i64);
    }

    // 7.5. CombatStats
    if let Some(cs) = world
        .query_one::<&oxide_core::CombatStats>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_combat_stats_component(
            conn,
            db_id,
            cs.base_attack_bonus,
            cs.fort_save,
            cs.ref_save,
            cs.will_save,
        );
    }

    // 8. Player component
    if let Some(player_comp) = world
        .query_one::<&Player>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Err(e) = oxide_data::save_player_component(
            conn,
            db_id,
            player_comp.account_id,
            player_comp.prompt.as_deref(),
            player_comp.screen_width,
        ) {
            tracing::error!(entity_id = db_id, error = %e, "Save player progress: failed to save player component");
        }
    }

    // 9. Attributes
    if let Some(attrs) = world
        .query_one::<&Attributes>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_attributes_component(
            conn,
            db_id,
            &oxide_data::AttributesRow {
                strength: attrs.strength,
                dexterity: attrs.dexterity,
                intelligence: attrs.intelligence,
                wisdom: attrs.wisdom,
                constitution: attrs.constitution,
                charisma: attrs.charisma,
            },
        );
    }

    // 10. Alignment
    if let Some(alignment) = world
        .query_one::<&Alignment>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_alignment_component(conn, db_id, &alignment.0);
    }

    // 11. Description
    if let Some(description) = world
        .query_one::<&Description>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_description_component(conn, db_id, &description.0);
    }

    // 12. Inventory
    let mut inventory_items = Vec::new();
    if let Some(inventory) = world
        .query_one::<&Inventory>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        for &item_entity in &inventory.0 {
            if let Ok(mut item_q) =
                world.query_one::<(&oxide_core::Item, Option<&DbId>)>(item_entity)
            {
                if let Some((item, opt_db_id)) = item_q.get() {
                    inventory_items.push((
                        item_entity,
                        item.template_id.clone(),
                        opt_db_id.map(|d| d.0),
                    ));
                }
            }
        }
    }

    if !inventory_items.is_empty() {
        let _ = oxide_data::delete_all_inventory(conn, db_id);
        for (slot_idx, (item_entity, template_id, opt_db_id)) in
            inventory_items.into_iter().enumerate()
        {
            let item_db_id = match opt_db_id {
                Some(id) => id,
                None => {
                    if let Ok(new_id) = oxide_data::insert_entity(conn, "item") {
                        let _ = world.insert(item_entity, (DbId::new(new_id),));
                        new_id
                    } else {
                        continue;
                    }
                }
            };
            let _ = oxide_data::save_item_component(conn, item_db_id, &template_id);
            let _ = oxide_data::add_inventory_item(conn, db_id, item_db_id, slot_idx as i32);
        }
    }

    // 13. Equipment
    let mut equipment_items = Vec::new();
    if let Some(equipment) = world
        .query_one::<&Equipment>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        for &(slot, item_entity) in &equipment.slots {
            if let Ok(mut item_q) =
                world.query_one::<(&oxide_core::Item, Option<&DbId>)>(item_entity)
            {
                if let Some((item, opt_db_id)) = item_q.get() {
                    equipment_items.push((
                        slot,
                        item_entity,
                        item.template_id.clone(),
                        opt_db_id.map(|d| d.0),
                    ));
                }
            }
        }
    }

    if !equipment_items.is_empty() {
        let _ = oxide_data::delete_all_equipment(conn, db_id);
        for (slot, item_entity, template_id, opt_db_id) in equipment_items {
            let item_db_id = match opt_db_id {
                Some(id) => id,
                None => {
                    if let Ok(new_id) = oxide_data::insert_entity(conn, "item") {
                        let _ = world.insert(item_entity, (DbId::new(new_id),));
                        new_id
                    } else {
                        continue;
                    }
                }
            };
            let _ = oxide_data::save_item_component(conn, item_db_id, &template_id);
            let slot_str = format!("{:?}", slot).to_lowercase();
            let _ = oxide_data::save_equipment_slot(conn, db_id, &slot_str, item_db_id);
        }
    }

    // 14. Appearance
    if let Some(appearance) = world
        .query_one::<&oxide_core::Appearance>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        let _ = oxide_data::save_appearance_component(conn, db_id, &appearance);
    }

    // 15. Age
    if let Some(age) = world
        .query_one::<&oxide_core::Age>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
    {
        let _ = oxide_data::save_age_component(conn, db_id, age.0 as i32);
    }

    // 16. Deity
    if let Some(deity) = world
        .query_one::<&oxide_core::Deity>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Some(ref deity_id) = deity.0 {
            let _ = oxide_data::save_deity_component(conn, db_id, deity_id);
        }
    }

    // 17. QuestLog
    if let Some(quest_log) = world
        .query_one::<&oxide_core::QuestLog>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Ok(json) = serde_json::to_string(&quest_log) {
            let _ = oxide_data::save_quest_log_component(conn, db_id, &json);
        }
    }

    // 18. FactionStanding
    if let Some(faction_standing) = world
        .query_one::<&oxide_core::FactionStanding>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Ok(json) = serde_json::to_string(&faction_standing) {
            let _ = oxide_data::save_faction_standing_component(conn, db_id, &json);
        }
    }

    // 19. LearnedRecipes
    if let Some(learned_recipes) = world
        .query_one::<&oxide_core::LearnedRecipes>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Ok(json) = serde_json::to_string(&learned_recipes) {
            let _ = oxide_data::save_learned_recipes_component(conn, db_id, &json);
        }
    }

    // 20. MultiClassInfo
    if let Some(multiclass_info) = world
        .query_one::<&oxide_core::MultiClassInfo>(player)
        .ok()
        .and_then(|mut q| q.get().cloned())
    {
        if let Ok(json) = serde_json::to_string(&multiclass_info) {
            let _ = oxide_data::save_multiclass_component(conn, db_id, &json);
        }
    }
}

fn current_xp(world: &World, player: Entity) -> u64 {
    world
        .query_one::<&Experience>(player)
        .ok()
        .and_then(|mut q| q.get().copied())
        .map(|xp| xp.0)
        .unwrap_or(0)
}

pub fn save_player_positions(world: &mut World, db: &oxide_data::Database) {
    let conn = db.conn();
    let players: Vec<(i64, Entity)> = world
        .query::<(&DbId, &Position, &Player)>()
        .iter()
        .map(|(_, (db_id, pos, _))| (db_id.0, pos.room))
        .collect();

    for (player_entity_id, room_entity) in players {
        if let Some(room_key) = world
            .query_one::<&RoomKey>(room_entity)
            .ok()
            .and_then(|mut q| q.get().map(|k| k.0.clone()))
        {
            let _ =
                oxide_data::update_character_current_room_key(conn, player_entity_id, &room_key);
            let _ = oxide_data::update_character_last_seen(conn, player_entity_id);
        }
    }
}

pub fn save_world_time(world: &World, db: &oxide_data::Database) {
    let mut q = world.query::<&oxide_core::GameTime>();
    if let Some((_, gt)) = q.into_iter().next() {
        let _ = db.save_world_time(gt.hour, gt.minute, gt.day, gt.season.name(), gt.year);
    }
}

pub fn load_or_init_world_time(
    world: &mut World,
    db: Option<&oxide_data::Database>,
    config: &oxide_core::TimeConfig,
) -> Entity {
    // Check if GameTime is already spawned in world
    let existing: Vec<Entity> = world
        .query::<&oxide_core::GameTime>()
        .iter()
        .map(|(e, _)| Entity::from(e))
        .collect();

    if let Some(&ent) = existing.first() {
        return ent;
    }

    if let Some(db) = db {
        if let Ok(Some((hour, minute, day, season_str, year))) = db.load_world_time() {
            let season = season_str
                .parse::<oxide_core::Season>()
                .unwrap_or(oxide_core::Season::Spring);
            let mut gt = oxide_core::GameTime::new(hour, day, season, year);
            gt.minute = minute;
            return world.spawn((gt,));
        }
    }

    let season = config
        .start_season
        .parse::<oxide_core::Season>()
        .unwrap_or(oxide_core::Season::Spring);
    let gt = oxide_core::GameTime::new(config.start_hour, 1, season, 1);
    world.spawn((gt,))
}
