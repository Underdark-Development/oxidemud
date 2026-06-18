use crate::connection::Connection;
use crate::registry::ConnectionRegistry;
use mud_core::World;

pub type CommandFn = fn(&mut World, &mut dyn Connection, &str, &str, &ConnectionRegistry);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessLevel {
    Player,
    Builder,
    Immortal,
    God,
    Admin,
}

pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub access: AccessLevel,
    pub category: &'static str,
    pub help_text: &'static str,
    pub handler: CommandFn,
}

pub struct CommandDispatch {
    commands: Vec<Command>,
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
        let input = input.trim();

        if input.is_empty() {
            return;
        }

        let (name, args) = match input.find(char::is_whitespace) {
            Some(pos) => (&input[..pos], input[pos..].trim()),
            None => (input, ""),
        };

        if let Some(cmd) = self.find(name) {
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

    fn find(&self, name: &str) -> Option<&Command> {
        self.commands
            .iter()
            .find(|cmd| cmd.name == name || cmd.aliases.contains(&name))
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
    use mud_core::World;

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
        let (mut conn, _) = TelnetConnection::new(1);
        let registry = empty_registry();
        dispatch.execute(&mut world, &mut conn, "", &registry);
        // No crash = pass
    }

    #[test]
    fn test_command_dispatch_unknown() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _rx) = TelnetConnection::new(1);
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
        let (mut conn, _rx) = TelnetConnection::new(1);
        let registry = empty_registry();
        dispatch.execute(&mut world, &mut conn, "test hello world", &registry);
        // "test hello world" -> name="test", args="hello world"
    }

    #[test]
    fn test_command_dispatch_no_args() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _rx) = TelnetConnection::new(1);
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
}
