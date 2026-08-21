use std::env;
use std::path::PathBuf;

fn print_help() {
    println!(
        "OxideMUD Server v{}

USAGE:
    oxide-server [OPTIONS]

OPTIONS:
    -h, --help                 Print help information and exit
    -V, --version              Print version information and exit
    -H, --host <address>       Bind host address [default: 127.0.0.1]
    -p, --port <port>          Bind port [default: 4000]
    -d, --db-path <path>       SQLite database file [default: data/mud.db]
    -C, --content-path <dir>   Content/asset directory [default: content]
    -c, --config-path <path>   Server config TOML [default: content/server.toml]
    -m, --motd-path <path>     Message-of-the-day file [default: content/motd.txt]
    -b, --banner-path <path>   Login banner file [default: content/banner.txt]
        --validate-content     Validate server.toml + content tree, print report, exit",
        env!("CARGO_PKG_VERSION")
    );
}

pub struct Config {
    pub host: String,
    pub port: u16,
    pub db_path: PathBuf,
    pub content_path: Option<PathBuf>,
    pub motd_path: Option<PathBuf>,
    pub banner_path: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    /// Preflight mode: validate server.toml + content tree, print report, exit.
    pub validate_content: bool,
}

impl Config {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();

        if args.iter().any(|arg| arg == "--version" || arg == "-V") {
            println!("OxideMUD Server v{}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        if args
            .iter()
            .any(|arg| arg == "--help" || arg == "-h" || arg == "help")
        {
            print_help();
            std::process::exit(0);
        }

        let mut host = "127.0.0.1".to_string();
        let mut port = 4000u16;
        let mut db_path = PathBuf::from("data/mud.db");
        let mut content_path: Option<PathBuf> = None;
        let mut motd_path = Some(PathBuf::from("content/motd.txt"));
        let mut banner_path = Some(PathBuf::from("content/banner.txt"));

        let mut config_path: Option<PathBuf> = None;
        let mut validate_content = false;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--validate-content" => {
                    validate_content = true;
                }
                "--host" | "-H" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        host = val.clone();
                    }
                }
                "--port" | "-p" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        port = val.parse().unwrap_or(4000);
                    }
                }
                "--db-path" | "-d" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        db_path = PathBuf::from(val);
                    }
                }
                "--content-path" | "-C" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        content_path = Some(PathBuf::from(val));
                    }
                }
                "--motd-path" | "-m" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        motd_path = Some(PathBuf::from(val));
                    }
                }
                "--banner-path" | "-b" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        banner_path = Some(PathBuf::from(val));
                    }
                }
                "--config-path" | "-c" => {
                    i += 1;
                    if let Some(val) = args.get(i) {
                        config_path = Some(PathBuf::from(val));
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Config {
            host,
            port,
            db_path,
            content_path,
            motd_path,
            banner_path,
            config_path,
            validate_content,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
