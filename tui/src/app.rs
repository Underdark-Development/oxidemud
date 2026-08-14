use crate::components::command_palette::CommandPalette;
use crate::components::command_sidebar::CommandSidebar;
use crate::components::menu_bar::MenuBar;
use crate::components::CommandAction;
use crate::config_file::{PrefsConfig, SpadeConfig};
use crate::content::{self, FileMap};
use crate::screens::entities::EntitiesScreen;
use crate::screens::file_browser::FileBrowserScreen;
use crate::screens::live_dashboard::LiveDashboardScreen;
use crate::screens::room_grid::RoomGridScreen;
use crate::screens::script_console::ScriptConsoleScreen;
use crate::screens::validation_panel::ValidationPanelScreen;
use crate::screens::{Screen, ScreenId};
use oxide_core::templates::TemplateRegistry;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Layout, Rect},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::time::Instant;

type Tui = Terminal<CrosstermBackend<io::Stdout>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    Offline,
    Online,
    Split,
}

pub struct App {
    pub mode: Mode,
    pub should_quit: bool,
    pub mouse_pos: Option<(u16, u16)>,
    pub status_message: Option<(String, Instant)>,
    pub connection_host: String,
    pub connection_port: u16,
    pub prefs: PrefsConfig,
    pub content_path: PathBuf,
    pub screens: Vec<Box<dyn Screen>>,
    pub active_screen: ScreenId,
    pub registry: TemplateRegistry,
    pub file_map: FileMap,
    pub command_sidebar: CommandSidebar,
    pub menu_bar: MenuBar,
    pub sidebar_visible: bool,
    pub sidebar_focused: bool,
    pub command_palette_open: bool,
    pub command_palette: CommandPalette,
    pub quit_dialog: Option<crate::components::Dialog>,
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

impl App {
    pub fn new(cli: crate::config::Config, file_config: SpadeConfig) -> Self {
        let host = cli
            .connect_host
            .unwrap_or_else(|| file_config.connection.host.clone());
        let port = cli.connect_port.unwrap_or(file_config.connection.port);
        let content_path = PathBuf::from(file_config.content_path.clone());

        let (registry, file_map) = content::load_templates(&content_path);
        let entities =
            EntitiesScreen::new_shared(content_path.clone(), registry.clone(), file_map.clone());
        let room_grid =
            RoomGridScreen::new(content_path.clone(), registry.clone(), file_map.clone());
        let file_browser = FileBrowserScreen::new(content_path.clone());
        let script_console = ScriptConsoleScreen::new();
        let live_dashboard = LiveDashboardScreen::new();

        let screens: Vec<Box<dyn Screen>> = vec![
            Box::new(entities),
            Box::new(room_grid),
            Box::new(ValidationPanelScreen::new(registry.clone())),
            Box::new(file_browser),
            Box::new(script_console),
            Box::new(live_dashboard),
        ];

        Self {
            mode: cli.mode,
            should_quit: false,
            mouse_pos: None,
            status_message: None,
            connection_host: host,
            connection_port: port,
            sidebar_visible: file_config.prefs.sidebar_open,
            prefs: file_config.prefs,
            content_path,
            screens,
            active_screen: ScreenId::Entities,
            registry,
            file_map,
            command_sidebar: CommandSidebar::new(),
            menu_bar: MenuBar::new(),
            sidebar_focused: false,
            command_palette_open: false,
            command_palette: CommandPalette::new(),
            quit_dialog: None,
        }
    }

    pub fn confirm_quit(&mut self) {
        let total_unsaved: usize = self.screens.iter().map(|s| s.unsaved_count()).sum();
        if total_unsaved > 0 {
            self.quit_dialog = Some(crate::components::Dialog::new(
                ratatui::style::Color::Red,
                " Unsaved Edits Warning ",
                "You have unsaved changes across entities! Are you sure you want to quit?",
                &[
                    "Cancel".into(),
                    "Save & Quit".into(),
                    "Discard & Quit".into(),
                ],
            ));
        } else {
            self.should_quit = true;
        }
    }

    pub fn active_screen(&self) -> &dyn Screen {
        &*self.screens[self.active_screen.as_index()]
    }

    pub fn active_screen_mut(&mut self) -> &mut dyn Screen {
        &mut *self.screens[self.active_screen.as_index()]
    }

    pub fn switch_screen(&mut self, id: ScreenId) {
        self.active_screen = id;
        let entities_idx = ScreenId::Entities.as_index();
        if let Some(registry) = self.screens[entities_idx].registry() {
            let registry = registry.clone();
            self.registry = registry.clone();
            let idx = id.as_index();
            if idx < self.screens.len() {
                self.screens[idx].update_registry(&registry);
                self.screens[idx].reload();
            }
        }
    }

    pub fn reload_content(&mut self) {
        let entities_idx = ScreenId::Entities.as_index();
        self.screens[entities_idx].reload();
        if let Some(registry) = self.screens[entities_idx].registry() {
            let registry = registry.clone();
            self.registry = registry.clone();
            for i in 1..self.screens.len() {
                self.screens[i].update_registry(&registry);
            }
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn clear_hover(&mut self) {
        self.mouse_pos = None;
    }

    pub fn handle_command_action(&mut self, action: CommandAction) {
        match action {
            CommandAction::ValidateContent => {
                self.set_status("Opening Validation Panel");
                self.switch_screen(ScreenId::Validation);
            }
            CommandAction::Quit => {
                self.confirm_quit();
            }
            CommandAction::SwitchScreen(idx) => {
                if let Some(id) = ScreenId::from_index(idx) {
                    self.switch_screen(id);
                    self.set_status(format!("Switched to {}", id.name()));
                }
            }
            CommandAction::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
                if self.sidebar_visible {
                    self.sidebar_focused = true;
                }
                self.set_status(if self.sidebar_visible {
                    "Sidebar shown"
                } else {
                    "Sidebar hidden"
                });
            }
            CommandAction::ShowAbout => {
                self.set_status("MUD Game Engine — spade v0.1.0");
            }
            ref action => {
                let active = self.active_screen;
                match self.screens[active.as_index()].handle_command_action(action) {
                    Ok(true) => {
                        if let Some(registry) =
                            self.screens[ScreenId::Entities.as_index()].registry()
                        {
                            let registry = registry.clone();
                            self.registry = registry.clone();
                            for i in 1..self.screens.len() {
                                self.screens[i].update_registry(&registry);
                            }
                        }

                        let msg = match action {
                            CommandAction::CreateEntity(cat) => format!("Created new {cat}"),
                            CommandAction::SaveEntity => "Entity saved".into(),
                            CommandAction::SaveAllEntities => "All entities saved".into(),
                            CommandAction::EditEntity => "Editing entity".into(),
                            CommandAction::DeleteEntity => "Deleting entity...".into(),
                            CommandAction::LookRoom => "Showing room preview".into(),
                            CommandAction::LookMobRoom => "Showing mob preview".into(),
                            CommandAction::LookMobDetail => "Showing mob detail".into(),
                            CommandAction::LookItem => "Showing item preview".into(),
                            CommandAction::GoToParent => "Navigated to parent".into(),
                            CommandAction::ExpandAll => "Expanded all nodes".into(),
                            CommandAction::CollapseAll => "Collapsed all nodes".into(),
                            CommandAction::ToggleSearch => "Search mode activated".into(),
                            CommandAction::ReloadContent => "Content reloaded".into(),
                            CommandAction::ToggleHelp => "Help toggled".into(),
                            _ => String::new(),
                        };
                        if !msg.is_empty() {
                            self.set_status(msg);
                        }
                    }
                    Ok(false) => {
                        self.set_status(format!("Not yet implemented: {action:?}"));
                    }
                    Err(e) => {
                        self.set_status(format!("Error: {e}"));
                    }
                }
            }
        }
    }

    fn handle_action(&mut self) {
        // Clear stale status messages after 5 seconds
        if let Some((_, ts)) = self.status_message {
            if ts.elapsed() > std::time::Duration::from_secs(5) {
                self.status_message = None;
            }
        }

        let action = self.screens[self.active_screen.as_index()].take_action();
        match action {
            crate::screens::ScreenAction::Inspect(category, id) => {
                self.active_screen = ScreenId::Entities;
                self.screens[ScreenId::Entities.as_index()].inspect_entity(&category, &id);
                self.set_status(format!("Inspecting {} {}", category, id));
            }
            crate::screens::ScreenAction::LoadScript(path) => {
                self.active_screen = ScreenId::ScriptConsole;
                self.screens[ScreenId::ScriptConsole.as_index()].load_script_file(&path);
                self.set_status(format!("Loaded script {}", path.display()));
            }
            crate::screens::ScreenAction::None => {}
        }
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut terminal = init_terminal(self.prefs.mouse)?;
        let _guard = TerminalGuard;
        let mut event_loop = crate::event::EventLoop::new()?;

        while !self.should_quit {
            terminal.draw(|f| crate::ui::render(self, f))?;

            match event_loop.next().await? {
                crate::event::Event::Key(key) => crate::input::handle_key(self, key),
                crate::event::Event::Mouse(mouse) if self.prefs.mouse => {
                    self.mouse_pos = Some((mouse.column, mouse.row));
                    let size = terminal.size()?;
                    let main_area = Rect::new(0, 2, size.width, size.height.saturating_sub(4));

                    // Route mouse to quit dialog if active
                    if let Some(ref mut dialog) = self.quit_dialog {
                        self.menu_bar.hovered_label = None;
                        if let Some(btn) = dialog.handle_mouse(mouse) {
                            if btn == 0 {
                                self.quit_dialog = None;
                            } else if btn == 1 {
                                self.quit_dialog = None;
                                self.handle_command_action(CommandAction::SaveEntity);
                                self.should_quit = true;
                            } else if btn == 2 {
                                self.quit_dialog = None;
                                self.should_quit = true;
                            }
                        }
                        continue;
                    }

                    // Route mouse to command palette if open
                    if self.command_palette_open {
                        self.menu_bar.hovered_label = None;
                        if let Some(action) = self
                            .command_palette
                            .handle_mouse(mouse, Rect::new(0, 0, size.width, size.height))
                        {
                            self.command_palette_open = false;
                            self.handle_command_action(action);
                        }

                        // Close palette if clicking outside its area
                        if mouse.kind
                            == ratatui::crossterm::event::MouseEventKind::Down(
                                ratatui::crossterm::event::MouseButton::Left,
                            )
                        {
                            let width = 60.min(size.width.saturating_sub(4));
                            let height = 15.min(size.height.saturating_sub(4));
                            let x = (size.width.saturating_sub(width)) / 2;
                            let y = (size.height.saturating_sub(height)) / 2;
                            let palette_rect = Rect::new(x, y, width, height);
                            if mouse.column < palette_rect.x
                                || mouse.column >= palette_rect.x + palette_rect.width
                                || mouse.row < palette_rect.y
                                || mouse.row >= palette_rect.y + palette_rect.height
                            {
                                self.command_palette_open = false;
                            }
                        }
                        continue;
                    }

                    // Route mouse to menu bar (top bar + dropdowns)
                    if mouse.row <= 1 || self.menu_bar.open_menu.is_some() {
                        if let Some(action) = self
                            .menu_bar
                            .handle_mouse(mouse, Rect::new(0, 0, size.width, size.height))
                        {
                            self.handle_command_action(action);
                        }
                        continue;
                    }
                    // Mouse left the menu bar area with no menu open — clear hover
                    self.menu_bar.hovered_label = None;

                    let content_area = if self.sidebar_visible {
                        let h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(36)]);
                        let [c, _] = h.areas(main_area);
                        c
                    } else {
                        main_area
                    };

                    if self.sidebar_visible {
                        let sidebar_area = Rect::new(
                            size.width.saturating_sub(36),
                            main_area.y,
                            36,
                            main_area.height,
                        );
                        let inner_area = Rect::new(
                            sidebar_area.x,
                            sidebar_area.y + 1,
                            sidebar_area.width,
                            sidebar_area.height.saturating_sub(1),
                        );
                        if mouse.column >= sidebar_area.x {
                            self.sidebar_focused = true;
                            if let Some(action) =
                                self.command_sidebar.handle_mouse(mouse, inner_area)
                            {
                                self.sidebar_focused = false;
                                self.handle_command_action(action);
                            }
                            continue;
                        }
                    }

                    self.sidebar_focused = false;
                    self.active_screen_mut().handle_mouse(mouse, content_area);
                }
                _ => {}
            }

            self.handle_action();
        }

        Ok(())
    }
}

fn init_terminal(enable_mouse: bool) -> color_eyre::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    if enable_mouse {
        execute!(stdout, ratatui::crossterm::event::EnableMouseCapture)?;
        use std::io::Write;
        write!(stdout, "\x1b[?1003h")?;
        stdout.flush()?;
    }
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal() -> color_eyre::Result<()> {
    let mut stdout = io::stdout();
    use std::io::Write;
    let _ = write!(stdout, "\x1b[?1003l");
    let _ = execute!(
        stdout,
        LeaveAlternateScreen,
        ratatui::crossterm::event::DisableMouseCapture
    );
    let _ = disable_raw_mode();
    Ok(())
}
