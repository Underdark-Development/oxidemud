use crate::{
    ActiveEffect, Armor, DbId, Entity, Equipment, EquipmentSlot, Formation, Group, GroupInvite,
    GroupManager, GroupMember, GroupMemberInfo, GroupRole, LootMode, Name, Player, Position, World,
};
use std::time::{Duration, Instant};

fn find_player_by_name(world: &World, name: &str) -> Option<Entity> {
    for (ent, (n_comp, _player_comp)) in world.query::<(&Name, &Player)>().iter() {
        if n_comp.as_str().eq_ignore_ascii_case(name) {
            return Some(ent);
        }
    }
    None
}

pub fn get_or_create_group_manager(world: &mut World) -> Entity {
    let gm_entity = world
        .query::<&GroupManager>()
        .iter()
        .next()
        .map(|(ent, _)| ent);
    if let Some(e) = gm_entity {
        e
    } else {
        world.spawn((GroupManager::default(),))
    }
}

pub fn handle_group_invite(
    world: &mut World,
    inviter: Entity,
    target_name: &str,
) -> Result<String, String> {
    if target_name.is_empty() {
        return Err("Invite who?".to_string());
    }

    let target = find_player_by_name(world, target_name)
        .ok_or_else(|| "No player by that name is online.".to_string())?;

    if target == inviter {
        return Err("You cannot invite yourself.".to_string());
    }

    // Check if target is already in a group
    if world
        .query_one::<&GroupMember>(target)
        .is_ok_and(|mut q| q.get().is_some())
    {
        return Err(format!("{} is already in a group.", target_name));
    }

    // If inviter is in a group, verify they are the leader and the group is not full
    let mut group_id_opt = None;
    if let Ok(mut q) = world.query_one::<&GroupMember>(inviter) {
        if let Some(gm) = q.get() {
            if gm.role != GroupRole::Leader {
                return Err("Only the group leader can invite new members.".to_string());
            }
            group_id_opt = Some(gm.group_id);

            // Check group size limit
            if let Ok(mut q_group) = world.query_one::<&Group>(gm.group_id) {
                if let Some(group) = q_group.get() {
                    if group.members.len() >= 6 {
                        return Err("The group is full (max 6 members).".to_string());
                    }
                }
            }
        }
    }

    // Add invite
    let gm_ent = get_or_create_group_manager(world);
    if let Ok(mut q_gm) = world.query_one::<&mut GroupManager>(gm_ent) {
        if let Some(gm) = q_gm.get() {
            // Remove any stale invite to the same target from the same inviter
            gm.invites
                .retain(|inv| !(inv.target == target && inv.from == inviter));

            gm.invites.push(GroupInvite {
                target,
                from: inviter,
                group_id: group_id_opt,
                expires_at: Instant::now() + Duration::from_secs(30),
            });
        }
    }

    Ok(format!("You invite {} to join your group.", target_name))
}

pub fn handle_group_accept(
    world: &mut World,
    invitee: Entity,
) -> Result<(Entity, Entity, String), String> {
    let gm_ent = get_or_create_group_manager(world);
    let mut invite_opt = None;

    if let Ok(mut q_gm) = world.query_one::<&mut GroupManager>(gm_ent) {
        if let Some(gm) = q_gm.get() {
            // Clean expired invites first
            let now = Instant::now();
            gm.invites.retain(|inv| inv.expires_at > now);

            // Find invite for invitee
            if let Some(pos) = gm.invites.iter().position(|inv| inv.target == invitee) {
                invite_opt = Some(gm.invites.remove(pos));
            }
        }
    }

    let invite = invite_opt.ok_or_else(|| "You have no pending group invites.".to_string())?;
    let inviter = invite.from;

    // Check if target is already in a group
    if world
        .query_one::<&GroupMember>(invitee)
        .is_ok_and(|mut q| q.get().is_some())
    {
        return Err("You are already in a group.".to_string());
    }

    let invitee_db_id = world
        .query_one::<&DbId>(invitee)
        .ok()
        .and_then(|mut q| q.get().map(|d| d.0))
        .unwrap_or(0);
    let invitee_name = world
        .query_one::<&Name>(invitee)
        .ok()
        .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
        .unwrap_or_else(|| "Someone".to_string());

    if let Some(group_entity) = invite.group_id {
        // Joining existing group
        let mut success = false;

        if let Ok(mut q_group) = world.query_one::<&mut Group>(group_entity) {
            if let Some(group) = q_group.get() {
                if group.members.len() >= 6 {
                    return Err("The group is full.".to_string());
                }
                group.members.push(GroupMemberInfo {
                    entity: Some(invitee),
                    db_id: invitee_db_id,
                    name: invitee_name.clone(),
                    joined_at: Instant::now(),
                    disconnected_at: None,
                });
                success = true;
            }
        }

        if success {
            let _ = world.insert(
                invitee,
                (GroupMember {
                    group_id: group_entity,
                    role: GroupRole::Member,
                },),
            );
            let _ = world.insert(invitee, (crate::Dirty,));

            Ok((inviter, group_entity, invitee_name))
        } else {
            Err("That group no longer exists.".to_string())
        }
    } else {
        // Creating new group
        if world
            .query_one::<&GroupMember>(inviter)
            .is_ok_and(|mut q| q.get().is_some())
        {
            return Err("The inviter has already joined another group.".to_string());
        }

        let inviter_db_id = world
            .query_one::<&DbId>(inviter)
            .ok()
            .and_then(|mut q| q.get().map(|d| d.0))
            .unwrap_or(0);
        let inviter_name = world
            .query_one::<&Name>(inviter)
            .ok()
            .and_then(|mut q| q.get().map(|n| n.as_str().to_string()))
            .unwrap_or_else(|| "Someone".to_string());

        let group = Group {
            leader: inviter,
            members: vec![
                GroupMemberInfo {
                    entity: Some(inviter),
                    db_id: inviter_db_id,
                    name: inviter_name.clone(),
                    joined_at: Instant::now(),
                    disconnected_at: None,
                },
                GroupMemberInfo {
                    entity: Some(invitee),
                    db_id: invitee_db_id,
                    name: invitee_name.clone(),
                    joined_at: Instant::now(),
                    disconnected_at: None,
                },
            ],
            loot_mode: LootMode::FreeForAll,
            formation: Formation::Default,
        };

        let group_entity = world.spawn((group,));

        let _ = world.insert(
            inviter,
            (GroupMember {
                group_id: group_entity,
                role: GroupRole::Leader,
            },),
        );
        let _ = world.insert(
            invitee,
            (GroupMember {
                group_id: group_entity,
                role: GroupRole::Member,
            },),
        );

        let _ = world.insert(inviter, (crate::Dirty,));
        let _ = world.insert(invitee, (crate::Dirty,));

        Ok((inviter, group_entity, invitee_name))
    }
}

pub fn handle_group_leave(
    world: &mut World,
    member: Entity,
) -> Result<(Entity, Vec<Entity>, String), String> {
    let gm = world
        .query_one::<&GroupMember>(member)
        .ok()
        .and_then(|mut q| q.get().copied())
        .ok_or_else(|| "You are not in a group.".to_string())?;

    let group_entity = gm.group_id;
    let mut disbanded = false;
    let mut remaining_active = Vec::new();
    let mut leave_msg = "You have left the group.".to_string();
    let mut lone_member_to_clean = None;
    let mut new_leader_to_insert = None;

    if let Ok(mut q_group) = world.query_one::<&mut Group>(group_entity) {
        if let Some(group) = q_group.get() {
            // Remove member from group list
            group.members.retain(|m| m.entity != Some(member));

            let active_members: Vec<Entity> =
                group.members.iter().filter_map(|m| m.entity).collect();

            if active_members.is_empty() {
                disbanded = true;
                leave_msg = "You left the group. The group has disbanded.".to_string();
            } else if active_members.len() == 1 && group.members.len() == 1 {
                // Only 1 member left total (no disconnected grace members) -> disband
                disbanded = true;
                lone_member_to_clean = Some(active_members[0]);
                leave_msg = "You left the group. The group has disbanded.".to_string();
            } else {
                remaining_active = active_members.clone();
                if group.leader == member {
                    // Leader left -> transfer leadership to next active member
                    let new_leader = active_members[0];
                    group.leader = new_leader;
                    new_leader_to_insert = Some(new_leader);
                }
            }
        }
    }

    if let Some(lone) = lone_member_to_clean {
        let _ = world.remove_one::<GroupMember>(lone);
        let _ = world.insert(lone, (crate::Dirty,));
    }

    if let Some(nl) = new_leader_to_insert {
        let _ = world.insert(
            nl,
            (GroupMember {
                group_id: group_entity,
                role: GroupRole::Leader,
            },),
        );
        let _ = world.insert(nl, (crate::Dirty,));
    }

    let _ = world.remove_one::<GroupMember>(member);
    let _ = world.insert(member, (crate::Dirty,));

    if disbanded {
        let _ = world.despawn(group_entity);
    }

    Ok((group_entity, remaining_active, leave_msg))
}

pub fn handle_group_disband(
    world: &mut World,
    leader: Entity,
) -> Result<(Vec<Entity>, String), String> {
    let gm = world
        .query_one::<&GroupMember>(leader)
        .ok()
        .and_then(|mut q| q.get().copied())
        .ok_or_else(|| "You are not in a group.".to_string())?;

    if gm.role != GroupRole::Leader {
        return Err("Only the group leader can disband the group.".to_string());
    }

    let group_entity = gm.group_id;
    let mut active_members = Vec::new();

    if let Ok(mut q_group) = world.query_one::<&Group>(group_entity) {
        if let Some(group) = q_group.get() {
            active_members = group.members.iter().filter_map(|m| m.entity).collect();
        }
    }

    for member in &active_members {
        let _ = world.remove_one::<GroupMember>(*member);
        let _ = world.insert(*member, (crate::Dirty,));
    }

    let _ = world.despawn(group_entity);

    Ok((active_members, "The group has been disbanded.".to_string()))
}

pub fn handle_group_kick(
    world: &mut World,
    leader: Entity,
    target_name: &str,
) -> Result<(Entity, Entity, String), String> {
    let gm = world
        .query_one::<&GroupMember>(leader)
        .ok()
        .and_then(|mut q| q.get().copied())
        .ok_or_else(|| "You are not in a group.".to_string())?;

    if gm.role != GroupRole::Leader {
        return Err("Only the group leader can kick members.".to_string());
    }

    let group_entity = gm.group_id;
    let mut kicked_entity = None;
    let mut disbanded = false;
    let mut lone_member_to_clean = None;

    if let Ok(mut q_group) = world.query_one::<&mut Group>(group_entity) {
        if let Some(group) = q_group.get() {
            if let Some(pos) = group
                .members
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(target_name))
            {
                let m_info = group.members.remove(pos);
                kicked_entity = m_info.entity;

                let active_members: Vec<Entity> =
                    group.members.iter().filter_map(|m| m.entity).collect();

                if active_members.is_empty() {
                    disbanded = true;
                } else if active_members.len() == 1 && group.members.len() == 1 {
                    disbanded = true;
                    lone_member_to_clean = Some(active_members[0]);
                }
            } else {
                return Err(format!("{} is not in your group.", target_name));
            }
        }
    }

    let kicked = kicked_entity.ok_or_else(|| format!("{} is not in your group.", target_name))?;

    if let Some(lone) = lone_member_to_clean {
        let _ = world.remove_one::<GroupMember>(lone);
        let _ = world.insert(lone, (crate::Dirty,));
    }

    let _ = world.remove_one::<GroupMember>(kicked);
    let _ = world.insert(kicked, (crate::Dirty,));

    if disbanded {
        let _ = world.despawn(group_entity);
    }

    Ok((
        group_entity,
        kicked,
        format!("You have kicked {} from the group.", target_name),
    ))
}

pub fn handle_group_loot(
    world: &mut World,
    leader: Entity,
    mode_str: &str,
) -> Result<LootMode, String> {
    let gm = world
        .query_one::<&GroupMember>(leader)
        .ok()
        .and_then(|mut q| q.get().copied())
        .ok_or_else(|| "You are not in a group.".to_string())?;

    if gm.role != GroupRole::Leader {
        return Err("Only the group leader can change the loot mode.".to_string());
    }

    let mode = match mode_str.to_lowercase().as_str() {
        "freeforall" | "ffa" => LootMode::FreeForAll,
        "roundrobin" | "rr" => LootMode::RoundRobin,
        "master" | "ml" => LootMode::MasterLooter,
        _ => return Err("Invalid loot mode. Choose: freeforall, roundrobin, master".to_string()),
    };

    if let Ok(mut q_group) = world.query_one::<&mut Group>(gm.group_id) {
        if let Some(group) = q_group.get() {
            group.loot_mode = mode;
        }
    }

    Ok(mode)
}

pub fn handle_group_formation(
    world: &mut World,
    leader: Entity,
    form_str: &str,
) -> Result<Formation, String> {
    let gm = world
        .query_one::<&GroupMember>(leader)
        .ok()
        .and_then(|mut q| q.get().copied())
        .ok_or_else(|| "You are not in a group.".to_string())?;

    if gm.role != GroupRole::Leader {
        return Err("Only the group leader can change the formation.".to_string());
    }

    let formation =
        match form_str.to_lowercase().as_str() {
            "default" => Formation::Default,
            "line" => Formation::Line,
            "scattered" => Formation::Scattered,
            "column" => Formation::Column,
            "wedge" => Formation::Wedge,
            "shieldwall" | "shield wall" => Formation::ShieldWall,
            _ => return Err(
                "Invalid formation. Choose: default, line, scattered, column, wedge, shieldwall"
                    .to_string(),
            ),
        };

    if let Ok(mut q_group) = world.query_one::<&mut Group>(gm.group_id) {
        if let Some(group) = q_group.get() {
            group.formation = formation;
        }
    }

    Ok(formation)
}

pub fn handle_group_leader(
    world: &mut World,
    leader: Entity,
    target_name: &str,
) -> Result<(Entity, String), String> {
    let gm = world
        .query_one::<&GroupMember>(leader)
        .ok()
        .and_then(|mut q| q.get().copied())
        .ok_or_else(|| "You are not in a group.".to_string())?;

    if gm.role != GroupRole::Leader {
        return Err("Only the group leader can transfer leadership.".to_string());
    }

    let group_entity = gm.group_id;
    let mut new_leader_entity = None;

    if let Ok(mut q_group) = world.query_one::<&mut Group>(group_entity) {
        if let Some(group) = q_group.get() {
            if let Some(m) = group
                .members
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(target_name))
            {
                if m.entity.is_none() {
                    return Err(format!(
                        "{} is disconnected. Cannot transfer leadership.",
                        target_name
                    ));
                }
                new_leader_entity = m.entity;
            } else {
                return Err(format!("{} is not in your group.", target_name));
            }

            if let Some(new_leader) = new_leader_entity {
                group.leader = new_leader;
            }
        }
    }

    let new_leader =
        new_leader_entity.ok_or_else(|| format!("{} is not in your group.", target_name))?;

    // Update components
    let _ = world.insert(
        leader,
        (GroupMember {
            group_id: group_entity,
            role: GroupRole::Member,
        },),
    );
    let _ = world.insert(
        new_leader,
        (GroupMember {
            group_id: group_entity,
            role: GroupRole::Leader,
        },),
    );

    let _ = world.insert(leader, (crate::Dirty,));
    let _ = world.insert(new_leader, (crate::Dirty,));

    Ok((
        new_leader,
        format!("You transfer group leadership to {}.", target_name),
    ))
}

pub fn handle_player_disconnect_group(world: &mut World, player_entity: Entity) {
    let gm_opt = world
        .query_one::<&GroupMember>(player_entity)
        .ok()
        .and_then(|mut q| q.get().copied());

    if let Some(gm) = gm_opt {
        let group_entity = gm.group_id;
        let mut disband_group = false;
        let mut lone_to_clean = None;
        let mut new_leader_to_insert = None;

        if let Ok(mut q_group) = world.query_one::<&mut Group>(group_entity) {
            if let Some(group) = q_group.get() {
                if let Some(m) = group
                    .members
                    .iter_mut()
                    .find(|m| m.entity == Some(player_entity))
                {
                    m.entity = None;
                    m.disconnected_at = Some(Instant::now());
                }

                let active_members: Vec<Entity> =
                    group.members.iter().filter_map(|m| m.entity).collect();

                if active_members.is_empty() {
                    // All members offline - cleanup tick will handle it
                } else if active_members.len() == 1 && group.members.len() == 1 {
                    disband_group = true;
                    lone_to_clean = Some(active_members[0]);
                } else if group.leader == player_entity {
                    let new_leader = active_members[0];
                    group.leader = new_leader;
                    new_leader_to_insert = Some(new_leader);
                }
            }
        }

        if let Some(lone) = lone_to_clean {
            let _ = world.remove_one::<GroupMember>(lone);
            let _ = world.insert(lone, (crate::Dirty,));
        }

        if let Some(nl) = new_leader_to_insert {
            let _ = world.insert(
                nl,
                (GroupMember {
                    group_id: group_entity,
                    role: GroupRole::Leader,
                },),
            );
            let _ = world.insert(nl, (crate::Dirty,));
        }

        if disband_group {
            let _ = world.despawn(group_entity);
        }
    }
    let _ = world.remove_one::<GroupMember>(player_entity);
}

pub fn handle_player_login_group(
    world: &mut World,
    player_entity: Entity,
    player_db_id: i64,
    _player_name: &str,
) {
    let mut join_opt = None;

    // Find if player is in any group's grace list
    for (group_ent, group) in world.query::<&mut Group>().iter() {
        if let Some(m) = group
            .members
            .iter_mut()
            .find(|m| m.db_id == player_db_id && m.entity.is_none())
        {
            m.entity = Some(player_entity);
            m.disconnected_at = None;

            // If the leader is offline, this player who is logging in must reclaim leadership!
            let leader_active = group
                .members
                .iter()
                .any(|mem| mem.entity.is_some() && mem.entity == Some(group.leader));
            let is_leader = !leader_active;

            join_opt = Some((group_ent, is_leader));
            break;
        }
    }

    if let Some((group_entity, is_leader)) = join_opt {
        let role = if is_leader {
            // Update group's leader field to target the new entity
            if let Ok(mut q_group) = world.query_one::<&mut Group>(group_entity) {
                if let Some(group) = q_group.get() {
                    group.leader = player_entity;
                }
            }
            GroupRole::Leader
        } else {
            GroupRole::Member
        };

        let _ = world.insert(
            player_entity,
            (GroupMember {
                group_id: group_entity,
                role,
            },),
        );
        let _ = world.insert(player_entity, (crate::Dirty,));
    }
}

pub fn run_group_cleanup(world: &mut World, current_time: Instant) {
    let mut groups_to_update = Vec::new();
    let mut groups_to_disband = Vec::new();

    for (group_ent, group) in world.query::<&Group>().iter() {
        let group_entity = group_ent;
        let mut members = group.members.clone();
        let old_len = members.len();

        // Filter out expired grace members
        members.retain(|m| {
            if let Some(disc_time) = m.disconnected_at {
                current_time.duration_since(disc_time) < Duration::from_secs(60)
            } else {
                true
            }
        });

        let active_count = members.iter().filter(|m| m.entity.is_some()).count();

        if active_count == 0 || members.len() <= 1 {
            groups_to_disband.push(group_entity);
        } else if members.len() != old_len {
            groups_to_update.push((group_entity, members));
        }
    }

    for group_ent in groups_to_disband {
        let mut active_members = Vec::new();
        if let Ok(mut q_group) = world.query_one::<&Group>(group_ent) {
            if let Some(group) = q_group.get() {
                active_members = group.members.iter().filter_map(|m| m.entity).collect();
            }
        }
        for member in active_members {
            let _ = world.remove_one::<GroupMember>(member);
            let _ = world.insert(member, (crate::Dirty,));
        }
        let _ = world.despawn(group_ent);
    }

    for (group_ent, members) in groups_to_update {
        if let Ok(mut q_group) = world.query_one::<&mut Group>(group_ent) {
            if let Some(group) = q_group.get() {
                group.members = members;
            }
        }
    }
}

fn has_shield_equipped(world: &World, entity: Entity) -> bool {
    world
        .query_one::<&Equipment>(entity)
        .ok()
        .and_then(|mut q| {
            q.get()
                .and_then(|eq| eq.equipped(&EquipmentSlot::Shield).copied())
        })
        .is_some_and(|s| {
            world
                .query_one::<&Armor>(s)
                .is_ok_and(|mut q| q.get().is_some())
        })
}

pub fn run_formation_effects(world: &mut World) {
    let mut group_effects = Vec::new();

    // 1. Gather all group formations and their members present in the room with the leader
    for (_group_ent, group) in world.query::<&Group>().iter() {
        let leader = group.leader;

        let leader_room = world
            .query_one::<&Position>(leader)
            .ok()
            .and_then(|mut q| q.get().map(|p| p.room));

        if let Some(room) = leader_room {
            // Find all active members present in the same room
            let mut room_members = Vec::new();
            for m in &group.members {
                if let Some(ent) = m.entity {
                    let m_room = world
                        .query_one::<&Position>(ent)
                        .ok()
                        .and_then(|mut q| q.get().map(|p| p.room));
                    if m_room == Some(room) {
                        room_members.push(ent);
                    }
                }
            }

            // Ensure the leader is always first in the room members list
            if let Some(pos) = room_members.iter().position(|&e| e == leader) {
                room_members.remove(pos);
            }
            room_members.insert(0, leader);

            let size = room_members.len();

            let (min_size, valid) = match group.formation {
                Formation::Default => (0, true),
                Formation::Line => (2, size >= 2),
                Formation::Scattered => (2, size >= 2),
                Formation::Column => (3, size >= 3),
                Formation::Wedge => (3, size >= 3),
                Formation::ShieldWall => {
                    let all_have_shields =
                        room_members.iter().all(|&e| has_shield_equipped(world, e));
                    (2, size >= 2 && all_have_shields)
                }
            };

            if valid && min_size > 0 {
                for (idx, &member) in room_members.iter().enumerate() {
                    let is_leader = member == leader;

                    let mut effects = Vec::new();
                    match group.formation {
                        Formation::Line => {
                            let is_front = idx < size / 2 + size % 2;
                            if is_front {
                                effects.push(ActiveEffect {
                                    source: "formation".to_string(),
                                    stat: Some("ac".to_string()),
                                    amount: Some(1),
                                    aura_id: None,
                                    radius: None,
                                });
                            } else {
                                effects.push(ActiveEffect {
                                    source: "formation".to_string(),
                                    stat: Some("ac".to_string()),
                                    amount: Some(-1),
                                    aura_id: None,
                                    radius: None,
                                });
                            }
                        }
                        Formation::Scattered => {
                            effects.push(ActiveEffect {
                                source: "formation".to_string(),
                                stat: Some("ac".to_string()),
                                amount: Some(-2),
                                aura_id: None,
                                radius: None,
                            });
                            effects.push(ActiveEffect {
                                source: "formation".to_string(),
                                stat: Some("dodge".to_string()),
                                amount: Some(2),
                                aura_id: None,
                                radius: None,
                            });
                        }
                        Formation::Column if is_leader => {
                            effects.push(ActiveEffect {
                                source: "formation".to_string(),
                                stat: Some("damage".to_string()),
                                amount: Some(1),
                                aura_id: None,
                                radius: None,
                            });
                        }
                        Formation::Column => {}
                        Formation::Wedge => {
                            effects.push(ActiveEffect {
                                source: "formation".to_string(),
                                stat: Some("attack".to_string()),
                                amount: Some(2),
                                aura_id: None,
                                radius: None,
                            });
                            if is_leader {
                                effects.push(ActiveEffect {
                                    source: "formation".to_string(),
                                    stat: Some("ac".to_string()),
                                    amount: Some(-4),
                                    aura_id: None,
                                    radius: None,
                                });
                            }
                        }
                        Formation::ShieldWall => {
                            effects.push(ActiveEffect {
                                source: "formation".to_string(),
                                stat: Some("ac".to_string()),
                                amount: Some(2),
                                aura_id: None,
                                radius: None,
                            });
                            effects.push(ActiveEffect {
                                source: "formation".to_string(),
                                stat: Some("attack".to_string()),
                                amount: Some(-2),
                                aura_id: None,
                                radius: None,
                            });
                        }
                        _ => {}
                    }

                    if !effects.is_empty() {
                        group_effects.push((member, effects));
                    }
                }
            }
        }
    }

    // 2. Apply the active effects to each entity
    // We do this by clearing all "formation" effects and then adding new ones.
    let mut all_entities_to_clear = Vec::new();
    for (ent, _effs) in world.query::<&Vec<ActiveEffect>>().iter() {
        all_entities_to_clear.push(ent);
    }

    for ent in all_entities_to_clear {
        let mut existing = Vec::new();
        if let Ok(mut q) = world.query_one::<&Vec<ActiveEffect>>(ent) {
            if let Some(effs) = q.get() {
                existing = effs.clone();
            }
        }
        let old_len = existing.len();
        existing.retain(|e| e.source != "formation");
        if existing.len() != old_len {
            if existing.is_empty() {
                let _ = world.remove_one::<Vec<ActiveEffect>>(ent);
            } else {
                let _ = world.insert(ent, (existing,));
            }
            let _ = world.insert(ent, (crate::Dirty,));
        }
    }

    for (ent, new_effs) in group_effects {
        let mut existing = Vec::new();
        if let Ok(mut q) = world.query_one::<&Vec<ActiveEffect>>(ent) {
            if let Some(effs) = q.get() {
                existing = effs.clone();
            }
        }
        existing.extend(new_effs);
        let _ = world.insert(ent, (existing,));
        let _ = world.insert(ent, (crate::Dirty,));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Health, Level, Name, Player, Position};

    fn setup_test_player(world: &mut World, name: &str) -> Entity {
        world.spawn((
            Name::new(name),
            Player {
                account_id: 1,
                screen_width: 80,
                prompt: None,
                no_resurrect: false,
            },
            Level(1),
            DbId(fastrand::i64(1..10000)),
            Health::new(100),
            Position::new(Entity::from(hecs::Entity::DANGLING)),
        ))
    }

    #[test]
    fn test_group_lifecycle() {
        let mut world = World::new();
        let p1 = setup_test_player(&mut world, "Alice");
        let p2 = setup_test_player(&mut world, "Bob");

        // Invite Bob
        let invite_res = handle_group_invite(&mut world, p1, "Bob");
        assert!(invite_res.is_ok());

        // Accept invite
        let accept_res = handle_group_accept(&mut world, p2);
        assert!(accept_res.is_ok());
        let (inviter, group_ent, name) = accept_res.unwrap();
        assert_eq!(inviter, p1);
        assert_eq!(name, "Bob");

        // Verify components
        let gm1 = world
            .query_one::<&GroupMember>(p1)
            .unwrap()
            .get()
            .copied()
            .unwrap();
        let gm2 = world
            .query_one::<&GroupMember>(p2)
            .unwrap()
            .get()
            .copied()
            .unwrap();
        assert_eq!(gm1.group_id, group_ent);
        assert_eq!(gm1.role, GroupRole::Leader);
        assert_eq!(gm2.group_id, group_ent);
        assert_eq!(gm2.role, GroupRole::Member);

        // Kick Bob
        let kick_res = handle_group_kick(&mut world, p1, "Bob");
        assert!(kick_res.is_ok());

        // Group should disband because only 1 active left
        assert!(
            world.query_one::<&Group>(group_ent).is_err()
                || world
                    .query_one::<&Group>(group_ent)
                    .unwrap()
                    .get()
                    .is_none()
        );
        assert!(world.query_one::<&GroupMember>(p1).unwrap().get().is_none());
        assert!(world.query_one::<&GroupMember>(p2).unwrap().get().is_none());
    }

    #[test]
    fn test_formations() {
        let mut world = World::new();
        let room = world.spawn(());

        let p1 = setup_test_player(&mut world, "Alice");
        let p2 = setup_test_player(&mut world, "Bob");
        let p3 = setup_test_player(&mut world, "Charlie");

        // Set same room
        let _ = world.insert(p1, (Position::new(room),));
        let _ = world.insert(p2, (Position::new(room),));
        let _ = world.insert(p3, (Position::new(room),));

        // Form group
        let _ = handle_group_invite(&mut world, p1, "Bob");
        let _ = handle_group_accept(&mut world, p2);

        let _ = handle_group_invite(&mut world, p1, "Charlie");
        let _ = handle_group_accept(&mut world, p3);

        let gm = world
            .query_one::<&GroupMember>(p1)
            .unwrap()
            .get()
            .copied()
            .unwrap();
        let _group_ent = gm.group_id;

        // Wedge formation (requires 3 members)
        let wedge_res = handle_group_formation(&mut world, p1, "wedge");
        assert!(wedge_res.is_ok());

        run_formation_effects(&mut world);

        // Check leader p1 effects
        let effs1 = world
            .query_one::<&Vec<ActiveEffect>>(p1)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(effs1.iter().any(|e| e.source == "formation"
            && e.stat == Some("attack".to_string())
            && e.amount == Some(2)));
        assert!(effs1.iter().any(|e| e.source == "formation"
            && e.stat == Some("ac".to_string())
            && e.amount == Some(-4)));

        // Check member p2 effects
        let effs2 = world
            .query_one::<&Vec<ActiveEffect>>(p2)
            .unwrap()
            .get()
            .cloned()
            .unwrap();
        assert!(effs2.iter().any(|e| e.source == "formation"
            && e.stat == Some("attack".to_string())
            && e.amount == Some(2)));
        assert!(!effs2
            .iter()
            .any(|e| e.source == "formation" && e.stat == Some("ac".to_string())));
    }
}
