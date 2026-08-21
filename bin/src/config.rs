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
        --validate-content        Validate content tree + server.toml, print report, exit

Values may be passed as a following argument (--base-dir /path) or with an
equals sign (--base-dir=/path).",
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
        self.base_dir.join("server.toml")
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

        // Default base dir = current working directory (dev convenience).
        let mut base_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let mut host = "127.0.0.1".to_string();
        let mut port = 4000u16;
        let mut validate_content = false;

        let mut i = 1;
        while i < args.len() {
            let arg = &args[i];

            // Split `--long=value` into flag + inline value. Only long options
            // (double-dash) use the `=` form; short options use `-X value`.
            let (flag, inline_value) = match arg.split_once('=') {
                Some((f, v)) if f.starts_with("--") && !v.is_empty() => {
                    (f.to_string(), Some(v.to_string()))
                }
                _ => (arg.clone(), None),
            };

            // Resolve the value for a value-taking flag: inline (`=v`) wins,
            // otherwise the next positional argument is consumed as the value.
            // A next argument that is itself a flag is treated as "missing value".
            macro_rules! take_value {
                () => {
                    match inline_value {
                        Some(v) => {
                            i += 1;
                            v
                        }
                        None => {
                            // Only consume the next arg if it looks like a value
                            // (does not start with `-`). This prevents a flag from
                            // being swallowed as another flag's value.
                            match args.get(i + 1) {
                                Some(next) if !next.starts_with('-') => {
                                    i += 2;
                                    next.clone()
                                }
                                _ => missing_value_arg(&flag),
                            }
                        }
                    }
                };
            }

            match flag.as_str() {
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                "-V" | "--version" => {
                    println!("OxideMUD Server v{}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "--validate-content" => {
                    validate_content = true;
                    i += 1;
                }
                "-B" | "--base-dir" => {
                    let v = take_value!();
                    base_dir = PathBuf::from(v);
                }
                "-H" | "--host" => {
                    let v = take_value!();
                    host = v;
                }
                "-p" | "--port" => {
                    let v = take_value!();
                    match v.parse::<u16>() {
                        Ok(p) => port = p,
                        Err(_) => invalid_value_arg(&flag, &v),
                    }
                }
                other => {
                    eprintln!("Unknown argument: {other}");
                    print_help();
                    std::process::exit(2);
                }
            }
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
