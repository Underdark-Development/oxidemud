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
