use crate::app::Mode;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "spade", about = "MUD Game Engine Builder TUI & MUD Client")]
pub struct Config {
    /// Execution mode: offline, online, or split
    #[arg(short, long, value_enum, default_value = "offline")]
    pub mode: Mode,

    /// Connection URL (ws://, wss://, http://, or https://)
    #[arg(long)]
    pub url: Option<String>,

    /// Connection host (optional, defaults to config file value)
    #[arg(long)]
    pub connect_host: Option<String>,

    /// Connection port (optional, defaults to config file value)
    #[arg(long)]
    pub connect_port: Option<u16>,

    /// API key for authenticated server connections
    #[arg(long)]
    pub api_key: Option<String>,

    #[command(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Connect to a MUD server directly via URL or host/port
    Connect {
        /// Connection target (e.g. wss://127.0.0.1:8080/ws/spade or 127.0.0.1)
        target: String,
        /// Connection port (optional if URL or host is specified)
        port: Option<u16>,
    },
}

impl Config {
    pub fn parse() -> Self {
        let mut cli = <Self as Parser>::parse();
        if let Some(SubCommand::Connect { target, port }) = cli.subcommand.take() {
            cli.mode = Mode::Online;
            if target.contains("://") || target.contains('/') {
                cli.url = Some(target);
            } else {
                cli.connect_host = Some(target);
                cli.connect_port = port;
            }
        }
        cli
    }
}
