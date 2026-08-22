//! Entity template CRUD handlers (mobs, items, quests, ...).
//!
//! `list_*`/`create_*`/`delete_*` are thinned onto the generic helpers in `crate::crud`
//! while preserving the exact output strings; `get_*` bodies are verbatim moves.

use std::collections::HashMap;

use oxide_core::templates::{
    AffixDef, ClassTemplate, FactionDef, HealthBounds, ItemTemplate, LootTable, MobTemplate,
    PassiveDef, QuestDef, QuestRewards, RaceAttributes, RaceTemplate, RecipeDef, SetDef, StanceDef,
};
use rmcp::handler::server::wrapper::Parameters;

use crate::context::HandlerContext;
use crate::params::*;

pub fn list_mobs(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Mobs", |r| {
        r.mobs
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_mob(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.get_mob(&p.id) {
        Some(mob) => {
            let route = if mob.patrol_route.is_empty() {
                String::new()
            } else {
                format!("\npatrol_route: {:?}", mob.patrol_route)
            };
            let w_rooms = if mob.wander_rooms.is_empty() {
                String::new()
            } else {
                format!("\nwander_rooms: {:?}", mob.wander_rooms)
            };
            let w_area = if mob.wander_area {
                "\nwander_area: true".to_string()
            } else {
                String::new()
            };
            format!(
                "id: {}\nname: {}\nlevel: {}\ndescription: {}\narmor: {}\nai: {}{}{}{}",
                p.id,
                mob.name,
                mob.level,
                mob.description,
                mob.armor,
                mob.ai_mode,
                route,
                w_rooms,
                w_area
            )
        }
        None => format!("Error: mob '{}' not found", p.id),
    }
}

pub async fn create_mob(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let mob = MobTemplate {
        id: p.id.clone(),
        name,
        description: String::new(),
        short_desc: String::new(),
        level: 1,
        attributes: RaceAttributes::default(),
        health: HealthBounds {
            current: 10,
            max: 10,
        },
        armor: 0,
        damage: None,
        damage_type: None,
        race: None,
        size: "medium".to_string(),
        equipment: Vec::new(),
        xp_value: 0,
        loot: LootTable::default(),
        ai_mode: "idle".to_string(),
        patrol_route: Vec::new(),
        wander_rooms: Vec::new(),
        wander_area: false,
        aggro_range: 0,
        aggro_players: false,
        aggro_mobs: false,
        aggro_race: Vec::new(),
        faction: None,
        faction_standing: 0,
        trainer_types: Vec::new(),
        languages: Vec::new(),
        shop: None,
        friendly: false,
        banker: false,
        skills: Vec::new(),
        scripts: Vec::new(),
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "mobs", "mob", mob).await
}

pub async fn delete_mob(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "mobs", "mob").await
}

pub fn list_items(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Items", |r| {
        r.items
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_item(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.get_item(&p.id) {
        Some(item) => format!(
            "id: {}\nname: {}\ntype: {}\nrarity: {}\nquality: {}\ndescription: {}",
            p.id, item.name, item.item_type, item.rarity, item.quality, item.description
        ),
        None => format!("Error: item '{}' not found", p.id),
    }
}

pub async fn create_item(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let item = ItemTemplate {
        id: p.id.clone(),
        name,
        description: String::new(),
        item_type: "misc".to_string(),
        subtype: String::new(),
        rarity: "common".to_string(),
        quality: "standard".to_string(),
        level_requirement: 1,
        weight: 1.0,
        value: 0,
        flags: Vec::new(),
        allowed_classes: Vec::new(),
        allowed_races: Vec::new(),
        allowed_alignments: Vec::new(),
        requires_skill: None,
        weapon: None,
        equipment: None,
        set: None,
        triggers: Vec::new(),
        params: HashMap::new(),
        ..Default::default()
    };
    crate::crud::create(ctx, &p.id, "items", "item", item).await
}

pub async fn delete_item(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "items", "item").await
}

pub fn list_quests(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Quests", |r| {
        r.quests
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_quest(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.quests.get(&p.id) {
        Some(q) => {
            let objectives: Vec<String> = q.objectives.iter().map(|o| format!("{:?}", o)).collect();
            let rewards_items: Vec<String> = q
                .rewards
                .items
                .iter()
                .map(|r| format!("{} x{}", r.item_template_id, r.count))
                .collect();
            let rewards_faction: Vec<String> = q
                .rewards
                .faction
                .iter()
                .map(|r| format!("{} {:+}", r.faction_id, r.amount))
                .collect();
            format!(
                    "id: {}\nname: {}\ndescription: {}\nlevel_requirement: {}\nrepeatable: {}\nauto_complete: {}\ngiver_npc: {}\nturn_in_npc: {}\nprerequisites: {:?}\nobjectives: [{}]\nrewards: xp={}, gold={}, items=[{}], faction=[{}]",
                    p.id,
                    q.name,
                    q.description,
                    q.level_requirement,
                    q.repeatable,
                    q.auto_complete,
                    q.giver_npc.as_deref().unwrap_or("none"),
                    q.turn_in_npc.as_deref().unwrap_or("none"),
                    q.prerequisites,
                    objectives.join(", "),
                    q.rewards.xp,
                    q.rewards.gold,
                    rewards_items.join(", "),
                    rewards_faction.join(", "),
                )
        }
        None => format!("Error: quest '{}' not found", p.id),
    }
}

pub async fn create_quest(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let quest = QuestDef {
        id: p.id.clone(),
        name,
        description: String::new(),
        level_requirement: 1,
        repeatable: false,
        auto_complete: false,
        giver_npc: None,
        turn_in_npc: None,
        prerequisites: Vec::new(),
        objectives: Vec::new(),
        rewards: QuestRewards {
            xp: 0,
            gold: 0,
            items: Vec::new(),
            faction: Vec::new(),
        },
        scripts: None,
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "quests", "quest", quest).await
}

pub async fn delete_quest(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "quests", "quest").await
}

pub fn list_factions(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Factions", |r| {
        r.factions
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_faction(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.factions.get(&p.id) {
        Some(f) => {
            let ranks: Vec<String> = f
                .ranks
                .iter()
                .map(|r| format!("{} (threshold: {})", r.name, r.threshold))
                .collect();
            format!(
                    "id: {}\nname: {}\ndescription: {}\nstarting_standing: {}\nmin_standing: {}\nmax_standing: {}\naggro_below: {}\nranks: [{}]\nrelationships: {:?}",
                    p.id, f.name, f.description, f.starting_standing,
                    f.min_standing, f.max_standing, f.aggro_below,
                    ranks.join(", "), f.relationships,
                )
        }
        None => format!("Error: faction '{}' not found", p.id),
    }
}

pub async fn create_faction(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let faction = FactionDef {
        id: p.id.clone(),
        name,
        description: String::new(),
        starting_standing: 0,
        min_standing: -10000,
        max_standing: 10000,
        ranks: Vec::new(),
        relationships: HashMap::new(),
        aggro_below: -500,
    };
    crate::crud::create(ctx, &p.id, "factions", "faction", faction).await
}

pub async fn delete_faction(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "factions", "faction").await
}

pub fn list_recipes(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Recipes", |r| {
        r.recipes
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_recipe(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.recipes.get(&p.id) {
        Some(r) => {
            let materials: Vec<String> = r
                .materials
                .iter()
                .map(|m| format!("{} x{}", m.template_id, m.quantity))
                .collect();
            let skill_req = r
                .skill_requirement
                .as_ref()
                .map(|s| format!("{} rank {}", s.id, s.rank))
                .unwrap_or_else(|| "none".to_string());
            format!(
                    "id: {}\nname: {}\ndescription: {}\nstation: {}\nskill_requirement: {}\ndifficulty: {}\nmaterials: [{}]\nresult: {} x{}\nsuccess_chance: {}\nquality_scaling: {}",
                    p.id, r.name, r.description,
                    r.station.as_deref().unwrap_or("none"),
                    skill_req, r.difficulty,
                    materials.join(", "),
                    r.result.template_id, r.result.quantity,
                    r.success_chance, r.quality_scaling,
                )
        }
        None => format!("Error: recipe '{}' not found", p.id),
    }
}

pub async fn create_recipe(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let recipe = RecipeDef {
        id: p.id.clone(),
        name,
        description: String::new(),
        station: None,
        skill_requirement: None,
        difficulty: 1,
        materials: Vec::new(),
        result: oxide_core::templates::RecipeResult {
            template_id: String::new(),
            quantity: 1,
        },
        success_chance: 95,
        quality_scaling: false,
        script: None,
    };
    crate::crud::create(ctx, &p.id, "recipes", "recipe", recipe).await
}

pub async fn delete_recipe(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "recipes", "recipe").await
}

pub fn list_shops(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Shops", |r| {
        r.shops
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_shop(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.shops.get(&p.id) {
        Some(s) => {
            let inv: Vec<String> = s
                .inventory
                .iter()
                .map(|e| {
                    format!(
                        "{} ({}-{}, price: {})",
                        e.item, e.count.min, e.count.max, e.price
                    )
                })
                .collect();
            let price_mods: Vec<String> = s
                .price_mods
                .iter()
                .map(|(k, v)| format!("{k}: {v}x"))
                .collect();
            let params: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}: {v}")).collect();
            format!(
                    "id: {}\nname: {}\nbuy_rate: {}\nsell_rate: {}\nrestock_secs: {}\nbuy_types: [{}]\nprice_mods: [{}]\nparams: [{}]\ninventory: [{}]",
                    p.id, s.name, s.buy_rate, s.sell_rate, s.restock_secs,
                    s.buy_types.join(", "),
                    price_mods.join(", "),
                    params.join(", "),
                    inv.join(", "),
                )
        }
        None => format!("Error: shop '{}' not found", p.id),
    }
}

pub async fn create_shop(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let shop = oxide_core::templates::ShopTemplate {
        id: p.id.clone(),
        name,
        buy_rate: 0.5,
        sell_rate: 1.0,
        restock_secs: 3600,
        inventory: Vec::new(),
        buy_types: Vec::new(),
        price_mods: HashMap::new(),
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "shops", "shop", shop).await
}

pub async fn delete_shop(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "shops", "shop").await
}

pub fn list_deities(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Deities", |r| {
        r.deities
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_deity(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.deities.get(&p.id) {
        Some(d) => {
            let prayer = d
                .prayer_effect
                .as_ref()
                .map(|pe| {
                    format!(
                        "buff={}, duration={}s, cooldown={}s, {}",
                        pe.buff_id, pe.duration_secs, pe.cooldown_secs, pe.description
                    )
                })
                .unwrap_or_else(|| "none".to_string());
            format!(
                    "id: {}\nname: {}\ndescription: {}\nalignment: {}\nsymbol: {}\nfavored_weapon: {}\ntenets: {:?}\ndomains: {:?}\nallowed_races: {:?}\nallowed_classes: {:?}\nallowed_alignments: {:?}\nprayer_effect: {}",
                    p.id, d.name, d.description,
                    d.alignment.as_deref().unwrap_or("any"),
                    d.symbol,
                    d.favored_weapon.as_deref().unwrap_or("none"),
                    d.tenets, d.domains, d.allowed_races, d.allowed_classes,
                    d.allowed_alignments, prayer,
                )
        }
        None => format!("Error: deity '{}' not found", p.id),
    }
}

pub async fn create_deity(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let deity = oxide_core::templates::DeityTemplate {
        id: p.id.clone(),
        name,
        description: String::new(),
        alignment: None,
        symbol: String::new(),
        favored_weapon: None,
        tenets: Vec::new(),
        domains: Vec::new(),
        allowed_races: Vec::new(),
        allowed_classes: Vec::new(),
        allowed_alignments: Vec::new(),
        prayer_effect: None,
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "deities", "deity", deity).await
}

pub async fn delete_deity(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "deities", "deity").await
}

pub fn list_stances(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Stances", |r| {
        r.stances
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_stance(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.stances.get(&p.id) {
            Some(s) => format!(
                "id: {}\nname: {}\nac_bonus: {}\nattack_penalty: {}\ndamage_bonus: {}\nac_penalty: {}\nmin_level: {}",
                p.id, s.name, s.ac_bonus, s.attack_penalty, s.damage_bonus, s.ac_penalty, s.min_level,
            ),
            None => format!("Error: stance '{}' not found", p.id),
        }
}

pub async fn create_stance(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let stance = StanceDef {
        id: p.id.clone(),
        name,
        ac_bonus: 0,
        attack_penalty: 0,
        damage_bonus: 0,
        ac_penalty: 0,
        min_level: 1,
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "stances", "stance", stance).await
}

pub async fn delete_stance(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "stances", "stance").await
}

pub fn list_sets(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Sets", |r| {
        r.sets
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_set(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.sets.get(&p.id) {
        Some(s) => {
            let bonuses: Vec<String> = s
                .bonuses
                .iter()
                .map(|b| {
                    let effects: Vec<String> = b
                        .effects
                        .iter()
                        .map(|e| {
                            format!(
                                "{} {} {:?}",
                                e.effect_type,
                                e.stat.as_deref().unwrap_or(""),
                                e.amount
                            )
                        })
                        .collect();
                    format!("min_pieces={}: [{}]", b.min_pieces, effects.join(", "))
                })
                .collect();
            format!(
                "id: {}\nname: {}\nbonuses: [{}]",
                p.id,
                s.name,
                bonuses.join("; "),
            )
        }
        None => format!("Error: set '{}' not found", p.id),
    }
}

pub async fn create_set(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let set = SetDef {
        id: p.id.clone(),
        name,
        bonuses: Vec::new(),
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "sets", "set", set).await
}

pub async fn delete_set(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "sets", "set").await
}

pub fn list_affixes(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Affixes", |r| {
        r.affixes
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_affix(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.affixes.get(&p.id) {
            Some(a) => format!(
                "id: {}\nname: {}\ndescription: {}\ntype: {}\nelement: {}\namount: {}\nstat: {}\nquality_min: {}\nslot: {:?}\nweight: {}",
                p.id, a.name, a.description, a.affix_type,
                a.element.as_deref().unwrap_or("none"),
                a.amount.as_deref().unwrap_or("none"),
                a.stat.as_deref().unwrap_or("none"),
                a.quality_min, a.slot, a.weight,
            ),
            None => format!("Error: affix '{}' not found", p.id),
        }
}

pub async fn create_affix(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let affix = AffixDef {
        id: p.id.clone(),
        name,
        description: String::new(),
        affix_type: "prefix".to_string(),
        element: None,
        amount: None,
        stat: None,
        quality_min: "common".to_string(),
        slot: Vec::new(),
        weight: 1,
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "affixes", "affix", affix).await
}

pub async fn delete_affix(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "affixes", "affix").await
}

pub fn list_passives(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Passives", |r| {
        r.passives
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_passive(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.passives.get(&p.id) {
        Some(pas) => {
            let effects: Vec<String> = pas
                .effects
                .iter()
                .map(|e| format!("{} {} {:?}", e.effect_type, e.target, e.amount))
                .collect();
            format!(
                "id: {}\nname: {}\ndescription: {}\neffects: [{}]",
                p.id,
                pas.name,
                pas.description,
                effects.join(", "),
            )
        }
        None => format!("Error: passive '{}' not found", p.id),
    }
}

pub async fn create_passive(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let passive = PassiveDef {
        id: p.id.clone(),
        name,
        description: String::new(),
        effects: Vec::new(),
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "passives", "passive", passive).await
}

pub async fn delete_passive(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "passives", "passive").await
}

pub fn list_skills(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Skills", |r| {
        r.skills
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_skill(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.skills.get(&p.id) {
            Some(s) => format!(
                "id: {}\nname: {}\ndescription: {}\nskill_type: {:?}\nmax_rank: {}\nlevel_requirement: {}\ncooldown_secs: {}\ntargeting: {:?}\ncost: {:?}\nallowed_classes: {:?}\nallowed_races: {:?}\nrequires_skill: {}\nmust_train: {}",
                p.id, s.name, s.description, s.skill_type, s.max_rank,
                s.level_requirement, s.cooldown_secs, s.targeting, s.cost,
                s.allowed_classes, s.allowed_races,
                s.requires_skill.as_deref().unwrap_or("none"), s.must_train,
            ),
            None => format!("Error: skill '{}' not found", p.id),
        }
}

pub async fn create_skill(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let skill = oxide_core::SkillDef::new(
        p.id.clone(),
        name,
        String::new(),
        oxide_core::SkillType::Combat,
    );
    crate::crud::create(ctx, &p.id, "skills", "skill", skill).await
}

pub async fn delete_skill(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "skills", "skill").await
}

pub fn list_races(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Races", |r| {
        r.races
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_race(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.races.get(&p.id) {
            Some(r) => format!(
                "id: {}\nname: {}\ndescription: {}\nattributes: STR={} DEX={} INT={} WIS={} CON={} CHA={}\nallowed_classes: {:?}\nallowed_alignments: {:?}\nracial_abilities: {:?}\nage_default: {}\nage_max: {}",
                p.id, r.name, r.description,
                r.attributes.strength, r.attributes.dexterity, r.attributes.intelligence,
                r.attributes.wisdom, r.attributes.constitution, r.attributes.charisma,
                r.allowed_classes, r.allowed_alignments, r.racial_abilities,
                r.age_default, r.age_max,
            ),
            None => format!("Error: race '{}' not found", p.id),
        }
}

pub async fn create_race(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let race = RaceTemplate {
        id: p.id.clone(),
        name,
        description: String::new(),
        attributes: RaceAttributes::default(),
        allowed_classes: Vec::new(),
        allowed_alignments: Vec::new(),
        racial_abilities: Vec::new(),
        allowed_genders: HashMap::new(),
        appearance_bounds: oxide_core::templates::AppearanceBounds::default(),
        age_default: 20,
        age_max: 100,
        params: HashMap::new(),
    };
    crate::crud::create(ctx, &p.id, "races", "race", race).await
}

pub async fn delete_race(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "races", "race").await
}

pub fn list_classes(ctx: &HandlerContext<'_>) -> String {
    crate::crud::list(ctx, "Classes", |r| {
        r.classes
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect()
    })
}

pub fn get_class(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    let (registry, _file_map) = ctx.load();
    match registry.classes.get(&p.id) {
            Some(c) => format!(
                "id: {}\nname: {}\ndescription: {}\nprestige: {}\nhit_die: {}\nbab: {}\nfort_save: {}\nref_save: {}\nwill_save: {}\nallowed_races: {:?}\nallowed_alignments: {:?}\nauto_skills: {:?}\nstarting_skill_slots: {}\ndeity_policy: {:?}",
                p.id, c.name, c.description, c.prestige, c.hit_die,
                c.bab, c.fort_save, c.ref_save, c.will_save,
                c.allowed_races, c.allowed_alignments, c.auto_skills,
                c.starting_skill_slots, c.deity_policy,
            ),
            None => format!("Error: class '{}' not found", p.id),
        }
}

pub async fn create_class(
    ctx: &HandlerContext<'_>,
    params: Parameters<CreateEntityParams>,
) -> String {
    let p = params.0;
    let name = p.name.unwrap_or_else(|| p.id.clone());
    let class = ClassTemplate {
        id: p.id.clone(),
        name,
        description: String::new(),
        prestige: false,
        prestige_gate: None,
        hit_die: 8,
        attribute_mods: oxide_core::templates::ClassAttributeMods::default(),
        bab: "poor".to_string(),
        fort_save: "poor".to_string(),
        ref_save: "poor".to_string(),
        will_save: "poor".to_string(),
        allowed_races: Vec::new(),
        allowed_alignments: Vec::new(),
        auto_skills: Vec::new(),
        params: HashMap::new(),
        skill_pool: Vec::new(),
        starting_skill_slots: 3,
        starting_items: Vec::new(),
        starting_gold: oxide_core::templates::WalletAmount::default(),
        deity_policy: oxide_core::templates::DeityPolicy::Any,
    };
    crate::crud::create(ctx, &p.id, "classes", "class", class).await
}

pub async fn delete_class(ctx: &HandlerContext<'_>, params: Parameters<IdParam>) -> String {
    let p = params.0;
    crate::crud::delete(ctx, &p.id, "classes", "class").await
}
