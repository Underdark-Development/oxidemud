use oxide_core::systems::combat::{CombatOutcome, CombatOutcomeKind};
use oxide_core::{Entity, Health, Inventory, Position, World};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::persistence::save_player_progress;
use crate::registry::ConnectionRegistry;

/// Process combat outcomes generated from `run_combat_pulse`.
/// Handles kill events (quest/faction progress, XP awards, looting) and sends output messages/prompts to involved players.
pub fn process_combat_pulse_results(
    world: &mut World,
    registry: &ConnectionRegistry,
    db: &Option<Arc<Mutex<oxide_data::Database>>>,
    outcomes: Vec<CombatOutcome>,
) {
    // Collect involved players before consuming outcomes
    let mut involved_players: Vec<Entity> = Vec::new();
    for o in &outcomes {
        if o.attacker_is_player && !involved_players.contains(&o.attacker) {
            involved_players.push(o.attacker);
        }
        if o.target_is_player && !involved_players.contains(&o.target) {
            involved_players.push(o.target);
        }
    }

    // Process kills (quest progress, faction standing, XP award, progress save)
    for outcome in &outcomes {
        if let CombatOutcomeKind::Killed {
            ref mob_template_id,
            ..
        } = &outcome.kind
        {
            if outcome.attacker_is_player {
                if let Some(templates) = crate::get_templates() {
                    if let Some(ref mob_id) = mob_template_id {
                        let quest_msgs = oxide_core::handle_kill_event(
                            world,
                            outcome.attacker,
                            mob_id,
                            &templates,
                        );
                        for msg in &quest_msgs {
                            if let Some(tx) = registry.sender(outcome.attacker) {
                                let _ = tx.send(format!("{msg}\r\n").into_bytes());
                            }
                        }

                        let faction_msgs = oxide_core::handle_faction_kill(
                            world,
                            outcome.attacker,
                            mob_id,
                            &templates,
                        );
                        for msg in &faction_msgs {
                            if let Some(tx) = registry.sender(outcome.attacker) {
                                let _ = tx.send(format!("{msg}\r\n").into_bytes());
                            }
                        }
                    }
                }

                // Determine all players to award XP / level-up check
                let mut players_to_award = vec![outcome.attacker];

                if let Ok(mut q_gm) = world.query_one::<&oxide_core::GroupMember>(outcome.attacker)
                {
                    if let Some(gm) = q_gm.get() {
                        let group_entity = gm.group_id;
                        if let Ok(mut q_group) = world.query_one::<&oxide_core::Group>(group_entity)
                        {
                            if let Some(group) = q_group.get() {
                                let attacker_room = world
                                    .query_one::<&oxide_core::Position>(outcome.attacker)
                                    .ok()
                                    .and_then(|mut q| q.get().map(|p| p.room));
                                if let Some(room) = attacker_room {
                                    for m in &group.members {
                                        if let Some(m_ent) = m.entity {
                                            if m_ent != outcome.attacker {
                                                let m_room = world
                                                    .query_one::<&oxide_core::Position>(m_ent)
                                                    .ok()
                                                    .and_then(|mut q| q.get().map(|p| p.room));
                                                if m_room == Some(room) {
                                                    players_to_award.push(m_ent);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                for player in players_to_award {
                    let msgs = crate::award_xp(world, player);
                    for msg in &msgs {
                        if let Some(tx) = registry.sender(player) {
                            let _ = tx.send(format!("{msg}\r\n").into_bytes());
                        }
                    }
                    if let Some(ref db) = db {
                        if let Ok(db_guard) = db.try_lock() {
                            save_player_progress(world, player, &db_guard);
                        }
                    }
                }
            }
        }
    }

    dispatch_combat_outcomes(registry, world, outcomes);

    // Send prompt to players involved in combat outcomes
    for entity in involved_players {
        crate::prompt::send_player_prompt(world, entity, registry);
    }
}

pub fn dispatch_combat_outcomes(
    registry: &ConnectionRegistry,
    world: &mut World,
    outcomes: Vec<CombatOutcome>,
) {
    for outcome in outcomes {
        match outcome.kind {
            CombatOutcomeKind::Hit {
                damage,
                unconscious,
                ..
            } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ = tx.send(
                            format!("You hit {} for {} damage.\r\n", outcome.target_name, damage)
                                .into_bytes(),
                        );
                    }
                } else if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!(
                                "{} hits you for {} damage.\r\n",
                                outcome.attacker_name, damage
                            )
                            .into_bytes(),
                        );
                        if unconscious {
                            let mut msg = "You fall unconscious!\r\n".to_string();
                            let mut room_msg =
                                format!("{} falls unconscious!\r\n", outcome.target_name);
                            if let Ok(mut q) = world.query_one::<&Health>(outcome.target) {
                                if let Some(hp) = q.get() {
                                    if hp.is_incapacitated() {
                                        msg = "You are incapacitated and will slowly die, if not aided.\r\n".to_string();
                                        room_msg = format!("{} is incapacitated and will slowly die, if not aided.\r\n", outcome.target_name);
                                    } else if hp.is_mortally_wounded() {
                                        msg = "You are mortally wounded and will slowly die, if not aided.\r\n".to_string();
                                        room_msg = format!("{} is mortally wounded and will slowly die, if not aided.\r\n", outcome.target_name);
                                    }
                                }
                            }
                            let _ = tx.send(msg.into_bytes());

                            if let Ok(mut q_pos) = world.query_one::<&Position>(outcome.target) {
                                if let Some(pos) = q_pos.get() {
                                    let room_msg_bytes = room_msg.into_bytes();
                                    for &other in &registry.occupants(world, pos.room) {
                                        if other != outcome.target {
                                            if let Some(other_tx) = registry.sender(other) {
                                                let _ = other_tx.send(room_msg_bytes.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            CombatOutcomeKind::Miss => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ =
                            tx.send(format!("You miss {}.\r\n", outcome.target_name).into_bytes());
                    }
                } else if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!("{} misses you.\r\n", outcome.attacker_name).into_bytes(),
                        );
                    }
                }
            }
            CombatOutcomeKind::Killed {
                damage,
                xp_gained,
                corpse,
                mob_template_id,
                mob_level,
                ..
            } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ = tx.send(
                            format!("You hit {} for {} damage.\r\n", outcome.target_name, damage)
                                .into_bytes(),
                        );
                        let _ = tx.send(
                            format!(
                                "You kill {}! You gain {} experience.\r\n",
                                outcome.target_name, xp_gained
                            )
                            .into_bytes(),
                        );
                    }
                }
                if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!(
                                "{} hits you for {} damage.\r\n",
                                outcome.attacker_name, damage
                            )
                            .into_bytes(),
                        );
                        let _ = tx.send(
                            format!("You have been slain by {}!\r\n", outcome.attacker_name)
                                .into_bytes(),
                        );
                        let _ = tx.send(b"Alas, you are dead! You are a ghost now...\r\n".to_vec());
                    }
                }

                // Broadcast death to others in the room
                if let Ok(mut q_pos) = world.query_one::<&Position>(outcome.attacker) {
                    if let Some(pos) = q_pos.get() {
                        let room_msg = format!("{} is dead! R.I.P.\r\n", outcome.target_name);
                        let room_msg_bytes = room_msg.into_bytes();
                        for &other in &registry.occupants(world, pos.room) {
                            if other != outcome.target && other != outcome.attacker {
                                if let Some(other_tx) = registry.sender(other) {
                                    let _ = other_tx.send(room_msg_bytes.clone());
                                }
                            }
                        }
                    }
                }

                // Loot spawning for NPC kills
                if let Some(ref mob_tmpl_id) = mob_template_id {
                    if let Some(templates) = crate::get_templates() {
                        if let Some(mob_tmpl) = templates.get_mob(mob_tmpl_id) {
                            if !mob_tmpl.loot.entries.is_empty() {
                                let spawns = oxide_core::systems::loot::roll_loot(
                                    &mob_tmpl.loot,
                                    mob_level,
                                    &templates,
                                );
                                for spawn in spawns {
                                    if let Some(item) = oxide_core::systems::loot::spawn_loot_item(
                                        world, &spawn, &templates,
                                    ) {
                                        // Add item to corpse's inventory
                                        if let Ok(mut q) = world.query_one::<&mut Inventory>(corpse)
                                        {
                                            if let Some(inv) = q.get() {
                                                inv.0.push(item);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            CombatOutcomeKind::FleeSuccess { dest: _, moved } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        if moved {
                            let _ = tx.send("You flee from combat!\r\n".to_string().into_bytes());
                        } else {
                            let _ = tx.send(
                                "You flee from combat, but there is nowhere to go!\r\n"
                                    .to_string()
                                    .into_bytes(),
                            );
                        }
                    }
                }
                if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!("{} flees from combat!\r\n", outcome.attacker_name)
                                .into_bytes(),
                        );
                    }
                }
            }
            CombatOutcomeKind::FleeFail { attempts } => {
                if outcome.attacker_is_player {
                    if let Some(tx) = registry.sender(outcome.attacker) {
                        let _ = tx.send(
                            format!("You failed to flee! (Attempt {})\r\n", attempts).into_bytes(),
                        );
                    }
                }
                if outcome.target_is_player {
                    if let Some(tx) = registry.sender(outcome.target) {
                        let _ = tx.send(
                            format!("{} attempts to flee, but fails!\r\n", outcome.attacker_name)
                                .into_bytes(),
                        );
                    }
                }
            }
        }
    }
}
