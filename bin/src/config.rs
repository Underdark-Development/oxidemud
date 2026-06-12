use std::env;
use std::path::PathBuf;

pub struct Config {
    pub host: String,
    pub port: u16,
    pub db_path: PathBuf,
}

impl Config {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut host = "127.0.0.1".to_string();
        let mut port = 4000u16;
        let mut db_path = PathBuf::from("data/mud.db");

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--host" | "-h" => {
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
                _ => {}
            }
            i += 1;
        }

        Config {
            host,
            port,
            db_path,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
