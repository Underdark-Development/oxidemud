use crate::connection::Connection;
use crate::registry::ConnectionRegistry;
use oxide_core::World;

pub type CommandFn = fn(&mut World, &mut dyn Connection, &str, &str, &ConnectionRegistry);

pub use oxide_core::AccessLevel;

pub struct CommandHelp {
    pub short: &'static str,
    pub body: Option<&'static str>,
}

pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub access: AccessLevel,
    pub topic: &'static str,
    pub help: CommandHelp,
    pub handler: CommandFn,
}

pub struct CommandDispatch {
    pub commands: Vec<Command>,
}

impl CommandDispatch {
    pub fn new() -> Self {
        CommandDispatch {
            commands: Vec::new(),
        }
    }

    pub fn register(&mut self, command: Command) {
        self.commands.push(command);
    }

    pub fn execute(
        &self,
        world: &mut World,
        conn: &mut dyn Connection,
        input: &str,
        registry: &ConnectionRegistry,
    ) {
        if let Some(entity) = conn.entity() {
            if let Ok(mut q) = world.query_one::<&AccessLevel>(entity) {
                if let Some(&level) = q.get() {
                    conn.set_access_level(level);
                }
            }
        }

        let input = input.trim();

        if input.is_empty() {
            return;
        }

        let (name, args) = match input.find(char::is_whitespace) {
            Some(pos) => (&input[..pos], input[pos..].trim()),
            None => (input, ""),
        };

        let mut is_unconscious = false;
        if let Some(entity) = conn.entity() {
            if let Ok(mut q) = world.query_one::<&oxide_core::Health>(entity) {
                if let Some(hp) = q.get() {
                    if hp.current <= 0 {
                        is_unconscious = true;
                    }
                }
            }
            if !is_unconscious {
                if let Ok(mut q) = world.query_one::<&oxide_core::PlayerState>(entity) {
                    if let Some(oxide_core::PlayerState::Resting(
                        oxide_core::RestState::Unconscious,
                    )) = q.get()
                    {
                        is_unconscious = true;
                    }
                }
            }
        }

        if is_unconscious {
            let cmd_name = name.to_lowercase();
            if cmd_name != "quit"
                && cmd_name != "score"
                && cmd_name != "help"
                && cmd_name != "commands"
                && cmd_name != "die"
            {
                conn.send_line("You are unconscious and cannot do that.");
                return;
            }
        }

        if let Some(entity) = conn.entity() {
            if check_entity_command(world, entity, name, args) {
                return;
            }
        }

        let dynamic_skill =
            oxide_core::with_dynamic_skills(|reg| reg.find_direct_command(name).cloned());
        if let Some(skill) = dynamic_skill {
            if let Some(entity) = conn.entity() {
                match check_command_restrictions(world, entity, None, false, &skill.restrictions) {
                    Ok(()) => {
                        if let Some(bridge) = oxide_core::get_scripting_bridge() {
                            let _ = bridge.execute_script_skill(&skill.script, entity, args, world);
                            return;
                        }
                    }
                    Err(msg) => {
                        conn.send_line(&msg);
                        return;
                    }
                }
            }
        }

        if let Some(cmd) = self.find(name) {
            if conn.access_level() < cmd.access {
                conn.send_line("Huh? Type 'help' for a list of commands.");
                return;
            }
            (cmd.handler)(world, conn, name, args, registry);
        } else {
            conn.send_line("Huh? Type 'help' for a list of commands.");
        }
    }

    /// Returns commands grouped by topic in registration order.
    pub fn help_groups(&self) -> Vec<(&'static str, Vec<&Command>)> {
        let mut groups: Vec<(&'static str, Vec<&Command>)> = Vec::new();
        for cmd in &self.commands {
            if let Some(group) = groups.iter_mut().find(|(t, _)| *t == cmd.topic) {
                group.1.push(cmd);
            } else {
                groups.push((cmd.topic, vec![cmd]));
            }
        }
        groups
    }

    pub fn find(&self, name: &str) -> Option<&Command> {
        // 1. Exact alias match — short aliases always take precedence
        if let Some(cmd) = self.commands.iter().find(|cmd| cmd.aliases.contains(&name)) {
            return Some(cmd);
        }
        // 2. Exact name match
        if let Some(cmd) = self.commands.iter().find(|cmd| cmd.name == name) {
            return Some(cmd);
        }
        // 3. Prefix name match
        if let Some(cmd) = self.commands.iter().find(|cmd| cmd.name.starts_with(name)) {
            return Some(cmd);
        }
        // 4. Prefix alias match (lowest priority)
        self.commands
            .iter()
            .find(|cmd| cmd.aliases.iter().any(|a| a.starts_with(name)))
    }
}

impl Default for CommandDispatch {
    fn default() -> Self {
        Self::new()
    }
}

fn check_command_restrictions(
    world: &mut World,
    actor: oxide_core::Entity,
    target_entity: Option<oxide_core::Entity>,
    is_equipped: bool,
    restr: &oxide_core::CommandRestrictions,
) -> Result<(), String> {
    if restr.requires_equipped && !is_equipped {
        return Err(restr
            .rejection_message
            .clone()
            .unwrap_or_else(|| "You must equip that item to use that ability.".to_string()));
    }

    if restr.min_level > 0 {
        let level = world
            .query_one::<&oxide_core::Level>(actor)
            .ok()
            .and_then(|mut q| q.get().map(|l| l.0))
            .unwrap_or(1);
        if level < restr.min_level {
            return Err(restr.rejection_message.clone().unwrap_or_else(|| {
                "You are not experienced enough to use that ability.".to_string()
            }));
        }
    }

    if !restr.allowed_classes.is_empty() {
        let actor_class = world
            .query_one::<&oxide_core::Class>(actor)
            .ok()
            .and_then(|mut q| q.get().map(|c| c.0.clone()));
        let allowed = actor_class.as_ref().is_some_and(|ac| {
            restr
                .allowed_classes
                .iter()
                .any(|c| c.eq_ignore_ascii_case(ac))
        });
        if !allowed {
            return Err(restr.rejection_message.clone().unwrap_or_else(|| {
                let required = restr.allowed_classes.join(" or ");
                format!("Only a {required} can use that ability.")
            }));
        }
    }

    if !restr.allowed_races.is_empty() {
        let actor_race = world
            .query_one::<&oxide_core::Race>(actor)
            .ok()
            .and_then(|mut q| q.get().map(|r| r.0.clone()));
        let allowed = actor_race.as_ref().is_some_and(|ar| {
            restr
                .allowed_races
                .iter()
                .any(|r| r.eq_ignore_ascii_case(ar))
        });
        if !allowed {
            return Err(restr.rejection_message.clone().unwrap_or_else(|| {
                let required = restr.allowed_races.join(" or ");
                format!("Only {required}s possess the ability to perform that.")
            }));
        }
    }

    if let Some(ref req_deity) = restr.required_deity {
        let actor_deity = world
            .query_one::<&oxide_core::Deity>(actor)
            .ok()
            .and_then(|mut q| q.get().cloned())
            .and_then(|d| d.0);
        let allowed = actor_deity
            .as_deref()
            .is_some_and(|ad| ad.eq_ignore_ascii_case(req_deity));
        if !allowed {
            return Err(restr.rejection_message.clone().unwrap_or_else(|| {
                format!("Only devout followers of {req_deity} can invoke that ability.")
            }));
        }
    }

    if let Some(ref pred_script) = restr.script_predicate {
        if let Some(bridge) = oxide_core::get_scripting_bridge() {
            let res = bridge.evaluate_script_predicate(pred_script, actor, target_entity, world);
            if let Ok(false) = res {
                return Err(restr
                    .rejection_message
                    .clone()
                    .unwrap_or_else(|| "You cannot use that ability right now.".to_string()));
            }
        }
    }

    Ok(())
}

fn check_entity_command(
    world: &mut World,
    actor: oxide_core::Entity,
    name: &str,
    args: &str,
) -> bool {
    let bridge = match oxide_core::get_scripting_bridge() {
        Some(b) => b,
        None => return false,
    };

    // 1. Room command
    if let Some(room) = oxide_core::get_pos_room(world, actor) {
        let room_cmd = {
            if let Ok(mut q) = world.query_one::<&oxide_core::EntityCommands>(room) {
                q.get()
                    .and_then(|cmds| cmds.find(name))
                    .map(|c| (c.script.clone(), c.restrictions.clone()))
            } else {
                None
            }
        };
        if let Some((script, restr)) = room_cmd {
            match check_command_restrictions(world, actor, Some(room), false, &restr) {
                Ok(()) => {
                    let _ = bridge.execute_entity_command(room, &script, actor, args, world);
                }
                Err(msg) => {
                    if let Some(msg_bridge) = oxide_core::get_message_bridge() {
                        msg_bridge.send_to_entity(actor, &msg);
                    }
                }
            }
            return true;
        }

        // 2. Room objects / mobs
        let entities = oxide_core::entities_in_room(world, room);
        for target in entities {
            if target == actor {
                continue;
            }
            let target_cmd = {
                if let Ok(mut q) = world.query_one::<&oxide_core::EntityCommands>(target) {
                    q.get()
                        .and_then(|cmds| cmds.find(name))
                        .map(|c| (c.script.clone(), c.restrictions.clone()))
                } else {
                    None
                }
            };
            if let Some((script, restr)) = target_cmd {
                match check_command_restrictions(world, actor, Some(target), false, &restr) {
                    Ok(()) => {
                        let _ = bridge.execute_entity_command(target, &script, actor, args, world);
                    }
                    Err(msg) => {
                        if let Some(msg_bridge) = oxide_core::get_message_bridge() {
                            msg_bridge.send_to_entity(actor, &msg);
                        }
                    }
                }
                return true;
            }
        }
    }

    // 3. Inventory / Equipment items on actor
    let mut actor_items: Vec<(oxide_core::Entity, bool)> = Vec::new();

    if let Ok(mut q) = world.query_one::<&oxide_core::Equipment>(actor) {
        if let Some(eq) = q.get() {
            for (_, e) in &eq.slots {
                actor_items.push((*e, true));
            }
        }
    }
    if let Ok(mut q) = world.query_one::<&oxide_core::Inventory>(actor) {
        if let Some(inv) = q.get() {
            for item in &inv.0 {
                if !actor_items.iter().any(|(e, _)| e == item) {
                    actor_items.push((*item, false));
                }
            }
        }
    }

    for (item_entity, is_equipped) in actor_items {
        let item_cmd = {
            if let Ok(mut q) = world.query_one::<&oxide_core::EntityCommands>(item_entity) {
                q.get()
                    .and_then(|cmds| cmds.find(name))
                    .map(|c| (c.script.clone(), c.restrictions.clone()))
            } else {
                None
            }
        };
        if let Some((script, restr)) = item_cmd {
            match check_command_restrictions(world, actor, Some(item_entity), is_equipped, &restr) {
                Ok(()) => {
                    let _ = bridge.execute_entity_command(item_entity, &script, actor, args, world);
                }
                Err(msg) => {
                    if let Some(msg_bridge) = oxide_core::get_message_bridge() {
                        msg_bridge.send_to_entity(actor, &msg);
                    }
                }
            }
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::TelnetConnection;
    use crate::registry::ConnectionRegistry;
    use oxide_core::World;

    fn noop(
        _world: &mut World,
        _conn: &mut dyn Connection,
        _name: &str,
        _args: &str,
        _registry: &ConnectionRegistry,
    ) {
    }

    fn test_handler(
        _world: &mut World,
        conn: &mut dyn Connection,
        _name: &str,
        args: &str,
        _registry: &ConnectionRegistry,
    ) {
        conn.send_line(&format!("handled: {args}"));
    }

    fn empty_registry() -> ConnectionRegistry {
        ConnectionRegistry::new()
    }

    fn make_dispatch() -> CommandDispatch {
        let mut d = CommandDispatch::new();
        d.register(Command {
            name: "test",
            aliases: &["t"],
            access: AccessLevel::Player,
            topic: "General",
            help: CommandHelp {
                short: "test command",
                body: None,
            },
            handler: test_handler,
        });
        d.register(Command {
            name: "admin",
            aliases: &[],
            access: AccessLevel::Admin,
            topic: "Admin",
            help: CommandHelp {
                short: "admin command",
                body: None,
            },
            handler: noop,
        });
        d
    }

    #[test]
    fn test_command_dispatch_empty_input() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _) = TelnetConnection::new("1".to_string());
        let registry = empty_registry();
        dispatch.execute(&mut world, &mut conn, "", &registry);
        // No crash = pass
    }

    #[test]
    fn test_command_dispatch_unknown() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _rx) = TelnetConnection::new("1".to_string());
        let registry = empty_registry();
        dispatch.execute(&mut world, &mut conn, "bogus", &registry);
        // No crash = pass
    }

    #[test]
    fn test_command_dispatch_find_by_name() {
        let dispatch = make_dispatch();
        assert!(dispatch.find("test").is_some());
        assert!(dispatch.find("admin").is_some());
        assert!(dispatch.find("nope").is_none());
    }

    #[test]
    fn test_command_dispatch_find_by_alias() {
        let dispatch = make_dispatch();
        assert!(dispatch.find("t").is_some());
    }

    #[test]
    fn test_command_dispatch_parse_args() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _rx) = TelnetConnection::new("1".to_string());
        let registry = empty_registry();
        dispatch.execute(&mut world, &mut conn, "test hello world", &registry);
        // "test hello world" -> name="test", args="hello world"
    }

    #[test]
    fn test_command_dispatch_no_args() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _rx) = TelnetConnection::new("1".to_string());
        let registry = empty_registry();
        dispatch.execute(&mut world, &mut conn, "test", &registry);
    }

    #[test]
    fn test_help_groups_preserves_order() {
        let dispatch = make_dispatch();
        let groups = dispatch.help_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "General");
        assert_eq!(groups[1].0, "Admin");
    }

    #[test]
    fn test_command_dispatch_partial_name_match() {
        let dispatch = make_dispatch();
        assert_eq!(dispatch.find("te").map(|c| c.name), Some("test"));
        assert_eq!(dispatch.find("ad").map(|c| c.name), Some("admin"));
        assert!(dispatch.find("xyzzy").is_none());
    }

    #[test]
    fn test_command_dispatch_alias_beats_prefix() {
        let mut d = CommandDispatch::new();
        d.register(Command {
            name: "targeting",
            aliases: &[],
            access: AccessLevel::Player,
            topic: "Test",
            help: CommandHelp {
                short: "",
                body: None,
            },
            handler: noop,
        });
        d.register(Command {
            name: "test",
            aliases: &["t"],
            access: AccessLevel::Player,
            topic: "Test",
            help: CommandHelp {
                short: "",
                body: None,
            },
            handler: noop,
        });
        // "t" should match the exact alias on "test", not the prefix of "targeting"
        assert_eq!(d.find("t").map(|c| c.name), Some("test"));
    }

    #[test]
    fn test_command_dispatch_permission_denied() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, mut rx) = TelnetConnection::new("1".to_string());
        let registry = empty_registry();

        dispatch.execute(&mut world, &mut conn, "admin", &registry);

        if let Ok(bytes) = rx.try_recv() {
            let msg = String::from_utf8_lossy(&bytes);
            assert!(msg.contains("Huh? Type 'help' for a list of commands."));
        } else {
            panic!("Expected output but got none");
        }
    }

    #[test]
    fn test_command_dispatch_permission_granted() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, mut rx) = TelnetConnection::new("1".to_string());
        let registry = empty_registry();

        conn.set_access_level(AccessLevel::Admin);

        dispatch.execute(&mut world, &mut conn, "admin", &registry);

        // Should not output "Huh? Type 'help'..."
        if let Ok(bytes) = rx.try_recv() {
            let msg = String::from_utf8_lossy(&bytes);
            assert!(!msg.contains("Huh? Type 'help' for a list of commands."));
        }
    }

    #[test]
    fn test_command_dispatch_sync_access_level_from_ecs() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let entity = world.spawn((AccessLevel::Admin,));

        let (mut conn, mut rx) = TelnetConnection::new("1".to_string());
        conn.set_entity(entity);
        conn.set_access_level(AccessLevel::Player);

        let registry = empty_registry();

        dispatch.execute(&mut world, &mut conn, "admin", &registry);

        assert_eq!(conn.access_level(), AccessLevel::Admin);

        if let Ok(bytes) = rx.try_recv() {
            let msg = String::from_utf8_lossy(&bytes);
            assert!(!msg.contains("Huh? Type 'help' for a list of commands."));
        }
    }

    #[test]
    fn test_dynamic_skill_and_entity_command_registration() {
        let skill = oxide_core::ScriptSkill {
            id: "parry_skill".to_string(),
            name: "Parry".to_string(),
            short: "Parry an incoming attack".to_string(),
            command: Some("parry".to_string()),
            is_spell: false,
            topic: "Skills".to_string(),
            help_text: "Parry an incoming attack.".to_string(),
            script: "skills/parry.rhai".to_string(),
            restrictions: oxide_core::CommandRestrictions::default(),
        };
        oxide_core::register_dynamic_skill(skill);

        let found_skill =
            oxide_core::with_dynamic_skills(|reg| reg.find_direct_command("parry").cloned());
        assert!(found_skill.is_some());
        assert_eq!(found_skill.unwrap().name, "Parry");

        let spell = oxide_core::ScriptSkill {
            id: "fireball_spell".to_string(),
            name: "Fireball".to_string(),
            short: "Hurl a ball of fire".to_string(),
            command: Some("fireball".to_string()),
            is_spell: true,
            topic: "Spells".to_string(),
            help_text: "Cast a ball of fire.".to_string(),
            script: "spells/fireball.rhai".to_string(),
            restrictions: oxide_core::CommandRestrictions::default(),
        };
        oxide_core::register_dynamic_skill(spell);

        let found_spell =
            oxide_core::with_dynamic_skills(|reg| reg.find_spell("fireball").cloned());
        assert!(found_spell.is_some());
        assert_eq!(found_spell.unwrap().name, "Fireball");

        let mut ec = oxide_core::EntityCommands::new();
        ec.add(
            "pull",
            "scripts/pull_lever.rhai",
            "Pull the mysterious lever.",
        );
        assert!(ec.find("pull").is_some());
        assert_eq!(ec.find("pull").unwrap().script, "scripts/pull_lever.rhai");
    }

    #[test]
    fn test_command_restrictions_evaluation() {
        let mut world = World::new();
        let actor = world.spawn((
            oxide_core::Level(5),
            oxide_core::Class("Mage".to_string()),
            oxide_core::Race("Elf".to_string()),
            oxide_core::Deity(Some("Elona".to_string())),
        ));

        let mut restr = oxide_core::CommandRestrictions::default();
        restr.min_level = 10;
        assert_eq!(
            check_command_restrictions(&mut world, actor, None, false, &restr).unwrap_err(),
            "You are not experienced enough to use that ability."
        );

        restr.min_level = 5;
        restr.allowed_classes = vec!["Fighter".to_string()];
        assert_eq!(
            check_command_restrictions(&mut world, actor, None, false, &restr).unwrap_err(),
            "Only a Fighter can use that ability."
        );

        restr.allowed_classes = vec!["Mage".to_string()];
        restr.allowed_races = vec!["Dwarf".to_string()];
        assert_eq!(
            check_command_restrictions(&mut world, actor, None, false, &restr).unwrap_err(),
            "Only Dwarfs possess the ability to perform that."
        );

        restr.allowed_races = vec!["Elf".to_string()];
        restr.required_deity = Some("Thor".to_string());
        assert_eq!(
            check_command_restrictions(&mut world, actor, None, false, &restr).unwrap_err(),
            "Only devout followers of Thor can invoke that ability."
        );

        restr.required_deity = Some("Elona".to_string());
        restr.requires_equipped = true;
        assert_eq!(
            check_command_restrictions(&mut world, actor, None, false, &restr).unwrap_err(),
            "You must equip that item to use that ability."
        );

        assert!(check_command_restrictions(&mut world, actor, None, true, &restr).is_ok());
    }
}
