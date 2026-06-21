use crate::app::Mode;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "spade", about = "MUD Game Engine Builder TUI & MUD Client")]
pub struct Config {
    /// Execution mode: offline, online, or split
    #[arg(short, long, value_enum, default_value = "offline")]
    pub mode: Mode,

    /// Connection host (optional, defaults to config file value)
    #[arg(long)]
    pub connect_host: Option<String>,

    /// Connection port (optional, defaults to config file value)
    #[arg(long)]
    pub connect_port: Option<u16>,

    #[command(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Connect to a MUD server directly
    Connect {
        /// Connection host
        host: String,
        /// Connection port
        port: u16,
    },
}

impl Config {
    pub fn parse() -> Self {
        let mut cli = <Self as Parser>::parse();
        if let Some(SubCommand::Connect { host, port }) = cli.subcommand.take() {
            cli.mode = Mode::Online;
            cli.connect_host = Some(host);
            cli.connect_port = Some(port);
        }
        cli
    }
}
