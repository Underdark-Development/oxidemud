use crate::connection::Connection;
use crate::registry::ConnectionRegistry;
use oxide_core::World;

pub type CommandFn = fn(&mut World, &mut dyn Connection, &str, &str, &ConnectionRegistry);

pub use oxide_core::AccessLevel;

pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub access: AccessLevel,
    pub category: &'static str,
    pub help_text: &'static str,
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

    /// Returns commands grouped by category in registration order.
    pub fn help_groups(&self) -> Vec<(&'static str, Vec<&Command>)> {
        let mut groups: Vec<(&'static str, Vec<&Command>)> = Vec::new();
        for cmd in &self.commands {
            if let Some(group) = groups.iter_mut().find(|(cat, _)| *cat == cmd.category) {
                group.1.push(cmd);
            } else {
                groups.push((cmd.category, vec![cmd]));
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
            category: "General",
            help_text: "test command",
            handler: test_handler,
        });
        d.register(Command {
            name: "admin",
            aliases: &[],
            access: AccessLevel::Admin,
            category: "Admin",
            help_text: "admin command",
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
            category: "Test",
            help_text: "",
            handler: noop,
        });
        d.register(Command {
            name: "test",
            aliases: &["t"],
            access: AccessLevel::Player,
            category: "Test",
            help_text: "",
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
}
