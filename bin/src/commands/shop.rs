use std::time::Instant;

use oxide_core as core;
use oxide_core::templates::{ItemTemplate, ShopInventoryEntry, ShopTemplate, TemplateRegistry};
use oxide_core::{get_pos_room, AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

/// Format a copper amount into gp/sp/cp denominations.
fn format_copper(copper: u64) -> String {
    let gp = copper / 10_000;
    let rem = copper % 10_000;
    let sp = rem / 100;
    let cp = rem % 100;
    let mut parts = Vec::new();
    if gp > 0 {
        parts.push(format!("{gp}gp"));
    }
    if sp > 0 || gp > 0 {
        parts.push(format!("{sp}sp"));
    }
    if cp > 0 || parts.is_empty() {
        parts.push(format!("{cp}cp"));
    }
    parts.join(" ")
}

fn keeper_name(world: &World, keeper: core::Entity) -> String {
    core::get_name(world, keeper)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "The shopkeeper".to_string())
}

/// Locate a shopkeeper in the player's room and resolve its shop template.
fn find_shop<'a>(
    world: &World,
    entity: core::Entity,
    templates: &'a TemplateRegistry,
) -> Option<(core::Entity, String, &'a ShopTemplate)> {
    let room = get_pos_room(world, entity)?;
    let keeper = core::shopkeeper_in_room(world, room)?;
    let shop_id = world
        .query_one::<&core::Shopkeeper>(keeper)
        .ok()
        .and_then(|mut q| q.get().map(|s| s.shop_id.clone()))?;
    let shop = templates.shops.get(&shop_id)?;
    Some((keeper, shop_id, shop))
}

enum ItemLookup<'a> {
    Found(&'a ShopInventoryEntry, &'a ItemTemplate),
    Ambiguous(Vec<String>),
    NotFound,
}

/// Match a player query against the names of the shop's stocked items.
fn resolve_stock_item<'a>(
    shop: &'a ShopTemplate,
    templates: &'a TemplateRegistry,
    query: &str,
) -> ItemLookup<'a> {
    let candidates: Vec<(String, usize)> = shop
        .inventory
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            templates
                .items
                .get(&entry.item)
                .map(|item| (item.name.clone(), i))
        })
        .collect();
    match core::trie::trie_match(query, candidates) {
        core::trie::TrieMatch::One(i) => {
            let entry = &shop.inventory[i];
            match templates.items.get(&entry.item) {
                Some(item) => ItemLookup::Found(entry, item),
                None => ItemLookup::NotFound,
            }
        }
        core::trie::TrieMatch::Many(matches) => {
            let names: Vec<String> = matches
                .into_iter()
                .filter_map(|i| {
                    templates
                        .items
                        .get(&shop.inventory[i].item)
                        .map(|t| t.name.clone())
                })
                .collect();
            ItemLookup::Ambiguous(names)
        }
        core::trie::TrieMatch::None => ItemLookup::NotFound,
    }
}

/// Split `"item name 25"` into (`"item name"`, `Some(25)`).
fn split_item_offer(input: &str) -> (&str, Option<u64>) {
    let trimmed = input.trim();
    if let Some((idx, _)) = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
    {
        let (head, tail) = trimmed.split_at(idx);
        if let Ok(n) = tail.trim().parse::<u64>() {
            return (head.trim(), Some(n));
        }
    }
    (trimmed, None)
}

/// Charge the player, deplete stock, and place the purchased item in their
/// inventory. Sends failure messages on the way; only sends the success line
/// after a completed sale.
fn complete_purchase(
    world: &mut World,
    conn: &mut dyn Connection,
    entity: core::Entity,
    keeper: core::Entity,
    item_id: &str,
    price: u64,
    templates: &TemplateRegistry,
) {
    let item_tmpl = match templates.items.get(item_id) {
        Some(t) => t,
        None => return,
    };
    let item_name = item_tmpl.name.clone();

    let mut wallet = match world.query_one::<&mut core::Wallet>(entity) {
        Ok(mut q) => match q.get() {
            Some(w) => w.clone(),
            None => {
                conn.send_line("You have no money.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no money.");
            return;
        }
    };

    let total = wallet.total_copper();
    if !wallet.deduct_copper(price) {
        conn.send_line(&format!(
            "You can't afford that. It costs {} and you have {}.",
            format_copper(price),
            format_copper(total)
        ));
        return;
    }

    if let Ok(mut q) = world.query_one::<&mut core::ShopStock>(keeper) {
        if let Some(stock) = q.get() {
            if let Some(count) = stock.0.get_mut(item_id) {
                *count = count.saturating_sub(1);
            }
        }
    }

    let item_ent = item_tmpl.spawn(world);
    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.push(item_ent);
        }
    }

    let _ = world.insert(entity, (wallet, core::Dirty));

    conn.send_line(&format!(
        "You buy {item_name} for {}.",
        format_copper(price)
    ));
}

/// End a haggle session: clear the pending negotiation and start the
/// session-only cooldown for this shop.
fn end_negotiation(world: &mut World, entity: core::Entity, shop_id: &str, cooldown_secs: u64) {
    let _ = world.remove_one::<core::PendingHaggle>(entity);
    let _ = world.insert(
        entity,
        (core::HaggleCooldown {
            shop_id: shop_id.to_string(),
            ready_at: Instant::now() + std::time::Duration::from_secs(cooldown_secs),
        },),
    );
}

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "list",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Economy",
        help: CommandHelp {
            short: "Browse a shopkeeper's wares",
            body: Some("Usage: list [item]\nShows the wares and prices of a nearby shopkeeper, optionally limited to a single item."),
        },
        handler: cmd_list,
    });
    server.register_command(Command {
        name: "buy",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Economy",
        help: CommandHelp {
            short: "Buy an item from a shopkeeper",
            body: Some("Usage: buy <item> [offer]\nBuys an item at the shopkeeper's asking price, or counters with a specific offer to haggle."),
        },
        handler: cmd_buy,
    });
    server.register_command(Command {
        name: "sell",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Economy",
        help: CommandHelp {
            short: "Sell an item to a shopkeeper",
            body: Some("Usage: sell <item>\nSells a carried item to a nearby shopkeeper for its buyback price."),
        },
        handler: cmd_sell,
    });
    server.register_command(Command {
        name: "value",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "Economy",
        help: CommandHelp {
            short: "Check what a shopkeeper would pay for an item",
            body: Some("Usage: value <item>\nShows how much a nearby shopkeeper would pay for a carried item."),
        },
        handler: cmd_value,
    });
}

fn cmd_list(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Server error: templates unavailable.");
            return;
        }
    };

    let (keeper, _shop_id, shop) = match find_shop(world, entity, &templates) {
        Some(found) => found,
        None => {
            conn.send_line("There is no shopkeeper here.");
            return;
        }
    };

    let rep_mult = core::reputation_multiplier(world, entity, keeper, shop, &templates);

    if !args.trim().is_empty() {
        let single = match resolve_stock_item(shop, &templates, args.trim()) {
            ItemLookup::Found(entry, item) => (entry, item),
            ItemLookup::Ambiguous(names) => {
                conn.send_line(&format!("Which one? {}", names.join(", ")));
                return;
            }
            ItemLookup::NotFound => {
                conn.send_line(&format!(
                    "{} doesn't sell that.",
                    keeper_name(world, keeper)
                ));
                return;
            }
        };
        let (entry, item) = single;
        let base = core::base_price(entry, item);
        let asking = core::asking_price(shop, base, rep_mult);
        let stock = world
            .query_one::<&core::ShopStock>(keeper)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .map(|s| s.count(&entry.item))
            .unwrap_or(0);
        let stock_desc = if stock == 0 {
            "out of stock".to_string()
        } else {
            format!("{stock} left")
        };
        conn.send_line(&format!(
            "{} {:<8} {}",
            item.name,
            format_copper(asking),
            stock_desc
        ));
        return;
    }

    conn.send_line(&format!("{} is selling:", shop.name));
    for entry in &shop.inventory {
        let Some(item) = templates.items.get(&entry.item) else {
            continue;
        };
        let base = core::base_price(entry, item);
        let asking = core::asking_price(shop, base, rep_mult);
        let stock = world
            .query_one::<&core::ShopStock>(keeper)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .map(|s| s.count(&entry.item))
            .unwrap_or(0);
        let stock_desc = if stock == 0 {
            "out of stock".to_string()
        } else {
            format!("{stock} left")
        };
        conn.send_line(&format!(
            "  {:<24} {:<10} {}",
            item.name,
            format_copper(asking),
            stock_desc
        ));
    }
}

fn cmd_buy(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let input = args.trim();
    if input.is_empty() {
        conn.send_line("Buy what?");
        return;
    }

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Server error: templates unavailable.");
            return;
        }
    };

    let (keeper, shop_id, shop) = match find_shop(world, entity, &templates) {
        Some(found) => found,
        None => {
            conn.send_line("There is no shopkeeper here.");
            return;
        }
    };
    let keeper_display = keeper_name(world, keeper);

    let (item_query, offer) = split_item_offer(input);

    let (entry, item) = match resolve_stock_item(shop, &templates, item_query) {
        ItemLookup::Found(e, i) => (e, i),
        ItemLookup::Ambiguous(names) => {
            conn.send_line(&format!("Which one? {}", names.join(", ")));
            return;
        }
        ItemLookup::NotFound => {
            conn.send_line(&format!("{keeper_display} doesn't sell that."));
            return;
        }
    };

    let stock = world
        .query_one::<&core::ShopStock>(keeper)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .map(|s| s.count(&entry.item))
        .unwrap_or(0);
    if stock == 0 {
        conn.send_line(&format!(
            "{keeper_display} is out of {} at the moment.",
            item.name
        ));
        return;
    }

    let base = core::base_price(entry, item);
    let rep_mult = core::reputation_multiplier(world, entity, keeper, shop, &templates);
    let asking = core::asking_price(shop, base, rep_mult);
    let params = core::ShopParams::from_shop(shop);

    if let Some(offer) = offer {
        if core::haggle_on_cooldown(world, entity, &shop_id, Instant::now()) {
            let remaining = world
                .query_one::<&core::HaggleCooldown>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .map(|c| c.remaining_secs(Instant::now()))
                .unwrap_or(0);
            conn.send_line(&format!(
                "{keeper_display} won't negotiate again for another {remaining} seconds."
            ));
            return;
        }

        let pending = if core::pending_haggle_valid(world, entity, keeper, &shop_id, &entry.item) {
            world
                .query_one::<&core::PendingHaggle>(entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
        } else {
            None
        };

        let (floor, rounds_used) = match &pending {
            Some(p) => (p.floor, p.rounds_used),
            None => (
                core::haggle_floor(asking, &params, core::charisma_of(world, entity), rep_mult),
                0,
            ),
        };
        let insult = core::insult_threshold(asking, &params);

        match core::evaluate_counter(
            offer,
            asking,
            floor,
            insult,
            rounds_used,
            params.haggle_rounds,
        ) {
            core::CounterOutcome::Accept => {
                let price = offer.min(asking);
                complete_purchase(world, conn, entity, keeper, &entry.item, price, &templates);
                end_negotiation(world, entity, &shop_id, params.haggle_cooldown_secs);
            }
            core::CounterOutcome::Counter(counter) => {
                let _ = world.insert(
                    entity,
                    (core::PendingHaggle {
                        shop_id: shop_id.clone(),
                        item_id: entry.item.clone(),
                        keeper,
                        asking,
                        floor,
                        rounds_used: rounds_used + 1,
                    },),
                );
                conn.send_line(&format!(
                    "{keeper_display} considers your offer of {} for a moment, then counters with {}.",
                    format_copper(offer),
                    format_copper(counter)
                ));
            }
            core::CounterOutcome::Refuse => {
                end_negotiation(world, entity, &shop_id, params.haggle_cooldown_secs);
                conn.send_line(&format!(
                    "{keeper_display} is offended and refuses to deal with you."
                ));
            }
        }
        return;
    }

    complete_purchase(world, conn, entity, keeper, &entry.item, asking, &templates);
    let _ = world.remove_one::<core::PendingHaggle>(entity);
}

fn cmd_sell(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let item_query = args.trim();
    if item_query.is_empty() {
        conn.send_line("Sell what?");
        return;
    }

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Server error: templates unavailable.");
            return;
        }
    };

    let (keeper, _shop_id, shop) = match find_shop(world, entity, &templates) {
        Some(found) => found,
        None => {
            conn.send_line("There is no shopkeeper here.");
            return;
        }
    };
    let keeper_name = keeper_name(world, keeper);

    let item_ent = match super::common::find_item_in_inventory(world, entity, item_query) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    let template_id = match world
        .query_one::<&core::Item>(item_ent)
        .ok()
        .and_then(|mut q| q.get().map(|i| i.template_id.clone()))
    {
        Some(id) => id,
        None => {
            conn.send_line("You can't sell that.");
            return;
        }
    };

    let item_tmpl = match templates.items.get(&template_id) {
        Some(t) => t,
        None => {
            conn.send_line("You can't sell that.");
            return;
        }
    };
    let item_name = world
        .query_one::<&core::Name>(item_ent)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.0.clone()))
        .unwrap_or_else(|| item_tmpl.name.clone());

    if !core::shop_buys_item(shop, item_tmpl) {
        conn.send_line(&format!("{keeper_name} has no interest in {item_name}."));
        return;
    }

    let rep_mult = core::reputation_multiplier(world, entity, keeper, shop, &templates);
    let price = core::sell_price(shop, item_tmpl.value, rep_mult);

    if let Ok(mut q) = world.query_one::<&mut core::Inventory>(entity) {
        if let Some(inv) = q.get() {
            inv.0.retain(|e| *e != item_ent);
        }
    }
    let _ = world.despawn(item_ent);

    let mut wallet = match world.query_one::<&mut core::Wallet>(entity) {
        Ok(mut q) => match q.get() {
            Some(w) => w.clone(),
            None => {
                conn.send_line("You have no money.");
                return;
            }
        },
        Err(_) => {
            conn.send_line("You have no money.");
            return;
        }
    };
    wallet.add(&core::Wallet::new(price, 0, 0, 0));
    let _ = world.insert(entity, (wallet, core::Dirty));

    conn.send_line(&format!(
        "You sell {item_name} for {}.",
        format_copper(price)
    ));
}

fn cmd_value(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let entity = match conn.entity() {
        Some(e) => e,
        None => {
            conn.send_line("You have no form.");
            return;
        }
    };

    let item_query = args.trim();
    if item_query.is_empty() {
        conn.send_line("Value what?");
        return;
    }

    let templates = match oxide_server::get_templates() {
        Some(t) => t,
        None => {
            conn.send_line("Server error: templates unavailable.");
            return;
        }
    };

    let (keeper, _shop_id, shop) = match find_shop(world, entity, &templates) {
        Some(found) => found,
        None => {
            conn.send_line("There is no shopkeeper here.");
            return;
        }
    };
    let keeper_name = keeper_name(world, keeper);

    let item_ent = match super::common::find_item_in_inventory(world, entity, item_query) {
        Some(i) => i,
        None => {
            conn.send_line("You don't have that item.");
            return;
        }
    };

    let template_id = match world
        .query_one::<&core::Item>(item_ent)
        .ok()
        .and_then(|mut q| q.get().map(|i| i.template_id.clone()))
    {
        Some(id) => id,
        None => {
            conn.send_line("You can't sell that.");
            return;
        }
    };

    let item_tmpl = match templates.items.get(&template_id) {
        Some(t) => t,
        None => {
            conn.send_line("You can't sell that.");
            return;
        }
    };
    let item_name = world
        .query_one::<&core::Name>(item_ent)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.0.clone()))
        .unwrap_or_else(|| item_tmpl.name.clone());

    if !core::shop_buys_item(shop, item_tmpl) {
        conn.send_line(&format!("{keeper_name} has no interest in {item_name}."));
        return;
    }

    let rep_mult = core::reputation_multiplier(world, entity, keeper, shop, &templates);
    let price = core::sell_price(shop, item_tmpl.value, rep_mult);

    conn.send_line(&format!(
        "{keeper_name} would pay you {} for {item_name}.",
        format_copper(price)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_helpers::{init_test_templates, test_player, test_world};
    use std::collections::HashMap;

    fn spawn_keeper(world: &mut World, room: core::Entity) -> core::Entity {
        world.spawn((
            core::Position::new(room),
            core::Name::new("Elias the Merchant"),
            core::Shopkeeper {
                shop_id: "test_shop".to_string(),
            },
            core::ShopStock(HashMap::from([
                ("healing_salve".to_string(), 3),
                ("torch".to_string(), 5),
            ])),
            core::LastRestock(std::time::Instant::now()),
        ))
    }

    fn wallet_total(world: &World, entity: core::Entity) -> u64 {
        world
            .query_one::<&core::Wallet>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .map(|w| w.total_copper())
            .unwrap_or(0)
    }

    fn inventory_len(world: &World, entity: core::Entity) -> usize {
        world
            .query_one::<&core::Inventory>(entity)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .map(|inv| inv.0.len())
            .unwrap_or(0)
    }

    fn stock_of(world: &World, keeper: core::Entity, item_id: &str) -> u64 {
        world
            .query_one::<&core::ShopStock>(keeper)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .map(|s| s.count(item_id))
            .unwrap_or(0)
    }

    fn has_dirty(world: &World, entity: core::Entity) -> bool {
        world
            .query_one::<&core::Dirty>(entity)
            .ok()
            .is_some_and(|mut q| q.get().is_some())
    }

    fn has_haggle_cooldown(world: &World, entity: core::Entity) -> bool {
        world
            .query_one::<&core::HaggleCooldown>(entity)
            .ok()
            .is_some_and(|mut q| q.get().is_some())
    }

    fn has_pending_haggle(world: &World, entity: core::Entity) -> bool {
        world
            .query_one::<&core::PendingHaggle>(entity)
            .ok()
            .is_some_and(|mut q| q.get().is_some())
    }

    #[test]
    fn list_requires_shopkeeper() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);
        cmd_list(&mut world, &mut conn, "list", "", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("no shopkeeper")));
    }

    #[test]
    fn buy_requires_shopkeeper() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let (_player, mut conn, registry) = test_player(&mut world, room_a);
        cmd_buy(&mut world, &mut conn, "buy", "salve", &registry);
        let lines = conn.take_lines();
        assert!(lines.iter().any(|l| l.contains("no shopkeeper")));
    }

    #[test]
    fn list_shows_asking_prices() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );

        cmd_list(&mut world, &mut conn, "list", "", &registry);
        let joined = conn.take_lines().join("\n");
        assert!(joined.contains("Healing Salve"), "output: {joined}");
        assert!(joined.contains("Torch"), "output: {joined}");
        assert!(joined.contains("45cp"), "output: {joined}"); // 30 * 1.5
    }

    #[test]
    fn buy_at_asking_price() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let keeper = spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );

        cmd_buy(&mut world, &mut conn, "buy", "salve", &registry);
        let lines = conn.take_lines();
        assert!(
            lines
                .iter()
                .any(|l| l.contains("You buy Healing Salve for 45cp.")),
            "lines: {lines:?}"
        );
        assert_eq!(wallet_total(&world, player), 50000 - 45);
        assert_eq!(stock_of(&world, keeper, "healing_salve"), 2);
        assert_eq!(inventory_len(&world, player), 1);
        assert!(has_dirty(&world, player));
    }

    #[test]
    fn buy_insufficient_funds() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let keeper = spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 0, 0), core::Inventory::new()),
        );

        cmd_buy(&mut world, &mut conn, "buy", "salve", &registry);
        let lines = conn.take_lines();
        assert!(
            lines.iter().any(|l| l.contains("can't afford")),
            "lines: {lines:?}"
        );
        assert_eq!(stock_of(&world, keeper, "healing_salve"), 3);
        assert_eq!(inventory_len(&world, player), 0);
    }

    #[test]
    fn buy_out_of_stock() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let keeper = spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );
        if let Ok(mut q) = world.query_one::<&mut core::ShopStock>(keeper) {
            if let Some(stock) = q.get() {
                stock.0.insert("healing_salve".to_string(), 0);
            }
        }

        cmd_buy(&mut world, &mut conn, "buy", "salve", &registry);
        let lines = conn.take_lines();
        assert!(
            lines.iter().any(|l| l.contains("out of Healing Salve")),
            "lines: {lines:?}"
        );
        assert_eq!(inventory_len(&world, player), 0);
    }

    #[test]
    fn buy_unknown_item() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );

        cmd_buy(&mut world, &mut conn, "buy", "dragon egg", &registry);
        let lines = conn.take_lines();
        assert!(
            lines.iter().any(|l| l.contains("doesn't sell that")),
            "lines: {lines:?}"
        );
    }

    #[test]
    fn sell_item_to_shop() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 0, 0), core::Inventory::new()),
        );

        let item = world.spawn((
            core::Item::new("healing_salve"),
            core::Name::new("Healing Salve"),
        ));
        if let Ok(mut q) = world.query_one::<&mut core::Inventory>(player) {
            if let Some(inv) = q.get() {
                inv.0.push(item);
            }
        }

        cmd_sell(&mut world, &mut conn, "sell", "salve", &registry);
        let lines = conn.take_lines();
        // 25 value * 0.5 buy_rate = 12.5 -> rounds to 13
        assert!(
            lines
                .iter()
                .any(|l| l.contains("You sell Healing Salve for 13cp.")),
            "lines: {lines:?}"
        );
        assert_eq!(wallet_total(&world, player), 13);
        assert_eq!(inventory_len(&world, player), 0);
        assert!(!world.contains(item));
        assert!(has_dirty(&world, player));
    }

    #[test]
    fn sell_item_shop_does_not_buy() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 0, 0), core::Inventory::new()),
        );

        let item = world.spawn((core::Item::new("torch"), core::Name::new("Torch")));
        if let Ok(mut q) = world.query_one::<&mut core::Inventory>(player) {
            if let Some(inv) = q.get() {
                inv.0.push(item);
            }
        }

        cmd_sell(&mut world, &mut conn, "sell", "torch", &registry);
        let lines = conn.take_lines();
        assert!(
            lines.iter().any(|l| l.contains("has no interest in Torch")),
            "lines: {lines:?}"
        );
        assert_eq!(wallet_total(&world, player), 0);
        assert_eq!(inventory_len(&world, player), 1);
        assert!(world.contains(item));
    }

    #[test]
    fn value_shows_buyback_price() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 0, 0), core::Inventory::new()),
        );

        let item = world.spawn((
            core::Item::new("healing_salve"),
            core::Name::new("Healing Salve"),
        ));
        if let Ok(mut q) = world.query_one::<&mut core::Inventory>(player) {
            if let Some(inv) = q.get() {
                inv.0.push(item);
            }
        }

        cmd_value(&mut world, &mut conn, "value", "salve", &registry);
        let lines = conn.take_lines();
        assert!(
            lines
                .iter()
                .any(|l| l.contains("would pay you 13cp for Healing Salve.")),
            "lines: {lines:?}"
        );
    }

    #[test]
    fn haggle_accepts_at_floor() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let keeper = spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );

        // asking = 30 * 1.5 = 45, floor = round(45 * 0.75) = 34
        cmd_buy(&mut world, &mut conn, "buy", "salve 34", &registry);
        let lines = conn.take_lines();
        assert!(
            lines
                .iter()
                .any(|l| l.contains("You buy Healing Salve for 34cp.")),
            "lines: {lines:?}"
        );
        assert_eq!(wallet_total(&world, player), 50000 - 34);
        assert_eq!(stock_of(&world, keeper, "healing_salve"), 2);
        assert!(has_haggle_cooldown(&world, player));
        assert!(!has_pending_haggle(&world, player));
    }

    #[test]
    fn haggle_counter_then_accept() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let keeper = spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );

        // offer 30 is between insult (27) and floor (34) -> counter 37
        cmd_buy(&mut world, &mut conn, "buy", "salve 30", &registry);
        let lines = conn.take_lines();
        assert!(
            lines.iter().any(|l| l.contains("counters with 37cp")),
            "lines: {lines:?}"
        );
        assert!(has_pending_haggle(&world, player));
        assert_eq!(wallet_total(&world, player), 50000);

        // offer 40 clears the floor -> accept at 40
        cmd_buy(&mut world, &mut conn, "buy", "salve 40", &registry);
        let lines = conn.take_lines();
        assert!(
            lines
                .iter()
                .any(|l| l.contains("You buy Healing Salve for 40cp.")),
            "lines: {lines:?}"
        );
        assert_eq!(wallet_total(&world, player), 50000 - 40);
        assert_eq!(stock_of(&world, keeper, "healing_salve"), 2);
    }

    #[test]
    fn haggle_insulting_offer_refuses() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        let keeper = spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );

        // insult threshold = round(45 * 0.6) = 27
        cmd_buy(&mut world, &mut conn, "buy", "salve 20", &registry);
        let lines = conn.take_lines();
        assert!(
            lines.iter().any(|l| l.contains("refuses to deal")),
            "lines: {lines:?}"
        );
        assert_eq!(stock_of(&world, keeper, "healing_salve"), 3);
        assert!(has_haggle_cooldown(&world, player));
        assert!(!has_pending_haggle(&world, player));
    }

    #[test]
    fn haggle_cooldown_blocks_second_attempt() {
        let _guard = init_test_templates();
        let (mut world, _void, room_a, _room_b) = test_world();
        spawn_keeper(&mut world, room_a);
        let (player, mut conn, registry) = test_player(&mut world, room_a);
        let _ = world.insert(
            player,
            (core::Wallet::new(0, 0, 5, 0), core::Inventory::new()),
        );

        cmd_buy(&mut world, &mut conn, "buy", "salve 20", &registry);
        let _ = conn.take_lines();

        cmd_buy(&mut world, &mut conn, "buy", "salve 40", &registry);
        let lines = conn.take_lines();
        assert!(
            lines.iter().any(|l| l.contains("won't negotiate again")),
            "lines: {lines:?}"
        );
        assert_eq!(wallet_total(&world, player), 50000);
        assert_eq!(inventory_len(&world, player), 0);
    }

    #[test]
    fn format_copper_denominations() {
        assert_eq!(format_copper(30), "30cp");
        assert_eq!(format_copper(150), "1sp 50cp");
        assert_eq!(format_copper(15_000), "1gp 50sp");
        assert_eq!(format_copper(1), "1cp");
        assert_eq!(format_copper(0), "0cp");
    }
}
