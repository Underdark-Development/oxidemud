//! Online-only immortal/admin handler implementations.

use rmcp::handler::server::wrapper::Parameters;

use crate::context::{rpc_error_message, HandlerContext};
use crate::params::*;

pub async fn list_connected_players(ctx: &HandlerContext<'_>) -> String {
    let client = match ctx.rpc().await {
        Ok(c) => c,
        Err(e) => return e,
    };
    match client.call("players.list", serde_json::json!({})).await {
        Ok(value) => {
            if let Some(players) = value.as_array() {
                if players.is_empty() {
                    return "No players currently online.".to_string();
                }
                let mut out = "### Connected Players:\n\n".to_string();
                out.push_str("| Name | Level | Class | Race | Room Key |\n");
                out.push_str("|---|---|---|---|---|\n");
                for p in players {
                    out.push_str(&format!(
                        "| {} | {} | {} | {} | {} |\n",
                        p.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                        p.get("level").and_then(|v| v.as_i64()).unwrap_or(1),
                        p.get("class").and_then(|v| v.as_str()).unwrap_or("None"),
                        p.get("race").and_then(|v| v.as_str()).unwrap_or("None"),
                        p.get("room_key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                    ));
                }
                out
            } else {
                "Error parsing players list from server.".to_string()
            }
        }
        Err(e) => rpc_error_message(e),
    }
}

pub async fn imm_put_item(ctx: &HandlerContext<'_>, params: Parameters<PutItemParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "item_template_id": p.item_template_id,
        "count": p.count
    });

    ctx.call_imm_prefixed("imm.put_item", payload).await
}

pub async fn imm_teleport(ctx: &HandlerContext<'_>, params: Parameters<TeleportParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "room_key": p.room_key
    });

    ctx.call_imm_prefixed("imm.teleport", payload).await
}

pub async fn imm_force_command(
    ctx: &HandlerContext<'_>,
    params: Parameters<ForceCommandParams>,
) -> String {
    let p = params.0;

    if !p.confirm {
        return "Error: This is a destructive operation. Set `confirm` to true to proceed."
            .to_string();
    }

    let payload = serde_json::json!({
        "player_name": p.player_name,
        "command": p.command
    });

    ctx.call_imm_prefixed("imm.force_command", payload).await
}

pub async fn imm_set_stat(ctx: &HandlerContext<'_>, params: Parameters<SetStatParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "strength": p.strength,
        "dexterity": p.dexterity,
        "intelligence": p.intelligence,
        "wisdom": p.wisdom,
        "constitution": p.constitution,
        "charisma": p.charisma,
        "hp": p.hp,
        "mana": p.mana,
        "stamina": p.stamina,
        "level": p.level,
        "xp": p.xp
    });

    ctx.call_imm("imm.set_stat", payload).await
}

pub async fn imm_load_mob(ctx: &HandlerContext<'_>, params: Parameters<LoadMobParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "room_key": p.room_key,
        "mob_template_id": p.mob_template_id
    });

    ctx.call_imm("imm.load_mob", payload).await
}

pub async fn imm_load_item(ctx: &HandlerContext<'_>, params: Parameters<LoadItemParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "room_key": p.room_key,
        "item_template_id": p.item_template_id,
        "count": p.count
    });

    ctx.call_imm("imm.load_item", payload).await
}

pub async fn imm_gecho(ctx: &HandlerContext<'_>, params: Parameters<GechoParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "message": p.message
    });

    ctx.call_imm("imm.gecho", payload).await
}

pub async fn imm_advance(ctx: &HandlerContext<'_>, params: Parameters<AdvanceParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "target_level": p.target_level
    });

    ctx.call_imm("imm.advance", payload).await
}

pub async fn imm_stat(ctx: &HandlerContext<'_>, params: Parameters<StatParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name
    });

    let client = match ctx.rpc().await {
        Ok(c) => c,
        Err(e) => return e,
    };
    match client.call("imm.stat", payload).await {
        Ok(value) => match serde_json::to_string(&value) {
            Ok(s) => s,
            Err(e) => format!("Failed to read response: {e}"),
        },
        Err(e) => rpc_error_message(e),
    }
}

pub async fn imm_heal(ctx: &HandlerContext<'_>, params: Parameters<HealParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name
    });

    ctx.call_imm("imm.heal", payload).await
}

pub async fn imm_damage(ctx: &HandlerContext<'_>, params: Parameters<DamageParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name,
        "amount": p.amount
    });

    ctx.call_imm("imm.damage", payload).await
}

pub async fn imm_kill(ctx: &HandlerContext<'_>, params: Parameters<KillParams>) -> String {
    let p = params.0;
    if !p.confirm {
        return "Error: This is a destructive operation. Set `confirm` to true to proceed."
            .to_string();
    }

    let payload = serde_json::json!({
        "target_name": p.target_name,
        "confirm": p.confirm
    });

    ctx.call_imm("imm.kill", payload).await
}

pub async fn imm_revive(ctx: &HandlerContext<'_>, params: Parameters<ReviveParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name
    });

    ctx.call_imm("imm.revive", payload).await
}

pub async fn imm_set_alignment(
    ctx: &HandlerContext<'_>,
    params: Parameters<SetAlignmentParams>,
) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "alignment": p.alignment
    });

    ctx.call_imm("imm.set_alignment", payload).await
}

pub async fn imm_set_faction(
    ctx: &HandlerContext<'_>,
    params: Parameters<SetFactionParams>,
) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "faction_id": p.faction_id,
        "standing": p.standing
    });

    ctx.call_imm("imm.set_faction", payload).await
}

pub async fn imm_purge_room(
    ctx: &HandlerContext<'_>,
    params: Parameters<PurgeRoomParams>,
) -> String {
    let p = params.0;
    if !p.confirm {
        return "Error: This is a destructive operation. Set `confirm` to true to proceed."
            .to_string();
    }

    let payload = serde_json::json!({
        "room_key": p.room_key,
        "confirm": p.confirm
    });

    ctx.call_imm("imm.purge_room", payload).await
}

pub async fn imm_reboot(ctx: &HandlerContext<'_>, params: Parameters<RebootParams>) -> String {
    let p = params.0;
    if !p.confirm {
        return "Error: This is a destructive operation. Set `confirm` to true to proceed."
            .to_string();
    }

    let payload = serde_json::json!({
        "confirm": p.confirm,
        "delay_secs": p.delay_secs
    });

    ctx.call_imm("imm.reboot", payload).await
}
