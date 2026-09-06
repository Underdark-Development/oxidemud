use crate::app::Mode;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "spade", about = "MUD Game Engine Builder TUI & MUD Client")]
pub struct Config {
    /// Execution mode: offline, online, or split (defaults to online if URL/host/api-key provided, else offline)
    #[arg(short, long, value_enum)]
    pub mode: Option<Mode>,

    /// Connection URL (ws://, wss://, http://, or https://)
    #[arg(short = 'u', long)]
    pub url: Option<String>,

    /// Connection host (optional, defaults to config file value)
    #[arg(short = 'H', long = "host", alias = "connect-host")]
    pub connect_host: Option<String>,

    /// Connection port (optional, defaults to config file value)
    #[arg(short = 'p', long = "port", alias = "connect-port")]
    pub connect_port: Option<u16>,

    /// API key for authenticated server connections
    #[arg(short = 'k', long = "api-key", alias = "key")]
    pub api_key: Option<String>,

    /// Launch into the design-system gallery (prototype build)
    #[arg(long, hide = true)]
    pub prototype: bool,

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
            cli.mode = Some(Mode::Online);
            if target.contains("://") || target.contains('/') {
                cli.url = Some(target);
            } else {
                cli.connect_host = Some(target);
                cli.connect_port = port;
            }
        }
        if cli.mode.is_none() {
            if cli.url.is_some() || cli.connect_host.is_some() || cli.api_key.is_some() {
                cli.mode = Some(Mode::Online);
            } else {
                cli.mode = Some(Mode::Offline);
            }
        }
        cli
    }

    pub fn mode(&self) -> Mode {
        self.mode.unwrap_or(Mode::Offline)
    }

    /// Whether to launch into the design-system prototype gallery.
    pub fn prototype(&self) -> bool {
        self.prototype
    }

    pub fn parse_from_args<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let mut cli = <Self as Parser>::parse_from(itr);
        if let Some(SubCommand::Connect { target, port }) = cli.subcommand.take() {
            cli.mode = Some(Mode::Online);
            if target.contains("://") || target.contains('/') {
                cli.url = Some(target);
            } else {
                cli.connect_host = Some(target);
                cli.connect_port = port;
            }
        }
        if cli.mode.is_none() {
            if cli.url.is_some() || cli.connect_host.is_some() || cli.api_key.is_some() {
                cli.mode = Some(Mode::Online);
            } else {
                cli.mode = Some(Mode::Offline);
            }
        }
        cli
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_offline() {
        let config = Config::parse_from_args(["spade"]);
        assert_eq!(config.mode(), Mode::Offline);
        assert_eq!(config.url, None);
        assert_eq!(config.api_key, None);
    }

    #[test]
    fn test_mode_inferred_online_when_url_provided() {
        let config = Config::parse_from_args(["spade", "--url", "ws://127.0.0.1:8080/ws/spade"]);
        assert_eq!(config.mode(), Mode::Online);
        assert_eq!(config.url.as_deref(), Some("ws://127.0.0.1:8080/ws/spade"));
    }

    #[test]
    fn test_mode_inferred_online_when_api_key_or_host_provided() {
        let config = Config::parse_from_args([
            "spade",
            "--host",
            "example.com",
            "-p",
            "9000",
            "-k",
            "secret-key",
        ]);
        assert_eq!(config.mode(), Mode::Online);
        assert_eq!(config.connect_host.as_deref(), Some("example.com"));
        assert_eq!(config.connect_port, Some(9000));
        assert_eq!(config.api_key.as_deref(), Some("secret-key"));
    }

    #[test]
    fn test_subcommand_connect_infers_online() {
        let config = Config::parse_from_args(["spade", "connect", "192.168.1.100", "8080"]);
        assert_eq!(config.mode(), Mode::Online);
        assert_eq!(config.connect_host.as_deref(), Some("192.168.1.100"));
        assert_eq!(config.connect_port, Some(8080));
    }
}
