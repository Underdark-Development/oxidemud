use crate::app::Mode;

pub struct Config {
    pub mode: Mode,
    pub connect_host: Option<String>,
    pub connect_port: Option<u16>,
}

impl Config {
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut mode = Mode::Offline;
        let mut connect_host = None;
        let mut connect_port = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "online" => mode = Mode::Online,
                "split" => mode = Mode::Split,
                "connect" => {
                    mode = Mode::Online;
                    i += 1;
                    connect_host = args.get(i).cloned();
                    i += 1;
                    connect_port = args.get(i).and_then(|p| p.parse().ok());
                }
                _ => {}
            }
            i += 1;
        }

        Config {
            mode,
            connect_host,
            connect_port,
        }
    }
}
