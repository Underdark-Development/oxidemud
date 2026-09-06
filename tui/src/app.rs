use crate::components::command_palette::CommandPalette;
use crate::components::command_sidebar::CommandSidebar;
use crate::components::menu_bar::MenuBar;
use crate::components::CommandAction;
use crate::config_file::{PrefsConfig, SpadeConfig};
use crate::content::{self, FileMap};
use crate::network::ConnectionStatus;
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
    pub connection_url: Option<String>,
    pub connection_host: String,
    pub connection_port: u16,
    pub connection_tls: bool,
    pub api_key: Option<String>,
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
    pub notification_history: Vec<(String, String)>,
    pub notification_dialog: Option<crate::components::Dialog>,
    pub connect_dialog: Option<crate::components::Dialog>,
    pub network_client: Option<crate::network::SpadeNetworkClient>,
    pub rpc_resp_tx:
        tokio::sync::mpsc::UnboundedSender<(String, Result<serde_json::Value, String>)>,
    pub rpc_resp_rx:
        tokio::sync::mpsc::UnboundedReceiver<(String, Result<serde_json::Value, String>)>,
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

impl App {
    pub fn new(cli: crate::config::Config, file_config: SpadeConfig) -> Self {
        let mode = cli.mode();
        let mut host = cli
            .connect_host
            .unwrap_or_else(|| file_config.connection.host.clone());
        let mut port = cli.connect_port.unwrap_or(file_config.connection.port);
        let url = cli.url.clone();
        let mut tls = file_config.connection.tls
            || url
                .as_ref()
                .map(|u| u.starts_with("wss://") || u.starts_with("https://"))
                .unwrap_or(false);

        if let Some(ref u) = url {
            if let Some((parsed_host, parsed_port, parsed_tls)) =
                crate::network::parse_host_port_from_url(u)
            {
                host = parsed_host;
                port = parsed_port;
                tls = parsed_tls;
            }
        }

        let api_key = cli.api_key.or(file_config.connection.api_key);
        let content_path = PathBuf::from(file_config.content_path.clone());

        let (registry, file_map) = content::load_templates(&content_path);
        let entities =
            EntitiesScreen::new_shared(content_path.clone(), registry.clone(), file_map.clone());
        let room_grid =
            RoomGridScreen::new(content_path.clone(), registry.clone(), file_map.clone());
        let file_browser = FileBrowserScreen::new(content_path.clone());
        let script_console = ScriptConsoleScreen::new();
        let mut live_dashboard = LiveDashboardScreen::new();

        let default_target = if let Some(ref u) = url {
            u.clone()
        } else {
            format!("{}:{}", host, port)
        };
        live_dashboard.connect_input = default_target;
        live_dashboard.connect_dialog = crate::screens::live_dashboard::ConnectDialogState::new(
            host.clone(),
            port.to_string(),
            tls,
            api_key.clone().unwrap_or_default(),
            true,
        );

        let screens: Vec<Box<dyn Screen>> = vec![
            Box::new(entities),
            Box::new(room_grid),
            Box::new(ValidationPanelScreen::new(registry.clone())),
            Box::new(file_browser),
            Box::new(script_console),
            Box::new(live_dashboard),
        ];

        let (rpc_resp_tx, rpc_resp_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            mode,
            should_quit: false,
            mouse_pos: None,
            status_message: None,
            connection_url: url.clone(),
            connection_host: host.clone(),
            connection_port: port,
            connection_tls: tls,
            api_key: api_key.clone(),
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
            notification_history: Vec::new(),
            notification_dialog: None,
            connect_dialog: None,
            rpc_resp_tx,
            rpc_resp_rx,
            network_client: if mode != Mode::Offline {
                Some(crate::network::SpadeNetworkClient::connect(
                    url.as_deref(),
                    &host,
                    port,
                    tls,
                    api_key,
                ))
            } else {
                None
            },
        }
    }

    pub fn connection_status(&self) -> ConnectionStatus {
        self.network_client
            .as_ref()
            .map(|c| c.status())
            .unwrap_or(ConnectionStatus::Disconnected)
    }

    pub fn ping_ms(&self) -> u64 {
        self.network_client
            .as_ref()
            .map(|c| c.ping_ms())
            .unwrap_or(0)
    }

    pub fn rpc(&self) -> Option<std::sync::Arc<oxide_ws_rpc::RpcClient>> {
        self.network_client.as_ref().and_then(|c| c.rpc())
    }

    pub fn confirm_quit(&mut self) {
        let total_unsaved: usize = self.screens.iter().map(|s| s.unsaved_count()).sum();
        if total_unsaved > 0 {
            self.quit_dialog = Some(crate::components::Dialog::new(
                crate::theme::DialogTone::Destructive,
                "Unsaved Changes",
                &format!(
                    "You have unsaved changes in {} entity/entities.\nDo you want to save before quitting?",
                    total_unsaved
                ),
                &["Cancel".into(), "Save & Quit".into(), "Quit Without Saving".into()],
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
        if id == ScreenId::LiveDashboard {
            self.sidebar_visible = false;
            self.sidebar_focused = false;
        }
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
                if self.active_screen == ScreenId::LiveDashboard {
                    self.sidebar_visible = false;
                    self.sidebar_focused = false;
                } else {
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
            }
            CommandAction::ShowNotificationHistory => {
                let history_text = if self.notification_history.is_empty() {
                    "No notifications logged yet.".to_string()
                } else {
                    self.notification_history
                        .iter()
                        .rev()
                        .take(15)
                        .map(|(t, m)| format!("[{t}] {m}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                self.notification_dialog = Some(crate::components::Dialog::new(
                    crate::theme::DialogTone::Info,
                    "Notification History",
                    &history_text,
                    &["Close".into()],
                ));
            }
            CommandAction::ShowAbout => {
                self.set_status("MUD Game Engine — spade v0.1.0");
            }
            CommandAction::SwitchMode(mode) => {
                self.mode = mode;
                self.set_status(format!("Switched execution mode to {:?}", mode));
            }
            CommandAction::ConnectServer => {
                let host = self.connection_host.clone();
                let port = self.connection_port;
                let url = self.connection_url.clone();
                let tls = self.connection_tls;
                let api_key = self.api_key.clone();
                self.network_client = Some(crate::network::SpadeNetworkClient::connect(
                    url.as_deref(),
                    &host,
                    port,
                    tls,
                    api_key,
                ));
                self.mode = Mode::Online;
                self.set_status(format!("Connecting to {}:{}...", host, port));
            }
            CommandAction::DisconnectServer => {
                self.network_client = None;
                self.mode = Mode::Offline;
                self.set_status("Disconnected from server.");
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
            crate::screens::ScreenAction::RpcCall {
                method,
                params,
                description,
            } => {
                if let Some(rpc) = self.rpc() {
                    let desc = description.clone();
                    let tx = self.rpc_resp_tx.clone();
                    tokio::spawn(async move {
                        let res = rpc.call(&method, params).await.map_err(|e| e.to_string());
                        let _ = tx.send((desc, res));
                    });
                    self.set_status(format!("Sending {description}..."));
                } else {
                    self.set_status(format!("Cannot {description}: not connected to server RPC"));
                    let screen = &mut self.screens[ScreenId::LiveDashboard.as_index()];
                    if let Some(dash) = screen.as_any_mut().downcast_mut::<LiveDashboardScreen>() {
                        dash.add_log(format!(
                            "[ERROR] Not connected to server RPC (cannot {description})"
                        ));
                    }
                }
            }
            crate::screens::ScreenAction::Reconnect {
                host,
                port,
                tls,
                api_key,
                url,
                save_default,
            } => {
                self.connection_host = host.clone();
                self.connection_port = port;
                self.connection_tls = tls;
                self.api_key = api_key.clone();
                self.connection_url = url.clone();
                self.mode = Mode::Online;

                if save_default {
                    let mut cfg = crate::config_file::load_config();
                    cfg.connection.host = host.clone();
                    cfg.connection.port = port;
                    cfg.connection.tls = tls;
                    cfg.connection.api_key = api_key.clone();
                    if let Err(e) = crate::config_file::save_config(&cfg) {
                        self.set_status(format!("Config save error: {e}"));
                    } else {
                        self.set_status("Connection saved to config.toml");
                    }
                }

                self.network_client = Some(crate::network::SpadeNetworkClient::connect(
                    url.as_deref(),
                    &host,
                    port,
                    tls,
                    api_key,
                ));
                self.set_status(format!("Connecting to {}:{}...", host, port));
            }
            crate::screens::ScreenAction::None => {}
        }
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut terminal = init_terminal(self.prefs.mouse)?;
        let _guard = TerminalGuard;
        let mut event_loop = crate::event::EventLoop::new()?;

        while !self.should_quit {
            while let Ok((desc, res)) = self.rpc_resp_rx.try_recv() {
                match res {
                    Ok(val) => {
                        let msg = val
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Success");
                        self.set_status(format!("{desc}: {msg}"));
                        let screen = &mut self.screens[ScreenId::LiveDashboard.as_index()];
                        if let Some(dash) =
                            screen.as_any_mut().downcast_mut::<LiveDashboardScreen>()
                        {
                            dash.handle_rpc_response(&desc, &val);
                            dash.add_log(format!("[RPC SUCCESS] {desc}: {msg}"));
                        }
                    }
                    Err(err) => {
                        self.set_status(format!("{desc} failed: {err}"));
                        let screen = &mut self.screens[ScreenId::LiveDashboard.as_index()];
                        if let Some(dash) =
                            screen.as_any_mut().downcast_mut::<LiveDashboardScreen>()
                        {
                            dash.add_log(format!("[RPC ERROR] {desc} failed: {err}"));
                        }
                    }
                }
            }

            if let Some(ref mut client) = self.network_client {
                let status = client.status();
                let ping = client.ping_ms();

                let screen = &mut self.screens[ScreenId::LiveDashboard.as_index()];
                if let Some(dash) = screen.as_any_mut().downcast_mut::<LiveDashboardScreen>() {
                    dash.status = status;
                    dash.ping_ms = ping;
                }

                if let Some(telemetry) = client.poll_telemetry() {
                    let screen = &mut self.screens[ScreenId::LiveDashboard.as_index()];
                    if let Some(dash) = screen.as_any_mut().downcast_mut::<LiveDashboardScreen>() {
                        dash.update_telemetry(telemetry, status, ping);
                    }
                }
                while let Some(log_line) = client.poll_log() {
                    let screen = &mut self.screens[ScreenId::LiveDashboard.as_index()];
                    if let Some(dash) = screen.as_any_mut().downcast_mut::<LiveDashboardScreen>() {
                        dash.add_log(log_line);
                    }
                }
            } else {
                let screen = &mut self.screens[ScreenId::LiveDashboard.as_index()];
                if let Some(dash) = screen.as_any_mut().downcast_mut::<LiveDashboardScreen>() {
                    dash.status = ConnectionStatus::Disconnected;
                }
            }

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

                    // Route mouse to notification dialog if active
                    if let Some(ref mut dialog) = self.notification_dialog {
                        self.menu_bar.hovered_label = None;
                        if dialog.handle_mouse(mouse).is_some() {
                            self.notification_dialog = None;
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

pub(crate) fn init_terminal(enable_mouse: bool) -> color_eyre::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _ = execute!(
        stdout,
        ratatui::crossterm::event::PushKeyboardEnhancementFlags(
            ratatui::crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    );
    if enable_mouse {
        execute!(stdout, ratatui::crossterm::event::EnableMouseCapture)?;
        use std::io::Write;
        write!(stdout, "\x1b[?1003h")?;
        stdout.flush()?;
    }
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

pub(crate) fn restore_terminal() -> color_eyre::Result<()> {
    let mut stdout = io::stdout();
    use std::io::Write;
    let _ = write!(stdout, "\x1b[?1003l");
    let _ = execute!(
        stdout,
        ratatui::crossterm::event::PopKeyboardEnhancementFlags,
        LeaveAlternateScreen,
        ratatui::crossterm::event::DisableMouseCapture
    );
    let _ = disable_raw_mode();
    Ok(())
}
