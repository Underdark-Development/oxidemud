use oxide_core as core;
use oxide_core::{AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

const MAX_ALIAS_NAME_LEN: usize = 32;
const MAX_ALIAS_VALUE_LEN: usize = 256;

const HELP_ALIAS: &str = r#"Usage: alias [name] [command]
  alias                    List all aliases
  alias <name>             Show the value of one alias
  alias <name> <command>   Define a command shortcut
  unalias <name>           Remove an alias

Alias names are case-insensitive and args are appended to the command.
Example: 'alias gc gtell' then 'gc hello' runs 'gtell hello'.
The alias and unalias commands themselves cannot be aliased."#;

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "alias",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Define command shortcuts",
            body: Some(HELP_ALIAS),
        },
        handler: cmd_alias,
    });
    server.register_command(Command {
        name: "unalias",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Remove a command shortcut",
            body: None,
        },
        handler: cmd_unalias,
    });
}

pub fn cmd_alias(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let trimmed = args.trim();

    if trimmed.is_empty() {
        return list_aliases(world, conn);
    }

    let (alias_name, alias_value) = match trimmed.find(char::is_whitespace) {
        Some(pos) => (&trimmed[..pos], trimmed[pos..].trim()),
        None => (trimmed, ""),
    };

    // `alias <name>` — show the current value
    if alias_value.is_empty() {
        return show_alias(world, conn, alias_name);
    }

    // `alias <name> <value>` — set the alias
    if let Some(entity) = conn.entity() {
        if let Err(msg) = validate_alias(alias_name, alias_value) {
            return conn.send_line(&msg);
        }

        let old_value = {
            let mut q = match world.query_one::<&mut core::Aliases>(entity) {
                Ok(q) => q,
                Err(_) => return conn.send_line("You can't manage aliases right now."),
            };
            match q.get() {
                Some(aliases) => aliases
                    .0
                    .insert(alias_name.to_ascii_lowercase(), alias_value.to_string()),
                None => return conn.send_line("You can't manage aliases right now."),
            }
        };
        let _ = world.insert(entity, (core::Dirty,));
        if let Some(old) = old_value {
            conn.send_line(&format!(
                "Alias '{}' reassigned from '{}' to '{}'.",
                alias_name, old, alias_value
            ));
        } else {
            conn.send_line(&format!("Alias '{}' set to '{}'.", alias_name, alias_value));
        }
    } else {
        conn.send_line("You can't manage aliases right now.");
    }
}

pub fn cmd_unalias(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let alias_name = args.trim();
    if alias_name.is_empty() {
        conn.send_line("Usage: unalias <name>");
        return;
    }

    if let Some(entity) = conn.entity() {
        let removed = {
            let mut q = match world.query_one::<&mut core::Aliases>(entity) {
                Ok(q) => q,
                Err(_) => return conn.send_line("You can't manage aliases right now."),
            };
            match q.get() {
                Some(aliases) => aliases.remove(alias_name),
                None => return conn.send_line("You can't manage aliases right now."),
            }
        };
        if removed.is_some() {
            let _ = world.insert(entity, (core::Dirty,));
            conn.send_line(&format!("Alias '{}' removed.", alias_name));
        } else {
            conn.send_line(&format!("No alias named '{}' is defined.", alias_name));
        }
    } else {
        conn.send_line("You can't manage aliases right now.");
    }
}

fn list_aliases(world: &mut World, conn: &mut dyn Connection) {
    let Some(entity) = conn.entity() else {
        return conn.send_line("You can't manage aliases right now.");
    };
    let mut aliases = match world.query_one::<&core::Aliases>(entity) {
        Ok(q) => q,
        Err(_) => return conn.send_line("You can't manage aliases right now."),
    };
    let Some(aliases) = aliases.get() else {
        return conn.send_line("You can't manage aliases right now.");
    };
    if aliases.0.is_empty() {
        return conn.send_line("No aliases defined. Type 'alias <name> <command>' to add one.");
    }
    conn.send_line("Your aliases:");
    let mut entries: Vec<(&String, &String)> = aliases.0.iter().collect();
    entries.sort_by_key(|(a, _)| *a);
    for (name, value) in entries {
        conn.send_line(&format!("  {name:<16} {value}"));
    }
}

fn show_alias(world: &mut World, conn: &mut dyn Connection, alias_name: &str) {
    let Some(entity) = conn.entity() else {
        return conn.send_line("You can't manage aliases right now.");
    };
    let mut aliases = match world.query_one::<&core::Aliases>(entity) {
        Ok(q) => q,
        Err(_) => return conn.send_line("You can't manage aliases right now."),
    };
    let Some(aliases) = aliases.get() else {
        return conn.send_line("You can't manage aliases right now.");
    };
    let lower = alias_name.to_ascii_lowercase();
    match aliases.0.get(&lower) {
        Some(value) => conn.send_line(&format!("Alias '{}' = '{}'", alias_name, value)),
        None => conn.send_line(&format!("No alias named '{}' is defined.", alias_name)),
    }
}

fn validate_alias(alias_name: &str, alias_value: &str) -> Result<(), String> {
    let lower = alias_name.to_ascii_lowercase();
    if lower == "alias" || lower == "unalias" {
        return Err("The 'alias' and 'unalias' commands cannot be aliased.".to_string());
    }
    if alias_name.contains(char::is_whitespace) {
        return Err("Alias names cannot contain spaces.".to_string());
    }
    if alias_name.len() > MAX_ALIAS_NAME_LEN {
        return Err(format!(
            "Alias names are limited to {MAX_ALIAS_NAME_LEN} characters."
        ));
    }
    if alias_value.trim().is_empty() {
        return Err("Alias command cannot be empty.".to_string());
    }
    if alias_value.len() > MAX_ALIAS_VALUE_LEN {
        return Err(format!(
            "Alias commands are limited to {MAX_ALIAS_VALUE_LEN} characters."
        ));
    }
    let value_cmd = alias_value.split_whitespace().next().unwrap_or("");
    if value_cmd.eq_ignore_ascii_case(alias_name) {
        return Err("Aliases cannot refer to themselves.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxide_core::World;

    #[test]
    fn test_validate_alias_ok() {
        assert!(validate_alias("gc", "gtell").is_ok());
        assert!(validate_alias("gob", "get orb").is_ok());
    }

    #[test]
    fn test_validate_alias_protected() {
        assert!(validate_alias("alias", "foo").is_err());
        assert!(validate_alias("ALIAS", "foo").is_err());
        assert!(validate_alias("unalias", "foo").is_err());
        assert!(validate_alias("UNALIAS", "foo").is_err());
    }

    #[test]
    fn test_validate_alias_self_reference() {
        assert!(validate_alias("gc", "gc").is_err());
        assert!(validate_alias("gc", "GC tell").is_err());
    }

    #[test]
    fn test_validate_alias_empty_value() {
        assert!(validate_alias("gc", " ").is_err());
    }

    #[test]
    fn test_validate_alias_lengths() {
        let long_name = "x".repeat(MAX_ALIAS_NAME_LEN + 1);
        assert!(validate_alias(&long_name, "look").is_err());
        let long_value = "y".repeat(MAX_ALIAS_VALUE_LEN + 1);
        assert!(validate_alias("gc", &long_value).is_err());
    }

    #[test]
    fn test_validate_alias_spaces() {
        assert!(validate_alias("my alias", "look").is_err());
    }

    #[test]
    fn test_list_aliases_empty() {
        let mut world = World::new();
        let entity = world.spawn((core::Aliases::default(),));
        let (mut conn, mut rx) = oxide_server::TelnetConnection::new("1".to_string());
        conn.set_entity(entity);
        list_aliases(&mut world, &mut conn);
        if let Ok(bytes) = rx.try_recv() {
            let msg = String::from_utf8_lossy(&bytes);
            assert!(msg.contains("No aliases defined"));
        } else {
            panic!("Expected output");
        }
    }
}
