//! Online-only immortal/admin handler implementations.

use rmcp::handler::server::wrapper::Parameters;

use crate::context::HandlerContext;
use crate::params::*;

pub async fn list_connected_players(ctx: &HandlerContext<'_>) -> String {
    let resp = match ctx
        .authenticated_request(reqwest::Method::GET, "/api/players".to_string())
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    if resp.status().is_success() {
        if let Ok(players) = resp.json::<Vec<serde_json::Value>>().await {
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
    } else {
        format!("Server returned error status: {}", resp.status())
    }
}

pub async fn imm_put_item(ctx: &HandlerContext<'_>, params: Parameters<PutItemParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "item_template_id": p.item_template_id,
        "count": p.count
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/put_item".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse MUD Server response as JSON: {e}"),
        }
    } else {
        match resp.text().await {
            Ok(err_text) => format!("Error from server: {err_text}"),
            Err(_) => format!("Server returned error status: {}", status),
        }
    }
}

pub async fn imm_teleport(ctx: &HandlerContext<'_>, params: Parameters<TeleportParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "room_key": p.room_key
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/teleport".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse MUD Server response as JSON: {e}"),
        }
    } else {
        match resp.text().await {
            Ok(err_text) => format!("Error from server: {err_text}"),
            Err(_) => format!("Server returned error status: {}", status),
        }
    }
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

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/force_command".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse MUD Server response as JSON: {e}"),
        }
    } else {
        match resp.text().await {
            Ok(err_text) => format!("Error from server: {err_text}"),
            Err(_) => format!("Server returned error status: {}", status),
        }
    }
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

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/set_stat".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_load_mob(ctx: &HandlerContext<'_>, params: Parameters<LoadMobParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "room_key": p.room_key,
        "mob_template_id": p.mob_template_id
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/load_mob".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_load_item(ctx: &HandlerContext<'_>, params: Parameters<LoadItemParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "room_key": p.room_key,
        "item_template_id": p.item_template_id,
        "count": p.count
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/load_item".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_gecho(ctx: &HandlerContext<'_>, params: Parameters<GechoParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "message": p.message
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/gecho".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_advance(ctx: &HandlerContext<'_>, params: Parameters<AdvanceParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "player_name": p.player_name,
        "target_level": p.target_level
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/advance".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_stat(ctx: &HandlerContext<'_>, params: Parameters<StatParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/stat".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.text().await {
            Ok(t) => t,
            Err(e) => format!("Failed to read response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_heal(ctx: &HandlerContext<'_>, params: Parameters<HealParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/heal".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_damage(ctx: &HandlerContext<'_>, params: Parameters<DamageParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name,
        "amount": p.amount
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/damage".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
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

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/kill".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}

pub async fn imm_revive(ctx: &HandlerContext<'_>, params: Parameters<ReviveParams>) -> String {
    let p = params.0;
    let payload = serde_json::json!({
        "target_name": p.target_name
    });

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/revive".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
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

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/set_alignment".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
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

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/set_faction".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
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

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/purge_room".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
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

    let resp = match ctx
        .authenticated_request_with_body(
            reqwest::Method::POST,
            "/api/imm/reboot".to_string(),
            Some(&payload),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return e,
    };
    let status = resp.status();
    if status.is_success() {
        match resp.json::<serde_json::Value>().await {
            Ok(res) => res
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string(),
            Err(e) => format!("Failed to parse response: {e}"),
        }
    } else {
        resp.text()
            .await
            .unwrap_or_else(|_| format!("Error status: {status}"))
    }
}
