use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::watch;

/// Run the server console — reads commands from stdin and dispatches them.
pub async fn run_console(shutdown_tx: watch::Sender<bool>) {
    // Wait until the server is fully initialized
    while oxide_server::get_world().is_none() || oxide_server::get_db().is_none() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::info!("Server console ready. Type 'help' for commands.");

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    let mut shutdown_rx = shutdown_tx.subscribe();

    loop {
        line.clear();

        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => {
                tracing::info!("Console: shutdown signal received, exiting");
                break;
            }
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => break, // EOF
                    Ok(n) if n <= 1 => continue,
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Console read error: {e}");
                        break;
                    }
                }
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (cmd, args) = match trimmed.find(char::is_whitespace) {
            Some(pos) => (&trimmed[..pos], trimmed[pos..].trim()),
            None => (trimmed, ""),
        };

        match cmd {
            "help" => print_help(),
            "save" => cmd_save(),
            "broadcast" => cmd_broadcast(args).await,
            "account" => cmd_account(&mut reader, args).await,
            "character" => cmd_character(&mut reader, args).await,
            "shutdown" => {
                if confirm_destructive(&mut reader, "shutdown").await {
                    tracing::info!("Console: initiating shutdown");
                    let _ = shutdown_tx.send(true);
                    break;
                }
            }
            "restart" => {
                if confirm_destructive(&mut reader, "restart").await {
                    tracing::info!("Console: initiating restart");
                    let _ = shutdown_tx.send(true);
                    break;
                }
            }
            _ => {
                println!("Unknown command: {cmd}. Type 'help' for available commands.");
            }
        }
    }
}

fn print_help() {
    println!();
    println!("Server Console Commands:");
    println!("  help                           Show this help");
    println!("  save                           Force flush dirty entities to database");
    println!("  broadcast <message>            Send message to all online players");
    println!("  account info <username>        Show account details");
    println!(
        "  account set-access <u> <level> Set account tier (player/builder/immortal/god/admin)"
    );
    println!("  account set-password <u>       Reset account password");
    println!(
        "  character set <char> <f> <v>   Modify character field (level, xp, name, race, class)"
    );
    println!("  shutdown                       Gracefully stop the server");
    println!("  restart                        Gracefully stop (restart not yet implemented)");
    println!();
}

fn cmd_save() {
    tracing::info!("Console: save requested");
    println!("Save requested. Dirty entities will be flushed on next persistence tick.");
}

async fn cmd_broadcast(message: &str) {
    if message.is_empty() {
        println!("Usage: broadcast <message>");
        return;
    }
    let count = oxide_server::console_broadcast(message).await;
    if count == 0 {
        println!("No players connected.");
    } else {
        println!("Broadcast sent to {count} player(s).");
    }
    tracing::info!(message, sent = count, "Console broadcast");
}

async fn confirm_destructive(reader: &mut BufReader<tokio::io::Stdin>, action: &str) -> bool {
    println!("[WARNING] This will {action} and disconnect all players. Are you sure? (y/N)");
    let mut line = String::new();
    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
        return false;
    }
    match line.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        _ => {
            println!("Cancelled.");
            false
        }
    }
}

async fn confirm_risky(reader: &mut BufReader<tokio::io::Stdin>, warning: &str) -> bool {
    println!("[WARNING] {warning} Are you sure? (y/N)");
    let mut line = String::new();
    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
        return false;
    }
    match line.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        _ => {
            println!("Cancelled.");
            false
        }
    }
}

async fn cmd_account(reader: &mut BufReader<tokio::io::Stdin>, args: &str) {
    let parts: Vec<&str> = args.splitn(3, char::is_whitespace).collect();
    match parts.as_slice() {
        ["info", username] => cmd_account_info(username).await,
        ["set-access", username, level] => {
            if !confirm_risky(
                reader,
                &format!("This grants '{level}' access to '{username}'."),
            )
            .await
            {
                return;
            }
            cmd_account_set_access(username, level).await;
        }
        ["set-password", username] => {
            if !confirm_risky(
                reader,
                &format!("This resets the password for '{username}'."),
            )
            .await
            {
                return;
            }
            cmd_account_set_password(reader, username).await;
        }
        _ => {
            println!("Usage:");
            println!("  account info <username>");
            println!("  account set-access <username> <level>");
            println!("  account set-password <username>");
        }
    }
}

async fn cmd_account_info(username: &str) {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let conn = db.lock().await;
    let account = match oxide_data::get_account_by_username(conn.conn(), username) {
        Ok(Some(a)) => a,
        Ok(None) => {
            println!("Account '{username}' not found.");
            return;
        }
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    };

    println!();
    println!("Account: {}", account.username);
    println!("  Access level: {}", account.access_level);
    println!("  Created: {}", account.created_at);
    println!(
        "  Last login: {}",
        account.last_login.as_deref().unwrap_or("never")
    );
    println!();
}

async fn cmd_account_set_access(username: &str, level: &str) {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let valid = ["player", "builder", "immortal", "god", "admin"];
    if !valid.contains(&level) {
        println!("Invalid level '{level}'. Valid: {}", valid.join(", "));
        return;
    }

    let conn = db.lock().await;
    let account = match oxide_data::get_account_by_username(conn.conn(), username) {
        Ok(Some(a)) => a,
        Ok(None) => {
            println!("Account '{username}' not found.");
            return;
        }
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    };

    match oxide_data::set_account_access_level(conn.conn(), account.id, level) {
        Ok(()) => {
            tracing::warn!(
                target: "audit",
                action = "set_access",
                target = username,
                level = level,
                "Console set access level"
            );
            println!("Access level for '{username}' set to '{level}'.");
        }
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_account_set_password(reader: &mut BufReader<tokio::io::Stdin>, username: &str) {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let conn = db.lock().await;
    let account = match oxide_data::get_account_by_username(conn.conn(), username) {
        Ok(Some(a)) => a,
        Ok(None) => {
            println!("Account '{username}' not found.");
            return;
        }
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    };
    drop(conn);

    println!("Enter new password for '{username}': ");

    let mut password = String::new();
    if reader.read_line(&mut password).await.unwrap_or(0) == 0 {
        println!("Cancelled.");
        return;
    }
    let password = password.trim();

    if password.len() < 8 {
        println!("Password must be at least 8 characters. Cancelled.");
        return;
    }

    let hash = match oxide_data::hash_password(password) {
        Ok(h) => h,
        Err(e) => {
            println!("Password hashing error: {e}");
            return;
        }
    };

    let conn = db.lock().await;
    match oxide_data::set_account_password_hash(conn.conn(), account.id, &hash) {
        Ok(()) => {
            tracing::warn!(
                target: "audit",
                action = "set_password",
                target = username,
                "Console reset password"
            );
            println!("Password for '{username}' has been reset.");
        }
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_character(reader: &mut BufReader<tokio::io::Stdin>, args: &str) {
    let parts: Vec<&str> = args.splitn(4, char::is_whitespace).collect();
    match parts.as_slice() {
        ["set", char_name, field, value] => {
            let risky = matches!(
                *field,
                "level" | "xp" | "experience" | "name" | "race" | "class"
            );
            if risky
                && !confirm_risky(
                    reader,
                    &format!("This sets '{field}' to '{value}' for '{char_name}'."),
                )
                .await
            {
                return;
            }
            cmd_character_set(char_name, field, value).await;
        }
        _ => {
            println!("Usage:");
            println!("  character set <name> <field> <value>");
            println!("    Fields: level, xp, name, race, class");
        }
    }
}

async fn cmd_character_set(char_name: &str, field: &str, value: &str) {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let db_field = if field == "xp" { "experience" } else { field };

    let conn = db.lock().await;
    let character = match oxide_data::get_character_by_name(conn.conn(), char_name) {
        Ok(Some(c)) => c,
        Ok(None) => {
            println!("Character '{char_name}' not found.");
            return;
        }
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    };

    match oxide_data::set_character_field(conn.conn(), character.id, db_field, value) {
        Ok(()) => {
            tracing::warn!(
                target: "audit",
                action = "set_character",
                target = char_name,
                field = field,
                value = value,
                "Console set character field"
            );
            println!("Character '{char_name}' field '{field}' set to '{value}'.");
        }
        Err(e) => println!("Error: {e}"),
    }
}
