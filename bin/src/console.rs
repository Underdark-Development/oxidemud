use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::watch;

use oxide_core::{AccessLevel, Dirty, Name, Player, Position, Room};

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
                tracing::info!("Console: shutdown signal received, exited");
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
            "help" => print_help(args),
            "save" => cmd_save(),
            "broadcast" => cmd_broadcast(args).await,
            "account" => cmd_account(&mut reader, args).await,
            "character" => cmd_character(&mut reader, args).await,
            "apikey" => cmd_apikey(args).await,
            "online" | "who" => cmd_online().await,
            "kick" => cmd_kick(args).await,
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

struct ConsoleCommand {
    name: &'static str,
    category: &'static str,
    syntax: &'static str,
    description: &'static str,
}

static CONSOLE_COMMANDS: &[ConsoleCommand] = &[
    ConsoleCommand {
        name: "help",
        category: "General",
        syntax: "help [command|category]",
        description: "Show help details for console commands or categories",
    },
    ConsoleCommand {
        name: "save",
        category: "General",
        syntax: "save",
        description: "Force flush dirty entities to database",
    },
    ConsoleCommand {
        name: "broadcast",
        category: "General",
        syntax: "broadcast <message>",
        description: "Send message to all online players",
    },
    ConsoleCommand {
        name: "online",
        category: "General",
        syntax: "online",
        description: "List all online players and locations",
    },
    ConsoleCommand {
        name: "who",
        category: "General",
        syntax: "who",
        description: "Alias for 'online' command",
    },
    ConsoleCommand {
        name: "kick",
        category: "General",
        syntax: "kick <username_or_character>",
        description: "Kick an online player or character",
    },
    ConsoleCommand {
        name: "shutdown",
        category: "General",
        syntax: "shutdown",
        description: "Gracefully stop the server",
    },
    ConsoleCommand {
        name: "restart",
        category: "General",
        syntax: "restart",
        description: "Gracefully stop (restart not yet implemented)",
    },
    ConsoleCommand {
        name: "account list",
        category: "Account Management",
        syntax: "account list",
        description: "List all registered accounts",
    },
    ConsoleCommand {
        name: "account create",
        category: "Account Management",
        syntax: "account create <user> <pass> [level]",
        description: "Create a new account (play/build/imm/god/admin)",
    },
    ConsoleCommand {
        name: "account info",
        category: "Account Management",
        syntax: "account info <username>",
        description: "Show account details",
    },
    ConsoleCommand {
        name: "account set-access",
        category: "Account Management",
        syntax: "account set-access <username> <level>",
        description: "Set account access level (play/build/imm/god/admin)",
    },
    ConsoleCommand {
        name: "account set-password",
        category: "Account Management",
        syntax: "account set-password <username>",
        description: "Reset account password",
    },
    ConsoleCommand {
        name: "character set",
        category: "Character Management",
        syntax: "character set <char> <field> <value>",
        description: "Modify character field (level, xp, name, race, class)",
    },
    ConsoleCommand {
        name: "apikey generate",
        category: "API Key Management",
        syntax: "apikey generate <u> [desc]",
        description: "Generate a new REST API key for user <u>",
    },
    ConsoleCommand {
        name: "apikey list",
        category: "API Key Management",
        syntax: "apikey list",
        description: "List active REST API keys",
    },
    ConsoleCommand {
        name: "apikey revoke",
        category: "API Key Management",
        syntax: "apikey revoke <k>",
        description: "Revoke/delete API key <k>",
    },
];

fn print_help(query: &str) {
    let query = query.trim();

    // Get unique categories list
    let mut categories = std::collections::BTreeSet::new();
    for cmd in CONSOLE_COMMANDS {
        categories.insert(cmd.category);
    }

    if !query.is_empty() {
        let query_lower = query.to_lowercase();

        // 1. Check for Category Match
        let matched_category = categories.iter().find(|cat| {
            let cat_lower = cat.to_lowercase();
            cat_lower == query_lower
                || cat_lower.starts_with(&query_lower)
                || cat_lower.contains(&query_lower)
        });

        if let Some(cat) = matched_category {
            println!();
            println!("Commands in Category '{}':", cat);
            println!("{}", "-".repeat(60));
            for cmd in CONSOLE_COMMANDS {
                if cmd.category == *cat {
                    println!("  {:<30} - {}", cmd.syntax, cmd.description);
                }
            }
            println!();
            return;
        }

        // 2. Check for Command Match
        let matched_command = CONSOLE_COMMANDS.iter().find(|c| {
            let name_lower = c.name.to_lowercase();
            name_lower == query_lower || name_lower.starts_with(&query_lower)
        });

        if let Some(cmd) = matched_command {
            println!();
            println!("Command:      {}", cmd.name);
            println!("Category:     {}", cmd.category);
            println!("Syntax:       {}", cmd.syntax);
            println!("Description:  {}", cmd.description);
            println!();
            return;
        }

        println!("No help found for '{query}'. Type 'help' to see all commands.");
        return;
    }

    // Print all commands grouped by category
    println!();
    println!("Server Console Commands:");
    println!("Type 'help <command>' or 'help <category>' for specific details (e.g. 'help account' or 'help account set-access').");

    for cat in categories {
        println!("\n[{cat}]");
        for cmd in CONSOLE_COMMANDS {
            if cmd.category == cat {
                println!("  {:<30} - {}", cmd.syntax, cmd.description);
            }
        }
    }
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
    let trimmed = args.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.is_empty() {
        print_account_usage();
        return;
    }

    match parts[0] {
        "list" => {
            cmd_account_list().await;
        }
        "create" => {
            let create_args = trimmed.strip_prefix("create").unwrap_or("").trim();
            cmd_account_create(create_args).await;
        }
        "info" => {
            if parts.len() < 2 {
                println!("Usage: account info <username>");
                return;
            }
            cmd_account_info(parts[1]).await;
        }
        "set-access" => {
            if parts.len() < 3 {
                println!("Usage: account set-access <username> <level>");
                return;
            }
            if !confirm_risky(
                reader,
                &format!("This grants '{}' access to '{}'.", parts[2], parts[1]),
            )
            .await
            {
                return;
            }
            cmd_account_set_access(parts[1], parts[2]).await;
        }
        "set-password" => {
            if parts.len() < 2 {
                println!("Usage: account set-password <username>");
                return;
            }
            if !confirm_risky(
                reader,
                &format!("This resets the password for '{}'.", parts[1]),
            )
            .await
            {
                return;
            }
            cmd_account_set_password(reader, parts[1]).await;
        }
        _ => {
            print_account_usage();
        }
    }
}

fn print_account_usage() {
    println!("Usage:");
    println!("  account list                           List all accounts");
    println!("  account create <user> <pass> [level]   Create a new account");
    println!("  account info <username>                Show account details");
    println!("  account set-access <username> <level>  Set account access tier");
    println!("  account set-password <username>        Reset account password");
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

    let level_lower = level.to_lowercase();
    let normalized_level = match level_lower.as_str() {
        "admin" | "adm" => "admin",
        "god" => "god",
        "immortal" | "imm" => "immortal",
        "builder" | "build" => "builder",
        "player" | "play" => "player",
        _ => {
            println!(
                "Invalid access level '{level}'. Valid: player, builder, immortal, god, admin"
            );
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

    match oxide_data::set_account_access_level(conn.conn(), account.id, normalized_level) {
        Ok(()) => {
            tracing::warn!(
                target: "audit",
                action = "set_access",
                target = username,
                level = normalized_level,
                "Console set access level"
            );
            println!("Access level for '{username}' set to '{normalized_level}'.");
        }
        Err(e) => println!("Database error: {e}"),
    }

    drop(conn);

    if let Some(world_mutex) = oxide_server::get_world() {
        let mut world = world_mutex.lock().await;
        let mut target_entity = None;
        {
            let mut query = world.query::<(&oxide_core::Player, &mut oxide_core::AccessLevel)>();
            for (entity, (player, access_level)) in query.iter() {
                if player.account_id == account.id {
                    let parsed_level = match normalized_level {
                        "admin" => oxide_core::AccessLevel::Admin,
                        "god" => oxide_core::AccessLevel::God,
                        "immortal" => oxide_core::AccessLevel::Immortal,
                        "builder" => oxide_core::AccessLevel::Builder,
                        _ => oxide_core::AccessLevel::Player,
                    };
                    *access_level = parsed_level;
                    target_entity = Some(oxide_core::Entity::from(entity));
                    break;
                }
            }
        }
        if let Some(entity) = target_entity {
            let _ = world.insert(entity, (oxide_core::Dirty,));
            println!("Updated active session access level for online character of '{username}' to '{normalized_level}'.");
        }
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

async fn cmd_apikey(args: &str) {
    let parts: Vec<&str> = args.split_whitespace().collect();
    match parts.as_slice() {
        ["generate", username, ..] => {
            let mut description = None;
            let mut scopes: Vec<&str> = Vec::new();
            let mut expires = None;
            for chunk in parts[2..].chunks(2) {
                if chunk.len() == 2 {
                    match chunk[0] {
                        "--description" | "-d" => description = Some(chunk[1]),
                        "--scope" | "-s" => {
                            scopes.extend(chunk[1].split(','));
                        }
                        "--expires" | "-e" => expires = Some(chunk[1]),
                        _ => {}
                    }
                }
            }
            if scopes.is_empty() {
                scopes = vec!["mcp"];
            }
            cmd_apikey_generate(username, description, &scopes, expires).await;
        }
        ["list"] => cmd_apikey_list().await,
        ["revoke", key] => cmd_apikey_revoke(key).await,
        ["scope", key, "add", scope] => cmd_apikey_scope_add(key, scope).await,
        ["scope", key, "remove", scope] => cmd_apikey_scope_remove(key, scope).await,
        _ => {
            println!("Usage:");
            println!("  apikey generate <username> [options]        Generate a new API key");
            println!("    Options:");
            println!("      --description, -d <text>                Key description");
            println!("      --scope, -s <mcp,spade>                Comma-separated scopes (default: mcp)");
            println!("      --expires, -e <30d|90d|1y>             Expiry duration");
            println!("  apikey list                                 List active API keys");
            println!("  apikey revoke <key>                         Revoke/delete an API key");
            println!(
                "  apikey scope <key> add <scope>              Add a scope to an existing key"
            );
            println!("  apikey scope <key> remove <scope>           Remove a scope from a key");
        }
    }
}

fn parse_expiry_duration(duration: &str) -> Option<String> {
    let now = chrono::Local::now();
    let duration = duration.trim();
    if duration.len() < 2 {
        return None;
    }
    let (num_str, unit) = duration.split_at(duration.len() - 1);
    let num: i64 = num_str.parse().ok()?;
    let dt = match unit {
        "d" => now + chrono::Duration::days(num),
        "w" => now + chrono::Duration::weeks(num),
        "m" => now + chrono::Duration::days(num * 30),
        "y" => now + chrono::Duration::days(num * 365),
        _ => return None,
    };
    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

async fn cmd_apikey_generate(
    username: &str,
    description: Option<&str>,
    scopes: &[&str],
    expires: Option<&str>,
) {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let expires_at = match expires {
        Some(e) => match parse_expiry_duration(e) {
            Some(dt) => Some(dt),
            None => {
                println!("Invalid expiry duration '{e}'. Use formats like 30d, 2w, 3m, 1y.");
                return;
            }
        },
        None => None,
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

    let key = uuid::Uuid::new_v4().to_string();
    let scope_refs: Vec<&str> = scopes.to_vec();
    match oxide_data::insert_api_key(
        conn.conn(),
        &key,
        account.id,
        description,
        expires_at.as_deref(),
        &scope_refs,
    ) {
        Ok(()) => {
            println!("New API key generated successfully:");
            println!("  User:        {}", account.username);
            println!("  Access Tier: {}", account.access_level);
            println!("  Scopes:      {}", scopes.join(", "));
            println!("  Key:         {key}");
            if let Some(desc) = description {
                println!("  Description: {desc}");
            }
            if let Some(exp) = &expires_at {
                println!("  Expires:     {exp}");
            }
            println!("  IMPORTANT: Store this key safely. It will not be shown again.");
        }
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_apikey_list() {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let conn = db.lock().await;
    let mut stmt = match conn.conn().prepare(
        "SELECT k.key, a.username, k.description, k.created_at, k.expires_at
         FROM api_keys k
         JOIN accounts a ON k.account_id = a.id
         ORDER BY k.created_at DESC",
    ) {
        Ok(s) => s,
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    };

    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let username: String = row.get(1)?;
        let desc: Option<String> = row.get(2)?;
        let created_at: String = row.get(3)?;
        let expires_at: Option<String> = row.get(4)?;
        Ok((key, username, desc, created_at, expires_at))
    });

    // Load scopes for each key
    let mut scope_stmt = match conn
        .conn()
        .prepare("SELECT scope FROM api_key_scopes WHERE key = ?1")
    {
        Ok(s) => s,
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    };

    match rows {
        Ok(iter) => {
            println!();
            let header = format!(
                "{:<36} | {:<15} | {:<10} | {:<20} | {:<20} | {}",
                "API Key", "User", "Scopes", "Created At", "Expires", "Description"
            );
            println!("{header}");
            println!("{}", "-".repeat(130));
            for r in iter.flatten() {
                let key_str = &r.0;
                let scopes: Vec<String> = scope_stmt
                    .query_map([key_str], |row| row.get::<_, String>(0))
                    .map(|iter| iter.flatten().collect())
                    .unwrap_or_default();
                let scope_str = scopes.join(",");
                let desc = r.2.unwrap_or_default();
                let expires = r.4.unwrap_or_else(|| "never".to_string());
                println!(
                    "{:<36} | {:<15} | {:<10} | {:<20} | {:<20} | {}",
                    key_str, r.1, scope_str, r.3, expires, desc
                );
            }
            println!();
        }
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_apikey_revoke(key: &str) {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let conn = db.lock().await;
    match oxide_data::revoke_api_key(conn.conn(), key) {
        Ok(0) => println!("API key not found."),
        Ok(n) => println!("Revoked {n} API key(s)."),
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_apikey_scope_add(key: &str, scope: &str) {
    if !matches!(scope, "mcp" | "spade") {
        println!("Invalid scope '{scope}'. Must be 'mcp' or 'spade'.");
        return;
    }
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let conn = db.lock().await;
    match oxide_data::add_api_key_scope(conn.conn(), key, scope) {
        Ok(()) => println!("Added scope '{scope}' to key."),
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_apikey_scope_remove(key: &str, scope: &str) {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let conn = db.lock().await;
    match oxide_data::remove_api_key_scope(conn.conn(), key, scope) {
        Ok(()) => println!("Removed scope '{scope}' from key."),
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_account_list() {
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let conn = db.lock().await;
    let mut stmt = match conn.conn().prepare(
        "SELECT id, username, access_level, created_at, last_login FROM accounts ORDER BY username ASC",
    ) {
        Ok(s) => s,
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    };

    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let username: String = row.get(1)?;
        let access_level: String = row.get(2)?;
        let created_at: String = row.get(3)?;
        let last_login: Option<String> = row.get(4)?;
        Ok((id, username, access_level, created_at, last_login))
    });

    match rows {
        Ok(iter) => {
            println!();
            println!(
                "{:<6} | {:<20} | {:<12} | {:<20} | {:<20}",
                "ID", "Username", "Access Level", "Created At", "Last Login"
            );
            println!("{}", "-".repeat(86));
            let mut count = 0;
            for r in iter.flatten() {
                count += 1;
                let last_login = r.4.unwrap_or_else(|| "never".to_string());
                println!(
                    "{:<6} | {:<20} | {:<12} | {:<20} | {:<20}",
                    r.0, r.1, r.2, r.3, last_login
                );
            }
            println!("\nTotal accounts: {count}");
            println!();
        }
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_account_create(args: &str) {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 3 {
        println!("Usage: account create <username> <password> [access_level]");
        return;
    }

    let username = parts[0];
    let password = parts[1];
    let access_level = parts.get(2).copied().unwrap_or("player");

    let level_lower = access_level.to_lowercase();
    let normalized_level = match level_lower.as_str() {
        "admin" | "adm" => "admin",
        "god" => "god",
        "immortal" | "imm" => "immortal",
        "builder" | "build" => "builder",
        "player" | "play" => "player",
        _ => {
            println!("Invalid access level '{access_level}'. Valid: player, builder, immortal, god, admin");
            return;
        }
    };

    if password.len() < 8 {
        println!("Password must be at least 8 characters.");
        return;
    }

    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let hash = match oxide_data::hash_password(password) {
        Ok(h) => h,
        Err(e) => {
            println!("Password hashing error: {e}");
            return;
        }
    };

    let conn = db.lock().await;
    match oxide_data::get_account_by_username(conn.conn(), username) {
        Ok(Some(_)) => {
            println!("Account '{username}' already exists.");
            return;
        }
        Ok(None) => {}
        Err(e) => {
            println!("Database error: {e}");
            return;
        }
    }

    match oxide_data::create_account(conn.conn(), username, &hash) {
        Ok(account_id) => {
            if normalized_level != "player" {
                if let Err(e) =
                    oxide_data::set_account_access_level(conn.conn(), account_id, normalized_level)
                {
                    println!("Failed to set access level: {e}");
                    return;
                }
            }
            println!(
                "Account '{username}' created successfully with access level '{normalized_level}'."
            );
        }
        Err(e) => println!("Database error: {e}"),
    }
}

async fn cmd_online() {
    let world_mutex = match oxide_server::get_world() {
        Some(w) => w,
        None => {
            println!("World not available.");
            return;
        }
    };
    let db = match oxide_server::get_db() {
        Some(d) => d,
        None => {
            println!("Database not available.");
            return;
        }
    };

    let world = world_mutex.lock().await;
    let conn = db.lock().await;

    let mut query = world.query::<(&Player, &Name, &Position, &AccessLevel)>();
    let mut players = Vec::new();
    for (entity, (player, name, position, access_level)) in query.iter() {
        let entity_wrapped: oxide_core::Entity = entity.into();
        let username = match oxide_data::get_account_by_id(conn.conn(), player.account_id) {
            Ok(Some(a)) => a.username,
            _ => "unknown".to_string(),
        };

        let room_name = if let Ok(mut q) = world.query_one::<&Room>(position.room) {
            if let Some(r) = q.get() {
                r.name.clone()
            } else {
                format!("Entity {:?}", position.room)
            }
        } else {
            format!("Entity {:?}", position.room)
        };

        players.push((
            entity_wrapped,
            username,
            name.as_str().to_string(),
            *access_level,
            room_name,
        ));
    }

    println!();
    if players.is_empty() {
        println!("No players online.");
    } else {
        println!("Connected Players ({}):", players.len());
        println!(
            "{:<10} | {:<20} | {:<20} | {:<12} | {:<25}",
            "Entity ID", "Username", "Character", "Access", "Location"
        );
        println!("{}", "-".repeat(95));
        for p in players {
            println!(
                "{:<10} | {:<20} | {:<20} | {:<12?} | {:<25}",
                p.0.id(),
                p.1,
                p.2,
                p.3,
                p.4
            );
        }
    }
    println!();
}

async fn cmd_kick(args: &str) {
    let target = args.trim();
    if target.is_empty() {
        println!("Usage: kick <username_or_character>");
        return;
    }

    let world_mutex = match oxide_server::get_world() {
        Some(w) => w,
        None => {
            println!("World not available.");
            return;
        }
    };
    let registry_mutex = match oxide_server::get_registry() {
        Some(r) => r,
        None => {
            println!("Connection registry not available.");
            return;
        }
    };

    let mut world = world_mutex.lock().await;
    let registry = registry_mutex.lock().await;

    let mut target_entity: Option<oxide_core::Entity> = None;
    let mut target_char_name = String::new();

    // 1. Search by character name
    {
        let mut query = world.query::<(&Name, &Player)>();
        for (entity, (name, _player)) in query.iter() {
            if name.as_str().eq_ignore_ascii_case(target) {
                target_entity = Some(entity.into());
                target_char_name = name.as_str().to_string();
                break;
            }
        }
    }

    // 2. Search by account username if not found by character name
    if target_entity.is_none() {
        let db = match oxide_server::get_db() {
            Some(d) => d,
            None => {
                println!("Database not available.");
                return;
            }
        };
        let conn = db.lock().await;
        if let Ok(Some(account)) = oxide_data::get_account_by_username(conn.conn(), target) {
            let mut found_entity = None;
            {
                let mut query = world.query::<(&Name, &Player)>();
                for (entity, (name, player)) in query.iter() {
                    if player.account_id == account.id {
                        found_entity = Some((entity.into(), name.as_str().to_string()));
                        break;
                    }
                }
            }
            if let Some((entity, char_name)) = found_entity {
                target_entity = Some(entity);
                target_char_name = char_name;
            }
        }
    }

    if let Some(entity) = target_entity {
        if let Some(tx) = registry.sender(entity) {
            let _ = tx.send(b"\x00\xFFKICK\x00".to_vec());
            println!("Kicked online player/character '{target_char_name}'.");
            let _ = world.insert(entity, (Dirty,));
        } else {
            println!("Character '{target_char_name}' is not connected.");
        }
    } else {
        println!("No online player or character found matching '{target}'.");
    }
}
