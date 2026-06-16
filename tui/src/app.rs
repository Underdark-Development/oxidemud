use crate::config_file::{PrefsConfig, SpadeConfig};
use crate::screens::world_tree::WorldTreeScreen;
use crate::screens::{PlaceholderScreen, Screen};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use std::io;
use std::path::PathBuf;

type Tui = Terminal<CrosstermBackend<io::Stdout>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Offline,
    Online,
    Split,
}

pub struct App {
    pub mode: Mode,
    pub should_quit: bool,
    pub connection_host: String,
    pub connection_port: u16,
    pub prefs: PrefsConfig,
    pub content_path: PathBuf,
    pub screens: Vec<Box<dyn Screen>>,
    pub active_screen: usize,
}

impl App {
    pub fn new(cli: crate::config::Config, file_config: SpadeConfig) -> Self {
        let host = cli
            .connect_host
            .unwrap_or_else(|| file_config.connection.host.clone());
        let port = cli.connect_port.unwrap_or(file_config.connection.port);
        let content_path = PathBuf::from(file_config.content_path.clone());

        let world_tree = WorldTreeScreen::new(content_path.clone());
        let screens: Vec<Box<dyn Screen>> = vec![
            Box::new(world_tree),
            Box::new(PlaceholderScreen::new("Template Editor")),
            Box::new(PlaceholderScreen::new("Room Graph")),
            Box::new(PlaceholderScreen::new("Entity Inspector")),
            Box::new(PlaceholderScreen::new("Command Palette")),
            Box::new(PlaceholderScreen::new("Live Dashboard")),
            Box::new(PlaceholderScreen::new("Validation Panel")),
            Box::new(PlaceholderScreen::new("File Browser")),
            Box::new(PlaceholderScreen::new("Script Console")),
        ];

        Self {
            mode: cli.mode,
            should_quit: false,
            connection_host: host,
            connection_port: port,
            prefs: file_config.prefs,
            content_path,
            screens,
            active_screen: 0,
        }
    }

    pub fn active_screen(&self) -> &dyn Screen {
        &*self.screens[self.active_screen]
    }

    pub fn active_screen_mut(&mut self) -> &mut dyn Screen {
        &mut *self.screens[self.active_screen]
    }

    pub fn switch_screen(&mut self, idx: usize) {
        if idx < self.screens.len() {
            self.active_screen = idx;
        }
    }

    pub fn reload_content(&mut self) {
        self.screens[0].reload();
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut terminal = init_terminal()?;
        let mut event_loop = crate::event::EventLoop::new()?;

        while !self.should_quit {
            terminal.draw(|f| crate::ui::render(self, f))?;

            if let crate::event::Event::Key(key) = event_loop.next().await? {
                crate::input::handle_key(self, key);
            }
        }

        restore_terminal()?;
        Ok(())
    }
}

fn init_terminal() -> color_eyre::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        ratatui::crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal() -> color_eyre::Result<()> {
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        ratatui::crossterm::event::DisableMouseCapture
    )?;
    disable_raw_mode()?;
    Ok(())
}
