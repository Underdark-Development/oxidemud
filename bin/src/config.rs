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

        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
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
                "--db-path" | "-d" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => db_path = PathBuf::from(val),
                        None => missing_value_arg("--db-path"),
                    }
                }
                "--content-path" | "-C" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => content_path = Some(PathBuf::from(val)),
                        None => missing_value_arg("--content-path"),
                    }
                }
                "--motd-path" | "-m" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => motd_path = Some(PathBuf::from(val)),
                        None => missing_value_arg("--motd-path"),
                    }
                }
                "--banner-path" | "-b" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => banner_path = Some(PathBuf::from(val)),
                        None => missing_value_arg("--banner-path"),
                    }
                }
                "--config-path" | "-c" => {
                    i += 1;
                    match args.get(i) {
                        Some(val) => config_path = Some(PathBuf::from(val)),
                        None => missing_value_arg("--config-path"),
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
