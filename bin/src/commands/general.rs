use oxide_core as core;
use oxide_core::{AccessLevel, World};
use oxide_server::{Command, CommandHelp, Connection, ConnectionRegistry, Server};

pub const HELP_HELP: &str = "Usage: help [topic|command]";

pub const HELP_WIDTH: &str = "Usage: width [columns]\n  0 = unlimited (default)";

pub const HELP_PROMPT: &str = r#"Usage: prompt [template|reset]
  %h/%H  HP current/max    %m/%M  Mana current/max
  %s/%S  Stamina curr/max  %e/%E  Energy current/max
  %p/%P  Psi current/max   %x/%X  XP current/to-next
  %l  Level                %n  Character name
  %v  Room name            %V  Room key
  %R  Rest state           %C  Combat target
  %t  Time of day          %w  Weather
  %%  Literal '%'"#;

pub fn register(server: &mut Server) {
    server.register_command(Command {
        name: "motd",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Display the Message of the Day",
            body: None,
        },
        handler: cmd_motd,
    });
    server.register_command(Command {
        name: "help",
        aliases: &["?"],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Display command help",
            body: Some(HELP_HELP),
        },
        handler: cmd_help,
    });
    server.register_command(Command {
        name: "who",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "List online players",
            body: None,
        },
        handler: cmd_who,
    });
    server.register_command(Command {
        name: "quit",
        aliases: &["exit"],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Disconnect from the game server",
            body: None,
        },
        handler: cmd_quit,
    });
    server.register_command(Command {
        name: "width",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Set terminal output line wrap width",
            body: Some(HELP_WIDTH),
        },
        handler: cmd_width,
    });
    server.register_command(Command {
        name: "prompt",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Customize your status prompt line",
            body: Some(HELP_PROMPT),
        },
        handler: cmd_prompt,
    });
    server.register_command(Command {
        name: "time",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Show current in-game time",
            body: None,
        },
        handler: cmd_time,
    });
    server.register_command(Command {
        name: "weather",
        aliases: &[],
        access: AccessLevel::Player,
        topic: "General",
        help: CommandHelp {
            short: "Show current weather conditions",
            body: None,
        },
        handler: cmd_weather,
    });
}

pub fn cmd_motd(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    if let Some(motd) = oxide_server::get_motd() {
        let ansi = conn.flags().has(oxide_server::ConnectionFlag::Ansi);
        let allow_blink = conn.flags().has(oxide_server::ConnectionFlag::Blink);
        let rich = oxide_core::format::parse_tags(&motd);
        conn.send_line("");
        conn.send_line(&rich.render(ansi, allow_blink));
        conn.send_line("");
    }
}

fn format_wide_list(items: &[String], max_width: usize) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }
    let max_len = items.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = max_len + 3;
    let num_cols = (max_width / col_width).max(1);

    let mut lines = Vec::new();
    let mut current_line = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 && i % num_cols == 0 {
            lines.push(current_line);
            current_line = String::new();
        }
        current_line.push_str(&format!("{:<width$}", item, width = col_width));
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

pub fn cmd_help(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let dispatch = match oxide_server::get_commands() {
        Some(d) => d,
        None => {
            conn.send_line("Help is unavailable.");
            return;
        }
    };

    let conn_access = conn.access_level();
    let mut topics = std::collections::BTreeSet::new();
    for cmd in &dispatch.commands {
        if conn_access >= cmd.access {
            topics.insert(cmd.topic.to_string());
        }
    }
    core::with_dynamic_skills(|reg| {
        for t in reg.topics() {
            topics.insert(t);
        }
    });

    let query = args.trim();

    if !query.is_empty() {
        let query_lower = query.to_lowercase();
        let matched_topic = topics
            .iter()
            .find(|t| t.to_lowercase() == query_lower)
            .cloned();
        if let Some(topic) = matched_topic {
            let mut entries: Vec<String> = Vec::new();
            for cmd in &dispatch.commands {
                if cmd.topic == topic && conn_access >= cmd.access {
                    let name_col = if cmd.aliases.is_empty() {
                        cmd.name.to_string()
                    } else {
                        format!("{}, {}", cmd.name, cmd.aliases.join(", "))
                    };
                    entries.push(format!("  {:<16}{}", name_col, cmd.help.short));
                }
            }
            core::with_dynamic_skills(|reg| {
                for skill in reg.skills_for_topic(&topic) {
                    let display_name = if let Some(cmd) = &skill.command {
                        if skill.is_spell {
                            format!("cast {}", cmd)
                        } else {
                            cmd.clone()
                        }
                    } else {
                        skill.name.clone()
                    };
                    entries.push(format!("  {:<16}{}", display_name, skill.short));
                }
            });

            entries.sort();
            conn.send_line(&format!("  {topic} Commands:"));
            conn.send_line("");
            for entry in &entries {
                conn.send_line(entry);
            }
            conn.send_line("");
            return;
        }

        // 1. Check dynamic skills / spells
        let dynamic_skill =
            core::with_dynamic_skills(|reg| reg.find_by_name_or_command(query).cloned());
        if let Some(skill) = dynamic_skill {
            let header = if let Some(cmd) = &skill.command {
                if skill.is_spell {
                    format!("cast {} - {}", cmd, skill.name)
                } else {
                    format!("{} - {}", cmd, skill.name)
                }
            } else {
                skill.name.clone()
            };
            conn.send_line(&header);
            for line in skill.help_text.lines() {
                conn.send_line(line);
            }
            conn.send_line("");
            return;
        }

        // 2. Check contextual entity commands on room / actor
        if let Some(entity) = conn.entity() {
            if let Some(room) = core::get_pos_room(_world, entity) {
                if let Ok(mut q) = _world.query_one::<&core::EntityCommands>(room) {
                    if let Some(cmds) = q.get() {
                        if let Some(cmd) = cmds.find(query) {
                            conn.send_line(&format!("{} (Room Command)", cmd.command_name));
                            for line in cmd.help_text.lines() {
                                conn.send_line(line);
                            }
                            conn.send_line("");
                            return;
                        }
                    }
                }
            }
        }

        // 3. Check static commands
        if let Some(cmd) = dispatch.find(query) {
            if conn_access >= cmd.access {
                let header = if cmd.aliases.is_empty() {
                    cmd.name.to_string()
                } else {
                    format!("{}, {}", cmd.name, cmd.aliases.join(", "))
                };
                conn.send_line(&format!("{} - {}", header, cmd.help.short));
                if let Some(body) = cmd.help.body {
                    for line in body.lines() {
                        conn.send_line(line);
                    }
                }
                conn.send_line("");
                return;
            }
        }

        conn.send_line(&format!("No help found for '{query}'."));
        return;
    }

    let cats: Vec<String> = topics.iter().map(|s| s.to_string()).collect();
    let width = if conn.screen_width() > 0 {
        conn.screen_width() as usize
    } else {
        80
    };
    conn.send_line("  Available Topics:");
    conn.send_line("");
    for line in format_wide_list(&cats, width) {
        conn.send_line(&format!("  {line}"));
    }
    conn.send_line("");
}

pub fn cmd_who(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    registry: &ConnectionRegistry,
) {
    let lines = oxide_server::login::list_who(world, registry);
    for line in &lines {
        conn.send_line(line);
    }
}

pub fn cmd_quit(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    if let Some(entity) = conn.entity() {
        let _ = world.insert(entity, (core::Dirty,));
    }
    conn.send_line("Goodbye!");
    conn.disconnect();
}

pub fn cmd_width(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        let current = conn.screen_width();
        if current == 0 {
            conn.send_line("Screen width: unlimited (0)");
        } else {
            conn.send_line(&format!("Screen width: {current} columns"));
        }
        conn.send_line("Usage: width <columns>  (0 = unlimited)");
        return;
    }

    let width: u16 = match trimmed.parse() {
        Ok(w) => w,
        Err(_) => {
            conn.send_line("Usage: width <columns>  (0 = unlimited)");
            return;
        }
    };

    conn.set_screen_width(width);

    if let Some(entity) = conn.entity() {
        let mut updated = false;
        if let Ok(mut q) = world.query_one::<&mut core::Player>(entity) {
            if let Some(player) = q.get() {
                player.screen_width = width;
                updated = true;
            }
        }
        if updated {
            let _ = world.insert(entity, (core::Dirty,));
        }
    }

    if width == 0 {
        conn.send_line("Screen width set to unlimited.");
    } else {
        conn.send_line(&format!("Screen width set to {width} columns."));
    }
}

pub fn cmd_prompt(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    args: &str,
    _registry: &ConnectionRegistry,
) {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        let msg = conn
            .entity()
            .and_then(|e| {
                world
                    .query_one::<&oxide_core::Player>(e)
                    .ok()
                    .and_then(|mut q| q.get().cloned())
                    .map(|p| match &p.prompt {
                        Some(t) => format!("Current prompt: {t}"),
                        None => format!(
                            "Using default prompt: {}",
                            oxide_server::config::get().default_prompt
                        ),
                    })
            })
            .unwrap_or_else(|| {
                format!(
                    "Using default prompt: {}",
                    oxide_server::config::get().default_prompt
                )
            });
        conn.send_line(&msg);
        conn.send_line("Usage: prompt <template>");
        conn.send_line("See 'help prompt' for available variables.");
        conn.send_line("Type 'prompt reset' to revert to the server default.");
        return;
    }

    if let Some(entity) = conn.entity() {
        let updated = {
            let mut q = match world.query_one::<&mut oxide_core::Player>(entity) {
                Ok(q) => q,
                Err(_) => return conn.send_line("You can't change your prompt right now."),
            };
            match q.get() {
                Some(player) => {
                    if trimmed == "reset" {
                        tracing::debug!(entity = ?entity, old_prompt = ?player.prompt, "cmd_prompt: reset to None");
                        player.prompt = None;
                        true
                    } else {
                        tracing::debug!(entity = ?entity, new_prompt = %trimmed, "cmd_prompt: set custom prompt");
                        player.prompt = Some(trimmed.to_string());
                        true
                    }
                }
                None => false,
            }
        };
        if updated {
            let _ = world.insert(entity, (oxide_core::Dirty,));
            if trimmed == "reset" {
                conn.send_line("Prompt reset to default.");
            } else {
                conn.send_line("Prompt updated.");
            }
            return;
        }
    }
    conn.send_line("You can't change your prompt right now.");
}

pub fn cmd_time(
    world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    let mut q = world.query::<&oxide_core::GameTime>();
    let time_str = q
        .into_iter()
        .next()
        .map(|(_, gt)| gt.format_time_cmd())
        .unwrap_or_else(|| "It is Dawn on the 1st day of Spring, Year 1.".to_string());
    conn.send_line(&time_str);
}

pub fn cmd_weather(
    _world: &mut World,
    conn: &mut dyn Connection,
    _name: &str,
    _args: &str,
    _registry: &ConnectionRegistry,
) {
    conn.send_line("The sky is clear and a gentle breeze blows from the east.");
}
