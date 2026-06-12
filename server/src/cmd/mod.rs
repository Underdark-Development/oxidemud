use crate::connection::Connection;
use mud_core::World;

pub type CommandFn = fn(&mut World, &mut dyn Connection, &str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessLevel {
    Player,
    Builder,
    Admin,
}

pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub access: AccessLevel,
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

    pub fn execute(&self, world: &mut World, conn: &mut dyn Connection, input: &str) {
        let input = input.trim();

        if input.is_empty() {
            return;
        }

        let (name, args) = match input.find(char::is_whitespace) {
            Some(pos) => (&input[..pos], input[pos..].trim()),
            None => (input, ""),
        };

        if let Some(cmd) = self.find(name) {
            (cmd.handler)(world, conn, args);
        } else {
            conn.send_line("Huh? Type 'help' for a list of commands.");
        }
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
    use mud_core::World;

    fn noop(_world: &mut World, _conn: &mut dyn Connection, _args: &str) {}

    fn test_handler(_world: &mut World, conn: &mut dyn Connection, args: &str) {
        conn.send_line(&format!("handled: {args}"));
    }

    fn make_dispatch() -> CommandDispatch {
        let mut d = CommandDispatch::new();
        d.register(Command {
            name: "test",
            aliases: &["t"],
            access: AccessLevel::Player,
            handler: test_handler,
        });
        d.register(Command {
            name: "admin",
            aliases: &[],
            access: AccessLevel::Admin,
            handler: noop,
        });
        d
    }

    #[test]
    fn test_command_dispatch_empty_input() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _) = TelnetConnection::new(1);
        dispatch.execute(&mut world, &mut conn, "");
        // No crash = pass
    }

    #[test]
    fn test_command_dispatch_unknown() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _rx) = TelnetConnection::new(1);
        dispatch.execute(&mut world, &mut conn, "bogus");
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
        dispatch.execute(&mut world, &mut conn, "test hello world");
        // "test hello world" -> name="test", args="hello world"
    }

    #[test]
    fn test_command_dispatch_no_args() {
        let dispatch = make_dispatch();
        let mut world = World::new();
        let (mut conn, _rx) = TelnetConnection::new(1);
        dispatch.execute(&mut world, &mut conn, "test");
    }
}
