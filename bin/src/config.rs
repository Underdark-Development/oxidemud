use std::env;
use std::path::{Path, PathBuf};

fn print_help() {
    println!(
        "OxideMUD Server v{}

USAGE:
    oxide-server [OPTIONS]

OPTIONS:
    -h, --help                    Print help information and exit
    -V, --version                 Print version information and exit
    -B, --base-dir <dir>          Base/root directory. Config, content, db,
                                  and logs resolve under it.
                                  [default: <current dir>]
    -H, --host <address>          Bind host address [default: 127.0.0.1]
    -p, --port <port>             Bind TCP port [default: 4000]
        --validate-content        Validate content tree + server.toml, print report, exit",
        env!("CARGO_PKG_VERSION")
    );
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Root directory that all server paths resolve against.
    pub base_dir: PathBuf,
    pub host: String,
    pub port: u16,
    /// Preflight mode: validate server.toml + content tree, print report, exit.
    pub validate_content: bool,
}

impl Config {
    // Fixed conventions under base_dir.
    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("content").join("server.toml")
    }
    pub fn content_path(&self) -> PathBuf {
        self.base_dir.join("content")
    }
    pub fn motd_path(&self) -> PathBuf {
        self.base_dir.join("content").join("motd.txt")
    }
    pub fn banner_path(&self) -> PathBuf {
        self.base_dir.join("content").join("banner.txt")
    }
    pub fn scripts_path(&self) -> PathBuf {
        self.base_dir.join("content").join("scripts")
    }
    pub fn db_path(&self) -> PathBuf {
        self.base_dir.join("data").join("mud.db")
    }
    pub fn log_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }

    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();

        if args.iter().any(|arg| arg == "--version" || arg == "-V") {
            println!("OxideMUD Server v{}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            print_help();
            std::process::exit(0);
        }

        // Default base dir = current working directory (dev convenience).
        let mut base_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let mut host = "127.0.0.1".to_string();
        let mut port = 4000u16;
        let mut validate_content = false;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--validate-content" => {
                    validate_content = true;
                }
                "--base-dir" | "-B" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => base_dir = PathBuf::from(val),
                        None => missing_value_arg("--base-dir"),
                    }
                }
                "--host" | "-H" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => host = val.clone(),
                        None => missing_value_arg("--host"),
                    }
                }
                "--port" | "-p" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => match val.parse::<u16>() {
                            Ok(p) => port = p,
                            Err(_) => invalid_value_arg("--port", val),
                        },
                        None => missing_value_arg("--port"),
                    }
                }
                other => {
                    eprintln!("Unknown argument: {other}");
                    print_help();
                    std::process::exit(2);
                }
            }
            i += 1;
        }

        Config {
            base_dir,
            host,
            port,
            validate_content,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn missing_value_arg(flag: &str) -> ! {
    eprintln!("Error: '{}' requires a value.", flag);
    print_help();
    std::process::exit(2);
}

fn invalid_value_arg(flag: &str, value: &str) -> ! {
    eprintln!("Error: invalid value '{value}' for '{}'.", flag);
    print_help();
    std::process::exit(2);
}
