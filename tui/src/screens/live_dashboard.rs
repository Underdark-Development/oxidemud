use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Sparkline, Table,
        TableState, Widget,
    },
};

use crate::network::{ConnectionStatus, SpadeTelemetry};
use crate::screens::{Screen, ScreenAction};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardView {
    Dashboard,
    FullLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardPane {
    Metrics,
    Players,
    Logs,
}

impl DashboardPane {
    pub fn next(self) -> Self {
        match self {
            DashboardPane::Metrics => DashboardPane::Players,
            DashboardPane::Players => DashboardPane::Logs,
            DashboardPane::Logs => DashboardPane::Metrics,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            DashboardPane::Metrics => DashboardPane::Logs,
            DashboardPane::Players => DashboardPane::Metrics,
            DashboardPane::Logs => DashboardPane::Players,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectField {
    Host,
    Port,
    Tls,
    ApiKey,
    SaveDefault,
}

impl ConnectField {
    pub fn next(self) -> Self {
        match self {
            ConnectField::Host => ConnectField::Port,
            ConnectField::Port => ConnectField::Tls,
            ConnectField::Tls => ConnectField::ApiKey,
            ConnectField::ApiKey => ConnectField::SaveDefault,
            ConnectField::SaveDefault => ConnectField::Host,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ConnectField::Host => ConnectField::SaveDefault,
            ConnectField::Port => ConnectField::Host,
            ConnectField::Tls => ConnectField::Port,
            ConnectField::ApiKey => ConnectField::Tls,
            ConnectField::SaveDefault => ConnectField::ApiKey,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectDialogState {
    pub is_open: bool,
    pub active_field: ConnectField,
    pub host: String,
    pub host_cursor: usize,
    pub port: String,
    pub port_cursor: usize,
    pub tls: bool,
    pub api_key: String,
    pub api_key_cursor: usize,
    pub save_default: bool,
}

impl Default for ConnectDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_field: ConnectField::Host,
            host: "127.0.0.1".into(),
            host_cursor: "127.0.0.1".len(),
            port: "8080".into(),
            port_cursor: "8080".len(),
            tls: false,
            api_key: String::new(),
            api_key_cursor: 0,
            save_default: true,
        }
    }
}

impl ConnectDialogState {
    pub fn new(host: String, port: String, tls: bool, api_key: String, save_default: bool) -> Self {
        let host_cursor = host.len();
        let port_cursor = port.len();
        let api_key_cursor = api_key.len();
        Self {
            is_open: false,
            active_field: ConnectField::Host,
            host,
            host_cursor,
            port,
            port_cursor,
            tls,
            api_key,
            api_key_cursor,
            save_default,
        }
    }

    pub fn sync_cursor_to_end(&mut self) {
        match self.active_field {
            ConnectField::Host => self.host_cursor = self.host.len(),
            ConnectField::Port => self.port_cursor = self.port.len(),
            ConnectField::ApiKey => self.api_key_cursor = self.api_key.len(),
            _ => {}
        }
    }
}

pub struct LiveDashboardScreen {
    action: ScreenAction,
    pub view_mode: DashboardView,
    pub focused_pane: DashboardPane,
    pub telemetry: Option<SpadeTelemetry>,
    pub status: ConnectionStatus,
    pub ping_ms: u64,
    pub drift_history: Vec<u64>,
    pub logs: Vec<String>,
    pub log_scroll_paused: bool,
    pub log_filter: String,
    pub table_state: TableState,
    pub gecho_dialog_open: bool,
    pub gecho_input: String,
    pub gecho_cursor: usize,
    pub kick_dialog_open: bool,
    pub kick_target: Option<String>,
    pub shutdown_dialog_open: bool,
    pub shutdown_delay_input: String,
    pub shutdown_cursor: usize,
    pub connect_dialog: ConnectDialogState,
    pub connect_input: String,
    pub metrics_rect: Rect,
    pub players_rect: Rect,
    pub logs_rect: Rect,
    pub header_rect: Rect,
    pub connect_dialog_rect: Rect,
    pub kick_dialog_rect: Rect,
    pub gecho_dialog_rect: Rect,
    pub shutdown_dialog_rect: Rect,
}

impl LiveDashboardScreen {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        LiveDashboardScreen {
            action: ScreenAction::None,
            view_mode: DashboardView::Dashboard,
            focused_pane: DashboardPane::Players,
            telemetry: None,
            status: ConnectionStatus::Disconnected,
            ping_ms: 0,
            drift_history: Vec::new(),
            logs: Vec::new(),
            log_scroll_paused: false,
            log_filter: String::new(),
            table_state,
            gecho_dialog_open: false,
            gecho_input: String::new(),
            gecho_cursor: 0,
            kick_dialog_open: false,
            kick_target: None,
            shutdown_dialog_open: false,
            shutdown_delay_input: String::new(),
            shutdown_cursor: 0,
            connect_dialog: ConnectDialogState::default(),
            connect_input: "127.0.0.1:8080".to_string(),
            metrics_rect: Rect::default(),
            players_rect: Rect::default(),
            logs_rect: Rect::default(),
            header_rect: Rect::default(),
            connect_dialog_rect: Rect::default(),
            kick_dialog_rect: Rect::default(),
            gecho_dialog_rect: Rect::default(),
            shutdown_dialog_rect: Rect::default(),
        }
    }

    pub fn open_connect_dialog(&mut self, host: &str, port: u16, tls: bool, api_key: Option<&str>) {
        let mut state = ConnectDialogState::new(
            host.to_string(),
            port.to_string(),
            tls,
            api_key.unwrap_or("").to_string(),
            true,
        );
        state.is_open = true;
        self.connect_dialog = state;
    }

    pub fn update_telemetry(
        &mut self,
        telemetry: SpadeTelemetry,
        status: ConnectionStatus,
        ping_ms: u64,
    ) {
        self.status = status;
        self.ping_ms = ping_ms;
        let drift_u64 = (telemetry.pulse_drift_ms.max(0.0) * 10.0) as u64;
        self.drift_history.push(drift_u64);
        if self.drift_history.len() > 30 {
            self.drift_history.remove(0);
        }

        for log in telemetry.logs.clone() {
            self.logs.push(log);
        }

        self.telemetry = Some(telemetry);
    }

    pub fn add_log(&mut self, log: String) {
        self.logs.push(log);
        if self.logs.len() > 5000 {
            self.logs.remove(0);
        }
    }

    pub fn submit_reconnect(&mut self) {
        let mut host = self.connect_dialog.host.trim().to_string();
        let mut port = self
            .connect_dialog
            .port
            .trim()
            .parse::<u16>()
            .unwrap_or(8080);
        let mut tls = self.connect_dialog.tls;
        let mut url = None;

        if host.starts_with("ws://")
            || host.starts_with("wss://")
            || host.starts_with("http://")
            || host.starts_with("https://")
        {
            if let Some((parsed_host, parsed_port, parsed_tls)) =
                crate::network::parse_host_port_from_url(&host)
            {
                url = Some(host.clone());
                host = parsed_host;
                port = parsed_port;
                tls = parsed_tls;
            }
        }

        let api_key = if self.connect_dialog.api_key.trim().is_empty() {
            None
        } else {
            Some(self.connect_dialog.api_key.trim().to_string())
        };

        self.action = ScreenAction::Reconnect {
            host: host.clone(),
            port,
            tls,
            api_key,
            url,
            save_default: self.connect_dialog.save_default,
        };
        self.connect_dialog.is_open = false;
        self.connect_input = format!("{}:{}", host, port);
        self.status = ConnectionStatus::Connecting;
    }

    fn submit_gecho(&mut self) {
        let msg = self.gecho_input.trim().to_string();
        if !msg.is_empty() {
            self.action = ScreenAction::RpcCall {
                method: "imm.gecho".into(),
                params: serde_json::json!({ "message": msg }),
                description: format!("Global Echo: \"{msg}\""),
            };
            self.logs.push(format!("[GECHO] {msg}"));
        }
        self.gecho_input.clear();
        self.gecho_cursor = 0;
        self.gecho_dialog_open = false;
    }

    fn submit_shutdown(&mut self) {
        let delay = self.shutdown_delay_input.trim().parse::<u32>().ok();
        self.action = ScreenAction::RpcCall {
            method: "imm.shutdown".into(),
            params: serde_json::json!({
                "confirm": true,
                "delay_mins": delay
            }),
            description: format!("Server Shutdown (delay: {delay:?})"),
        };
        self.logs.push(format!(
            "[SHUTDOWN] Initiating server shutdown (delay: {delay:?})..."
        ));
        self.shutdown_dialog_open = false;
        self.shutdown_delay_input.clear();
        self.shutdown_cursor = 0;
    }

    fn submit_kick(&mut self) {
        if let Some(target) = self.kick_target.take() {
            self.action = ScreenAction::RpcCall {
                method: "imm.force_command".into(),
                params: serde_json::json!({
                    "player_name": target,
                    "command": "quit",
                    "confirm": true
                }),
                description: format!("Kick Player {target}"),
            };
            self.logs.push(format!("[KICK] Kicking player {target}..."));
        }
        self.kick_dialog_open = false;
    }
}

impl Default for LiveDashboardScreen {
    fn default() -> Self {
        Self::new()
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn prev_char_boundary(s: &str, cursor: usize) -> usize {
    let cur = cursor.min(s.len());
    if cur == 0 {
        return 0;
    }
    s.char_indices()
        .map(|(idx, _)| idx)
        .take_while(|&idx| idx < cur)
        .last()
        .unwrap_or(0)
}

fn next_char_boundary(s: &str, cursor: usize) -> usize {
    let cur = cursor.min(s.len());
    s.char_indices()
        .map(|(idx, _)| idx)
        .find(|&idx| idx > cur)
        .unwrap_or(s.len())
}

fn prev_word_boundary(s: &str, cursor: usize) -> usize {
    let cur = cursor.min(s.len());
    if cur == 0 {
        return 0;
    }
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let idx = chars
        .iter()
        .position(|&(byte_idx, _)| byte_idx >= cur)
        .unwrap_or(chars.len());
    if idx == 0 {
        return 0;
    }
    let mut i = idx;
    while i > 0 && chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    if i == 0 {
        return 0;
    }
    if is_word_char(chars[i - 1].1) {
        while i > 0 && is_word_char(chars[i - 1].1) {
            i -= 1;
        }
    } else {
        while i > 0 && !is_word_char(chars[i - 1].1) && !chars[i - 1].1.is_whitespace() {
            i -= 1;
        }
    }
    if i < chars.len() {
        chars[i].0
    } else {
        0
    }
}

fn next_word_boundary(s: &str, cursor: usize) -> usize {
    let cur = cursor.min(s.len());
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let idx = chars
        .iter()
        .position(|&(byte_idx, _)| byte_idx >= cur)
        .unwrap_or(chars.len());
    if idx >= chars.len() {
        return s.len();
    }
    let mut i = idx;
    if chars[i].1.is_whitespace() {
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }
    } else if is_word_char(chars[i].1) {
        while i < chars.len() && is_word_char(chars[i].1) {
            i += 1;
        }
    } else {
        while i < chars.len() && !is_word_char(chars[i].1) && !chars[i].1.is_whitespace() {
            i += 1;
        }
    }
    if i < chars.len() {
        chars[i].0
    } else {
        s.len()
    }
}

fn handle_text_field_key(
    text: &mut String,
    cursor: &mut usize,
    key: KeyEvent,
    is_digits_only: bool,
) -> bool {
    let m = key.modifiers;
    *cursor = (*cursor).min(text.len());
    if !text.is_char_boundary(*cursor) {
        *cursor = prev_char_boundary(text, *cursor);
    }

    // Line navigation (Home / Cmd+Left / Ctrl+A)
    if key.code == KeyCode::Home
        || (m.contains(KeyModifiers::SUPER) && key.code == KeyCode::Left)
        || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('a'))
    {
        *cursor = 0;
        return true;
    }

    // Line navigation (End / Cmd+Right / Ctrl+E)
    if key.code == KeyCode::End
        || (m.contains(KeyModifiers::SUPER) && key.code == KeyCode::Right)
        || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('e'))
    {
        *cursor = text.len();
        return true;
    }

    // Word navigation (Alt+Left / Option+Left / Ctrl+Left / Alt+b)
    if ((m.contains(KeyModifiers::ALT) || m.contains(KeyModifiers::CONTROL))
        && key.code == KeyCode::Left)
        || (m.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('b'))
    {
        *cursor = prev_word_boundary(text, *cursor);
        return true;
    }

    // Word navigation (Alt+Right / Option+Right / Ctrl+Right / Alt+f)
    if ((m.contains(KeyModifiers::ALT) || m.contains(KeyModifiers::CONTROL))
        && key.code == KeyCode::Right)
        || (m.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('f'))
    {
        *cursor = next_word_boundary(text, *cursor);
        return true;
    }

    // Line deletion to start (Cmd+Backspace / Ctrl+U)
    if (m.contains(KeyModifiers::SUPER) && key.code == KeyCode::Backspace)
        || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('u'))
    {
        if *cursor > 0 {
            text.drain(..*cursor);
            *cursor = 0;
        }
        return true;
    }

    // Line deletion to end (Cmd+Delete / Ctrl+K)
    if (m.contains(KeyModifiers::SUPER) && key.code == KeyCode::Delete)
        || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('k'))
    {
        if *cursor < text.len() {
            text.truncate(*cursor);
        }
        return true;
    }

    // Word deletion backward (Alt+Backspace / Option+Backspace / Ctrl+Backspace / Ctrl+W)
    if (m.contains(KeyModifiers::ALT) && key.code == KeyCode::Backspace)
        || (m.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Backspace)
        || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('w'))
    {
        let prev = prev_word_boundary(text, *cursor);
        if prev < *cursor {
            text.drain(prev..*cursor);
            *cursor = prev;
        }
        return true;
    }

    // Word deletion forward (Alt+Delete / Option+Delete / Ctrl+Delete / Alt+d)
    if (m.contains(KeyModifiers::ALT) && key.code == KeyCode::Delete)
        || (m.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Delete)
        || (m.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('d'))
    {
        let next = next_word_boundary(text, *cursor);
        if next > *cursor {
            text.drain(*cursor..next);
        }
        return true;
    }

    // Single character deletion backward (Backspace / Ctrl+H)
    if key.code == KeyCode::Backspace
        || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('h'))
    {
        if *cursor > 0 {
            let prev = prev_char_boundary(text, *cursor);
            text.drain(prev..*cursor);
            *cursor = prev;
        }
        return true;
    }

    // Single character deletion forward (Delete / Ctrl+D)
    if key.code == KeyCode::Delete || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('d'))
    {
        if *cursor < text.len() {
            let next = next_char_boundary(text, *cursor);
            text.drain(*cursor..next);
        }
        return true;
    }

    // Single character navigation (Left / Ctrl+B)
    if key.code == KeyCode::Left || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('b')) {
        *cursor = prev_char_boundary(text, *cursor);
        return true;
    }

    // Single character navigation (Right / Ctrl+F)
    if key.code == KeyCode::Right || (m == KeyModifiers::CONTROL && key.code == KeyCode::Char('f'))
    {
        *cursor = next_char_boundary(text, *cursor);
        return true;
    }

    // Character insertion
    if let KeyCode::Char(c) = key.code {
        if !m.contains(KeyModifiers::CONTROL)
            && !m.contains(KeyModifiers::SUPER)
            && !m.contains(KeyModifiers::ALT)
            && (!is_digits_only || c.is_ascii_digit())
        {
            text.insert(*cursor, c);
            *cursor += c.len_utf8();
            return true;
        }
    }

    false
}

impl Screen for LiveDashboardScreen {
    fn name(&self) -> &str {
        "Live Dashboard"
    }

    fn modal_overlay_active(&self) -> bool {
        self.connect_dialog.is_open
            || self.gecho_dialog_open
            || self.kick_dialog_open
            || self.shutdown_dialog_open
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.connect_dialog.is_open {
            match key.code {
                KeyCode::Esc => {
                    self.connect_dialog.is_open = false;
                    return true;
                }
                KeyCode::Tab | KeyCode::Down => {
                    self.connect_dialog.active_field = self.connect_dialog.active_field.next();
                    self.connect_dialog.sync_cursor_to_end();
                    return true;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    self.connect_dialog.active_field = self.connect_dialog.active_field.prev();
                    self.connect_dialog.sync_cursor_to_end();
                    return true;
                }
                KeyCode::Enter => {
                    self.submit_reconnect();
                    return true;
                }
                KeyCode::Char(' ') => match self.connect_dialog.active_field {
                    ConnectField::Tls => {
                        self.connect_dialog.tls = !self.connect_dialog.tls;
                        return true;
                    }
                    ConnectField::SaveDefault => {
                        self.connect_dialog.save_default = !self.connect_dialog.save_default;
                        return true;
                    }
                    _ => {}
                },
                _ => {}
            }

            match self.connect_dialog.active_field {
                ConnectField::Host => {
                    handle_text_field_key(
                        &mut self.connect_dialog.host,
                        &mut self.connect_dialog.host_cursor,
                        key,
                        false,
                    );
                    return true;
                }
                ConnectField::Port => {
                    handle_text_field_key(
                        &mut self.connect_dialog.port,
                        &mut self.connect_dialog.port_cursor,
                        key,
                        true,
                    );
                    return true;
                }
                ConnectField::Tls => match key.code {
                    KeyCode::Left | KeyCode::Right => {
                        self.connect_dialog.tls = !self.connect_dialog.tls;
                        return true;
                    }
                    _ => return true,
                },
                ConnectField::ApiKey => {
                    handle_text_field_key(
                        &mut self.connect_dialog.api_key,
                        &mut self.connect_dialog.api_key_cursor,
                        key,
                        false,
                    );
                    return true;
                }
                ConnectField::SaveDefault => match key.code {
                    KeyCode::Left | KeyCode::Right => {
                        self.connect_dialog.save_default = !self.connect_dialog.save_default;
                        return true;
                    }
                    _ => return true,
                },
            }
        }

        if self.gecho_dialog_open {
            match key.code {
                KeyCode::Esc => {
                    self.gecho_dialog_open = false;
                    self.gecho_input.clear();
                    self.gecho_cursor = 0;
                    return true;
                }
                KeyCode::Enter => {
                    self.submit_gecho();
                    return true;
                }
                _ => {
                    handle_text_field_key(
                        &mut self.gecho_input,
                        &mut self.gecho_cursor,
                        key,
                        false,
                    );
                    return true;
                }
            }
        }

        if self.kick_dialog_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.kick_dialog_open = false;
                    self.kick_target = None;
                    return true;
                }
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.submit_kick();
                    return true;
                }
                _ => return true,
            }
        }

        if self.shutdown_dialog_open {
            match key.code {
                KeyCode::Esc => {
                    self.shutdown_dialog_open = false;
                    self.shutdown_delay_input.clear();
                    self.shutdown_cursor = 0;
                    return true;
                }
                KeyCode::Enter => {
                    self.submit_shutdown();
                    return true;
                }
                _ => {
                    handle_text_field_key(
                        &mut self.shutdown_delay_input,
                        &mut self.shutdown_cursor,
                        key,
                        true,
                    );
                    return true;
                }
            }
        }

        match key.code {
            KeyCode::Tab => {
                self.focused_pane = self.focused_pane.next();
                true
            }
            KeyCode::BackTab => {
                self.focused_pane = self.focused_pane.prev();
                true
            }
            KeyCode::Char('1') => {
                self.focused_pane = DashboardPane::Metrics;
                true
            }
            KeyCode::Char('2') => {
                self.focused_pane = DashboardPane::Players;
                true
            }
            KeyCode::Char('3') => {
                self.focused_pane = DashboardPane::Logs;
                true
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.view_mode = match self.view_mode {
                    DashboardView::Dashboard => DashboardView::FullLog,
                    DashboardView::FullLog => DashboardView::Dashboard,
                };
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.connect_dialog.is_open = true;
                self.connect_dialog.active_field = ConnectField::Host;
                self.connect_dialog.sync_cursor_to_end();
                true
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                self.gecho_dialog_open = true;
                self.gecho_cursor = self.gecho_input.len();
                true
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                if let Some(ref t) = self.telemetry {
                    if let Some(sel) = self.table_state.selected() {
                        if let Some(p) = t.players.get(sel) {
                            self.kick_target = Some(p.name.clone());
                            self.kick_dialog_open = true;
                            return true;
                        }
                    }
                }
                true
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.shutdown_dialog_open = true;
                self.shutdown_cursor = self.shutdown_delay_input.len();
                true
            }
            KeyCode::Char(' ') if self.view_mode == DashboardView::FullLog => {
                self.log_scroll_paused = !self.log_scroll_paused;
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref t) = self.telemetry {
                    if !t.players.is_empty() {
                        let i = match self.table_state.selected() {
                            Some(i) => (i + 1) % t.players.len(),
                            None => 0,
                        };
                        self.table_state.select(Some(i));
                    }
                }
                true
            }
            KeyCode::Up => {
                if let Some(ref t) = self.telemetry {
                    if !t.players.is_empty() {
                        let i = match self.table_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    t.players.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        self.table_state.select(Some(i));
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        match self.view_mode {
            DashboardView::Dashboard => self.render_dashboard(area, buf),
            DashboardView::FullLog => self.render_full_log(area, buf),
        }

        if self.gecho_dialog_open {
            self.render_gecho_dialog(area, buf);
        }

        if self.kick_dialog_open {
            self.render_kick_dialog(area, buf);
        }

        if self.shutdown_dialog_open {
            self.render_shutdown_dialog(area, buf);
        }

        if self.connect_dialog.is_open {
            self.render_connect_dialog(area, buf);
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, _area: Rect) {
        let (col, row) = (mouse.column, mouse.row);

        // 1. Connection Dialog mouse handling
        if self.connect_dialog.is_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let rect = self.connect_dialog_rect;
                if rect.width > 0 && rect.height > 0 {
                    // Click outside dialog closes/dismisses it
                    if col < rect.x
                        || col >= rect.x + rect.width
                        || row < rect.y
                        || row >= rect.y + rect.height
                    {
                        self.connect_dialog.is_open = false;
                        return;
                    }

                    let inner_y = rect.y + 1;
                    let inner_x = rect.x + 2;

                    let text_input_x = inner_x + 13 + 1;
                    if row == inner_y + 1 {
                        self.connect_dialog.active_field = ConnectField::Host;
                        if col >= text_input_x {
                            let click_col = (col - text_input_x) as usize;
                            self.connect_dialog.host_cursor = self
                                .connect_dialog
                                .host
                                .char_indices()
                                .nth(click_col)
                                .map(|(i, _)| i)
                                .unwrap_or(self.connect_dialog.host.len());
                        } else {
                            self.connect_dialog.host_cursor = self.connect_dialog.host.len();
                        }
                    } else if row == inner_y + 3 {
                        self.connect_dialog.active_field = ConnectField::Port;
                        if col >= text_input_x {
                            let click_col = (col - text_input_x) as usize;
                            self.connect_dialog.port_cursor = self
                                .connect_dialog
                                .port
                                .char_indices()
                                .nth(click_col)
                                .map(|(i, _)| i)
                                .unwrap_or(self.connect_dialog.port.len());
                        } else {
                            self.connect_dialog.port_cursor = self.connect_dialog.port.len();
                        }
                    } else if row == inner_y + 5 {
                        self.connect_dialog.active_field = ConnectField::Tls;
                        self.connect_dialog.tls = !self.connect_dialog.tls;
                    } else if row == inner_y + 7 {
                        self.connect_dialog.active_field = ConnectField::ApiKey;
                        if col >= text_input_x {
                            let click_col = (col - text_input_x) as usize;
                            self.connect_dialog.api_key_cursor = self
                                .connect_dialog
                                .api_key
                                .char_indices()
                                .nth(click_col)
                                .map(|(i, _)| i)
                                .unwrap_or(self.connect_dialog.api_key.len());
                        } else {
                            self.connect_dialog.api_key_cursor = self.connect_dialog.api_key.len();
                        }
                    } else if row == inner_y + 9 {
                        self.connect_dialog.active_field = ConnectField::SaveDefault;
                        self.connect_dialog.save_default = !self.connect_dialog.save_default;
                    } else if row == inner_y + 12 {
                        let rects = dialog_button_rects(rect, inner_y + 12, &CONNECT_BUTTONS);
                        if let Some(connect) = rects.first() {
                            if col >= connect.x && col < connect.x + connect.width {
                                self.submit_reconnect();
                            }
                        }
                        if let Some(cancel) = rects.last() {
                            if col >= cancel.x && col < cancel.x + cancel.width {
                                self.connect_dialog.is_open = false;
                            }
                        }
                    }
                }
            }
            return;
        }

        // 2. Kick Dialog mouse handling
        if self.kick_dialog_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let r = self.kick_dialog_rect;
                if r.width > 0 && r.height > 0 {
                    if col < r.x || col >= r.x + r.width || row < r.y || row >= r.y + r.height {
                        self.kick_dialog_open = false;
                        self.kick_target = None;
                    } else if row == r.y + 1 + r.height.saturating_sub(2) - 1 {
                        let rects = dialog_button_rects(r, row, &KICK_BUTTONS);
                        if let Some(rect) = rects.first() {
                            if col >= rect.x && col < rect.x + rect.width {
                                self.submit_kick();
                            }
                        }
                        if let Some(rect) = rects.last() {
                            if col >= rect.x && col < rect.x + rect.width {
                                self.kick_dialog_open = false;
                                self.kick_target = None;
                            }
                        }
                    }
                }
            }
            return;
        }

        // 3. Gecho & Shutdown dialogs
        if self.gecho_dialog_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let r = self.gecho_dialog_rect;
                if r.width > 0
                    && (col < r.x || col >= r.x + r.width || row < r.y || row >= r.y + r.height)
                {
                    self.gecho_dialog_open = false;
                    self.gecho_input.clear();
                    self.gecho_cursor = 0;
                } else if row == r.y + 1 + r.height.saturating_sub(2) - 1 {
                    let rects = dialog_button_rects(r, row, &GECHO_BUTTONS);
                    if let Some(rect) = rects.first() {
                        if col >= rect.x && col < rect.x + rect.width {
                            self.submit_gecho();
                        }
                    }
                    if let Some(rect) = rects.last() {
                        if col >= rect.x && col < rect.x + rect.width {
                            self.gecho_dialog_open = false;
                            self.gecho_input.clear();
                            self.gecho_cursor = 0;
                        }
                    }
                }
            }
            return;
        }
        if self.shutdown_dialog_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let r = self.shutdown_dialog_rect;
                if r.width > 0
                    && (col < r.x || col >= r.x + r.width || row < r.y || row >= r.y + r.height)
                {
                    self.shutdown_dialog_open = false;
                    self.shutdown_delay_input.clear();
                    self.shutdown_cursor = 0;
                } else if row == r.y + 1 + r.height.saturating_sub(2) - 1 {
                    let rects = dialog_button_rects(r, row, &SHUTDOWN_BUTTONS);
                    if let Some(rect) = rects.first() {
                        if col >= rect.x && col < rect.x + rect.width {
                            self.submit_shutdown();
                        }
                    }
                    if let Some(rect) = rects.last() {
                        if col >= rect.x && col < rect.x + rect.width {
                            self.shutdown_dialog_open = false;
                            self.shutdown_delay_input.clear();
                            self.shutdown_cursor = 0;
                        }
                    }
                }
            }
            return;
        }

        // 4. Main Dashboard Panes
        let in_rect = |r: Rect| {
            r.width > 0
                && r.height > 0
                && col >= r.x
                && col < r.x + r.width
                && row >= r.y
                && row < r.y + r.height
        };

        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            if in_rect(self.players_rect) {
                self.focused_pane = DashboardPane::Players;
                let rel_y = row.saturating_sub(self.players_rect.y + 2);
                if let Some(ref t) = self.telemetry {
                    if (rel_y as usize) < t.players.len() {
                        self.table_state.select(Some(rel_y as usize));
                    }
                }
            } else if in_rect(self.metrics_rect) {
                self.focused_pane = DashboardPane::Metrics;
            } else if in_rect(self.logs_rect) {
                self.focused_pane = DashboardPane::Logs;
            } else if in_rect(self.header_rect) {
                let host = self.connect_dialog.host.clone();
                let port = self.connect_dialog.port.parse().unwrap_or(8080);
                let tls = self.connect_dialog.tls;
                let api_key = self.connect_dialog.api_key.clone();
                self.open_connect_dialog(&host, port, tls, Some(&api_key));
            }
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn pane_block(title: &str, focused: bool) -> Block<'static> {
    let base = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {title} "));
    if focused {
        base.title_style(
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(theme::border_accent())
    } else {
        base.title_style(Style::default().fg(theme::FG_MUTED))
            .border_style(Style::default().fg(theme::SURFACE))
    }
}

impl LiveDashboardScreen {
    fn render_dashboard(&mut self, area: Rect, buf: &mut Buffer) {
        let is_small = area.width < 100 || area.height < 28;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 0: Top Header Banner
                Constraint::Min(10),   // 1: Middle (Metrics & Players)
                Constraint::Length(8), // 2: Bottom Log Feed
                Constraint::Length(1), // 3: Quick Keybindings Bar
            ])
            .split(area);

        self.header_rect = chunks[0];
        self.logs_rect = chunks[2];

        // 1. Header Box (Server Telemetry Summary)
        let uptime = self.telemetry.as_ref().map(|t| t.uptime_secs).unwrap_or(0);
        let uptime_str = format!(
            "{}h {}m {}s",
            uptime / 3600,
            (uptime % 3600) / 60,
            uptime % 60
        );
        let ping_str = if self.status == ConnectionStatus::Connected {
            format!("{}ms", self.ping_ms)
        } else {
            "--".to_string()
        };

        let wal_kb = self
            .telemetry
            .as_ref()
            .map(|t| t.wal_size_bytes / 1024)
            .unwrap_or(0);

        let header_text = format!(
            " Host: {} | Ping: {} | Uptime: {} | WAL: {} KB ",
            self.connect_input, ping_str, uptime_str, wal_kb
        );
        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Server Telemetry ")
            .style(Style::default().fg(theme::SURFACE));
        Paragraph::new(header_text)
            .block(header_block)
            .style(theme::text())
            .render(chunks[0], buf);

        // 2. Middle Row: Adaptive Split (Horizontal on wide, Vertical on narrow)
        let (sys_area, player_area) = if is_small {
            let small_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(9), Constraint::Min(6)])
                .split(chunks[1]);
            (small_chunks[0], small_chunks[1])
        } else {
            let wide_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[1]);
            (wide_chunks[0], wide_chunks[1])
        };

        self.metrics_rect = sys_area;
        self.players_rect = player_area;

        // --- Left/Top: System & Database Health Pane ---
        let sys_block = pane_block("System Health", self.focused_pane == DashboardPane::Metrics);
        sys_block.render(sys_area, buf);

        let inner_sys = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), // Memory Gauge
                Constraint::Length(2), // Players Gauge
                Constraint::Length(2), // Sparkline Tick Drift
                Constraint::Min(3),    // Stats Summary
            ])
            .split(sys_area);

        // Memory Gauge
        let mem_used = self
            .telemetry
            .as_ref()
            .map(|t| t.memory_used_bytes)
            .unwrap_or(0);
        let mem_total = self
            .telemetry
            .as_ref()
            .map(|t| t.total_memory_bytes)
            .unwrap_or(0);
        let mem_ratio = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64).min(1.0)
        } else {
            0.0
        };
        let mem_used_mb = mem_used as f64 / (1024.0 * 1024.0);
        let mem_total_gb = mem_total as f64 / (1024.0 * 1024.0 * 1024.0);
        let mem_color = if mem_ratio > 0.85 {
            theme::DANGER
        } else if mem_ratio > 0.60 {
            theme::WARNING
        } else {
            theme::PRIMARY
        };
        let mem_label = if mem_total > 0 {
            format!(
                "{:.1} MB / {:.1} GB ({:.1}%)",
                mem_used_mb,
                mem_total_gb,
                mem_ratio * 100.0
            )
        } else {
            "-".to_string()
        };

        Gauge::default()
            .block(Block::default().title("Memory Usage"))
            .gauge_style(Style::default().fg(mem_color).bg(theme::PANEL))
            .ratio(mem_ratio)
            .label(mem_label)
            .render(inner_sys[0], buf);

        // Players Gauge
        let players_count = self
            .telemetry
            .as_ref()
            .map(|t| t.players.len())
            .unwrap_or(0);
        let players_ratio = (players_count as f64 / 50.0).min(1.0);
        let players_label = format!("{} / 50 Active", players_count);
        Gauge::default()
            .block(Block::default().title("Active Players"))
            .gauge_style(Style::default().fg(theme::POSITIVE).bg(theme::PANEL))
            .ratio(players_ratio)
            .label(players_label)
            .render(inner_sys[1], buf);

        // Tick Drift Sparkline
        let last_drift = self.drift_history.last().copied().unwrap_or(0) as f64 / 10.0;
        let drift_color = if last_drift > 50.0 {
            theme::DANGER
        } else if last_drift > 15.0 {
            theme::WARNING
        } else {
            theme::POSITIVE
        };
        let drift_title = format!("Tick Drift: {:.1}ms", last_drift);
        Sparkline::default()
            .block(Block::default().title(drift_title))
            .data(&self.drift_history)
            .style(Style::default().fg(drift_color))
            .render(inner_sys[2], buf);

        // Stats summary
        let rooms = self.telemetry.as_ref().map(|t| t.room_count).unwrap_or(0);
        let mobs = self.telemetry.as_ref().map(|t| t.mob_count).unwrap_or(0);
        let items = self.telemetry.as_ref().map(|t| t.item_count).unwrap_or(0);
        let dirty = self
            .telemetry
            .as_ref()
            .map(|t| t.dirty_entities)
            .unwrap_or(0);
        let rhai_timers = self.telemetry.as_ref().map(|t| t.rhai_timers).unwrap_or(0);
        let game_time = self
            .telemetry
            .as_ref()
            .map(|t| t.game_time.as_str())
            .unwrap_or("N/A");
        let season = self
            .telemetry
            .as_ref()
            .map(|t| t.season.as_str())
            .unwrap_or("N/A");
        let weather = self
            .telemetry
            .as_ref()
            .map(|t| t.weather.as_str())
            .unwrap_or("N/A");

        let stats_text = format!(
            "SQLite WAL : {} KB (Queue: {} dirty)\n\
             Entities   : {} Rooms, {} Mobs, {} Items\n\
             Clock/Env  : {} ({}, {})\n\
             Rhai Timers: {} active scripts",
            wal_kb, dirty, rooms, mobs, items, game_time, season, weather, rhai_timers
        );
        Paragraph::new(stats_text)
            .style(theme::text())
            .render(inner_sys[3], buf);

        // --- Right/Bottom: Online Players Table Pane ---
        let player_title = format!("Online Players ({})", players_count);
        let player_block = pane_block(&player_title, self.focused_pane == DashboardPane::Players);

        let rows: Vec<Row> = if let Some(ref t) = self.telemetry {
            t.players
                .iter()
                .map(|p| {
                    Row::new(vec![
                        p.name.clone(),
                        p.level.to_string(),
                        p.class.clone(),
                        p.room.clone(),
                        format!("{}s", p.idle_secs),
                        p.protocol.clone(),
                    ])
                })
                .collect()
        } else {
            Vec::new()
        };

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(20),
                Constraint::Percentage(25),
                Constraint::Percentage(10),
                Constraint::Percentage(15),
            ],
        )
        .header(
            Row::new(vec!["Player", "Lvl", "Class", "Room", "Idle", "Proto"]).style(
                theme::text()
                    .fg(theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(player_block)
        .row_highlight_style(
            Style::default()
                .bg(theme::SURFACE)
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        );

        StatefulWidget::render(table, player_area, buf, &mut self.table_state);

        // --- 3. Bottom Panel: Event Log Feed ---
        let log_block = pane_block("Server Logs", self.focused_pane == DashboardPane::Logs);
        let items: Vec<ListItem> = self
            .logs
            .iter()
            .rev()
            .take(chunks[2].height.saturating_sub(2) as usize)
            .rev()
            .map(|l| {
                let style = if l.contains("ERROR") {
                    Style::default()
                        .fg(theme::DANGER)
                        .add_modifier(Modifier::BOLD)
                } else if l.contains("WARN") {
                    Style::default().fg(theme::WARNING)
                } else if l.contains("[NETWORK") || l.contains("[RPC") {
                    Style::default().fg(theme::PRIMARY)
                } else {
                    Style::default().fg(theme::FG_BRIGHT)
                };
                ListItem::new(l.as_str()).style(style)
            })
            .collect();
        Widget::render(List::new(items).block(log_block), chunks[2], buf);

        // --- 4. Footer Action Bar ---
        let footer_text = " [Tab] Switch Pane | [L] Full Logs | [G] Global Echo | [K] Kick Player | [S] Shutdown | [C] Settings ";
        Paragraph::new(footer_text)
            .style(Style::default().bg(theme::BG_DARK).fg(theme::FG_BRIGHT))
            .render(chunks[3], buf);
    }

    fn render_full_log(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);

        let pause_status = if self.log_scroll_paused {
            "PAUSED"
        } else {
            "ON"
        };
        let header_text = format!(
            " Filter: [All] | Auto-Scroll: {} (Press Space to Pause) | Total Lines: {} ",
            pause_status,
            self.logs.len()
        );
        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" SERVER LOG VIEW - UNEDITED STREAM ")
            .style(theme::border_accent());

        Paragraph::new(header_text)
            .block(header_block)
            .style(Style::default().fg(theme::WARNING))
            .render(chunks[0], buf);

        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(theme::border_accent());

        let items: Vec<ListItem> = self
            .logs
            .iter()
            .rev()
            .take(chunks[1].height.saturating_sub(2) as usize)
            .rev()
            .map(|l| {
                let style = if l.contains("ERROR") {
                    Style::default()
                        .fg(theme::DANGER)
                        .add_modifier(Modifier::BOLD)
                } else if l.contains("WARN") {
                    Style::default().fg(theme::WARNING)
                } else {
                    Style::default().fg(theme::PRIMARY)
                };
                ListItem::new(l.as_str()).style(style)
            })
            .collect();

        Widget::render(List::new(items).block(log_block), chunks[1], buf);

        let footer_text = " [Tab] Back to Dashboard | [Space] Pause Stream | [Q] Quit ";
        Paragraph::new(footer_text)
            .style(
                Style::default()
                    .bg(theme::PRIMARY)
                    .fg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            )
            .render(chunks[2], buf);
    }
}

fn clear_and_fill_dialog(area: Rect, title: &str, border_color: Color, buf: &mut Buffer) {
    Clear.render(area, buf);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(theme::BG_DARK));
            }
        }
    }
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(border_color).bg(theme::BG_DARK))
        .render(area, buf);

    let title_w = title.chars().count() as u16;
    if area.width >= title_w + 2 {
        for cx in (area.x + 1)..(area.x + area.width - 1) {
            if let Some(cell) = buf.cell_mut((cx, area.y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(theme::BG_DARK));
            }
        }
        let title_x = area.x + (area.width - title_w) / 2;
        buf.set_string(
            title_x,
            area.y,
            title,
            Style::default()
                .fg(border_color)
                .bg(theme::BG_DARK)
                .add_modifier(Modifier::BOLD),
        );
    }
}

struct TextField<'a> {
    label: &'a str,
    value: &'a str,
    cursor: usize,
    field_width: usize,
    placeholder: Option<&'a str>,
    is_focused: bool,
    is_masked: bool,
}

fn render_text_field(buf: &mut Buffer, x: u16, y: u16, field: TextField<'_>) {
    let is_focused = field.is_focused;
    let label = field.label;
    let value = field.value;
    let cursor = field.cursor.min(value.len());
    let field_width = field.field_width;
    let placeholder = field.placeholder;
    let is_masked = field.is_masked;
    let label_style = if is_focused {
        Style::default()
            .fg(theme::PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::FG_MUTED)
    };

    let bracket_style = if is_focused {
        Style::default()
            .fg(theme::PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::HOVER)
    };

    let bg_color = if is_focused {
        theme::BG
    } else {
        theme::BG_DARK
    };

    let text_style = if is_focused {
        Style::default().fg(theme::FG).bg(bg_color)
    } else {
        Style::default().fg(theme::FG_BRIGHT).bg(bg_color)
    };

    // Draw label (padded to 13 chars)
    let label_str = format!("{:<13}", label);
    buf.set_string(x, y, &label_str, label_style);

    let input_x = x + 13;

    // Draw opening bracket
    buf.set_string(input_x, y, "[", bracket_style);

    // Draw interior background
    let interior_x = input_x + 1;
    for i in 0..field_width {
        if let Some(cell) = buf.cell_mut((interior_x + i as u16, y)) {
            cell.set_char(' ');
            cell.set_style(Style::default().bg(bg_color));
        }
    }

    let max_chars = field_width.saturating_sub(2);
    let total_chars = value.chars().count();
    let char_cursor = value[..cursor].chars().count();

    // Horizontal scroll calculation if text exceeds box width
    let scroll_offset = if char_cursor >= max_chars {
        char_cursor + 1 - max_chars
    } else {
        0
    };

    // Draw text value or placeholder
    if value.is_empty() {
        if let Some(ph) = placeholder {
            let ph_disp: String = ph.chars().take(max_chars).collect();
            buf.set_string(
                interior_x + 1,
                y,
                &ph_disp,
                Style::default().fg(theme::FG_FAINT).bg(bg_color),
            );
        }
    } else {
        let text_to_show = if is_masked {
            "•".repeat(total_chars)
        } else {
            value.to_string()
        };
        let val_disp: String = text_to_show
            .chars()
            .skip(scroll_offset)
            .take(max_chars)
            .collect();
        buf.set_string(interior_x + 1, y, &val_disp, text_style);
    }

    // Draw active cursor
    if is_focused {
        let cursor_col = char_cursor.saturating_sub(scroll_offset);
        let cursor_offset = interior_x + 1 + (cursor_col.min(max_chars) as u16);
        if let Some(cell) = buf.cell_mut((cursor_offset, y)) {
            if cursor_col < max_chars && char_cursor < total_chars && !value.is_empty() {
                cell.set_style(
                    Style::default()
                        .fg(theme::BG)
                        .bg(theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                );
            } else {
                cell.set_char('█');
                cell.set_style(Style::default().fg(theme::PRIMARY).bg(bg_color));
            }
        }
    }

    // Draw closing bracket
    buf.set_string(interior_x + field_width as u16, y, "]", bracket_style);
}

fn render_checkbox(buf: &mut Buffer, x: u16, y: u16, label: &str, checked: bool, is_focused: bool) {
    let bracket_style = if is_focused {
        Style::default()
            .fg(theme::PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::HOVER)
    };

    let bg_color = if is_focused {
        theme::BG
    } else {
        theme::BG_DARK
    };

    let text_style = if is_focused {
        Style::default().fg(theme::FG).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::FG_BRIGHT)
    };

    buf.set_string(x, y, "[", bracket_style);
    if let Some(cell) = buf.cell_mut((x + 1, y)) {
        if checked {
            cell.set_char('X');
            cell.set_style(
                Style::default()
                    .fg(theme::POSITIVE)
                    .bg(bg_color)
                    .add_modifier(Modifier::BOLD),
            );
        } else {
            cell.set_char(' ');
            cell.set_style(Style::default().bg(bg_color));
        }
    }
    buf.set_string(x + 2, y, "]", bracket_style);

    buf.set_string(x + 4, y, label, text_style);
}

const CONNECT_BUTTONS: [(&str, theme::ButtonVariant, theme::ButtonState); 2] = [
    (
        " Connect (enter) ",
        theme::ButtonVariant::Default,
        theme::ButtonState::Active,
    ),
    (
        " Cancel (esc) ",
        theme::ButtonVariant::Neutral,
        theme::ButtonState::Inactive,
    ),
];

const KICK_BUTTONS: [(&str, theme::ButtonVariant, theme::ButtonState); 2] = [
    (
        " Confirm Kick (y) ",
        theme::ButtonVariant::Destructive,
        theme::ButtonState::Active,
    ),
    (
        " Cancel (n) ",
        theme::ButtonVariant::Neutral,
        theme::ButtonState::Inactive,
    ),
];

const GECHO_BUTTONS: [(&str, theme::ButtonVariant, theme::ButtonState); 2] = [
    (
        " Send (y) ",
        theme::ButtonVariant::Default,
        theme::ButtonState::Active,
    ),
    (
        " Cancel (n) ",
        theme::ButtonVariant::Neutral,
        theme::ButtonState::Inactive,
    ),
];

const SHUTDOWN_BUTTONS: [(&str, theme::ButtonVariant, theme::ButtonState); 2] = [
    (
        " Confirm (y) ",
        theme::ButtonVariant::Destructive,
        theme::ButtonState::Active,
    ),
    (
        " Cancel (n) ",
        theme::ButtonVariant::Neutral,
        theme::ButtonState::Inactive,
    ),
];

fn dialog_button_rects(
    dialog_area: Rect,
    row_y: u16,
    buttons: &[(&str, theme::ButtonVariant, theme::ButtonState)],
) -> Vec<Rect> {
    let btn_gap = 2u16;
    let row_w: u16 = buttons
        .iter()
        .map(|(label, _, _)| label.chars().count() as u16)
        .sum::<u16>()
        + btn_gap * (buttons.len().saturating_sub(1)) as u16;
    let mut x = dialog_area.x + dialog_area.width.saturating_sub(row_w + 2);
    let mut rects = Vec::with_capacity(buttons.len());
    for (label, _, _) in buttons {
        let width = label.chars().count() as u16;
        rects.push(Rect::new(x, row_y, width, 1));
        x += width + btn_gap;
    }
    rects
}

fn render_dialog_buttons(
    buf: &mut Buffer,
    dialog_area: Rect,
    row_y: u16,
    buttons: &[(&str, theme::ButtonVariant, theme::ButtonState)],
) {
    for ((label, variant, state), rect) in
        buttons
            .iter()
            .zip(dialog_button_rects(dialog_area, row_y, buttons))
    {
        buf.set_string(rect.x, rect.y, label, theme::button_style(*variant, *state));
    }
}

impl LiveDashboardScreen {
    fn render_kick_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        self.kick_dialog_rect = dialog_area;
        clear_and_fill_dialog(dialog_area, "Confirm Kick Player", theme::DANGER, buf);

        let target_name = self.kick_target.as_deref().unwrap_or("Unknown");
        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };
        let msg = format!("Disconnect player '{target_name}'?");
        buf.set_string(
            inner.x,
            inner.y,
            &msg,
            Style::default().fg(theme::FG).bg(theme::BG_DARK),
        );

        let row_y = inner.y + inner.height - 1;
        render_dialog_buttons(buf, dialog_area, row_y, &KICK_BUTTONS);
    }

    fn render_shutdown_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        self.shutdown_dialog_rect = dialog_area;
        clear_and_fill_dialog(dialog_area, "Server Shutdown (Admin)", theme::DANGER, buf);

        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };
        buf.set_string(
            inner.x,
            inner.y,
            "Shutdown delay (minutes):",
            Style::default().fg(theme::FG).bg(theme::BG_DARK),
        );

        let prompt_x = inner.x;
        let value_x = prompt_x + 3;
        buf.set_string(prompt_x, inner.y + 1, " > ", theme::prompt_style());
        if self.shutdown_delay_input.is_empty() {
            buf.set_string(value_x, inner.y + 1, "█", theme::prompt_style());
        } else {
            let cursor = self.shutdown_cursor.min(self.shutdown_delay_input.len());
            let before = &self.shutdown_delay_input[..cursor];
            let after = &self.shutdown_delay_input[cursor..];
            let value_style = Style::default().fg(theme::FG).bg(theme::BG_DARK);
            buf.set_string(value_x, inner.y + 1, before, value_style);
            buf.set_string(
                value_x + before.chars().count() as u16,
                inner.y + 1,
                "█",
                theme::prompt_style(),
            );
            buf.set_string(
                value_x + before.chars().count() as u16 + 1,
                inner.y + 1,
                after,
                value_style,
            );
        }

        let row_y = inner.y + inner.height - 1;
        render_dialog_buttons(buf, dialog_area, row_y, &SHUTDOWN_BUTTONS);
    }

    fn render_gecho_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        self.gecho_dialog_rect = dialog_area;
        clear_and_fill_dialog(
            dialog_area,
            "Broadcast Global Echo (gecho)",
            theme::WARNING,
            buf,
        );

        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };
        buf.set_string(
            inner.x,
            inner.y,
            "Enter announcement to broadcast:",
            Style::default().fg(theme::FG).bg(theme::BG_DARK),
        );

        let prompt_x = inner.x;
        let value_x = prompt_x + 3;
        buf.set_string(prompt_x, inner.y + 1, " > ", theme::prompt_style());
        if self.gecho_input.is_empty() {
            buf.set_string(value_x, inner.y + 1, "█", theme::prompt_style());
        } else {
            let cursor = self.gecho_cursor.min(self.gecho_input.len());
            let before = &self.gecho_input[..cursor];
            let after = &self.gecho_input[cursor..];
            let value_style = Style::default().fg(theme::FG).bg(theme::BG_DARK);
            buf.set_string(value_x, inner.y + 1, before, value_style);
            buf.set_string(
                value_x + before.chars().count() as u16,
                inner.y + 1,
                "█",
                theme::prompt_style(),
            );
            buf.set_string(
                value_x + before.chars().count() as u16 + 1,
                inner.y + 1,
                after,
                value_style,
            );
        }

        let row_y = inner.y + inner.height - 1;
        render_dialog_buttons(buf, dialog_area, row_y, &GECHO_BUTTONS);
    }

    fn render_connect_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        let width = 66.min(area.width.saturating_sub(2));
        let height = 17.min(area.height.saturating_sub(2));
        let dialog_area = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        self.connect_dialog_rect = dialog_area;
        clear_and_fill_dialog(dialog_area, "Connection Settings", theme::PRIMARY, buf);

        let inner_y = dialog_area.y + 1;
        let inner_x = dialog_area.x + 2;
        let field_box_width = 44.min(dialog_area.width.saturating_sub(20) as usize);

        // Row 0: Host / URL
        render_text_field(
            buf,
            inner_x,
            inner_y + 1,
            TextField {
                label: "Host / URL",
                value: &self.connect_dialog.host,
                cursor: self.connect_dialog.host_cursor,
                field_width: field_box_width,
                placeholder: Some("e.g. 127.0.0.1 or mud.oxide.org"),
                is_focused: self.connect_dialog.active_field == ConnectField::Host,
                is_masked: false,
            },
        );

        // Row 1: Port
        render_text_field(
            buf,
            inner_x,
            inner_y + 3,
            TextField {
                label: "Port",
                value: &self.connect_dialog.port,
                cursor: self.connect_dialog.port_cursor,
                field_width: field_box_width,
                placeholder: Some("e.g. 8080"),
                is_focused: self.connect_dialog.active_field == ConnectField::Port,
                is_masked: false,
            },
        );

        // Row 2: Use TLS (WSS/HTTPS)
        render_checkbox(
            buf,
            inner_x,
            inner_y + 5,
            "Use TLS (WSS/HTTPS)",
            self.connect_dialog.tls,
            self.connect_dialog.active_field == ConnectField::Tls,
        );

        // Row 3: API Key
        render_text_field(
            buf,
            inner_x,
            inner_y + 7,
            TextField {
                label: "API Key",
                value: &self.connect_dialog.api_key,
                cursor: self.connect_dialog.api_key_cursor,
                field_width: field_box_width,
                placeholder: Some("(none - optional)"),
                is_focused: self.connect_dialog.active_field == ConnectField::ApiKey,
                is_masked: true,
            },
        );

        // Row 4: Save as Default
        render_checkbox(
            buf,
            inner_x,
            inner_y + 9,
            "Save as default",
            self.connect_dialog.save_default,
            self.connect_dialog.active_field == ConnectField::SaveDefault,
        );

        // Footer buttons: [ Connect ] and [ Cancel ] (right-aligned)
        let btn_y = inner_y + 12;
        render_dialog_buttons(buf, dialog_area, btn_y, &CONNECT_BUTTONS);

        // Clean keyboard hints (centered, no bracket fatigue)
        let hint_line = "Tab/↑↓ Field  •  Space Toggle  •  Enter Connect  •  Esc Cancel";
        let hint_len = hint_line.chars().count() as u16;
        let hint_x = dialog_area.x + (dialog_area.width.saturating_sub(hint_len)) / 2;
        buf.set_string(
            hint_x,
            inner_y + 14,
            hint_line,
            Style::default().fg(theme::FG_MUTED).bg(theme::BG_DARK),
        );
    }
}

use ratatui::widgets::StatefulWidget;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::OnlinePlayerInfo;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_dialog_button_row_preserves_right_border() {
        let w = 66u16;
        let h = 2u16;
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        for y in 0..h {
            if let Some(cell) = buf.cell_mut((w - 1, y)) {
                cell.set_symbol("│");
            }
        }
        render_dialog_buttons(
            &mut buf,
            Rect::new(0, 0, w, h),
            1,
            &[
                (
                    " Connect (enter) ",
                    crate::theme::ButtonVariant::Default,
                    crate::theme::ButtonState::Active,
                ),
                (
                    " Cancel (esc) ",
                    crate::theme::ButtonVariant::Neutral,
                    crate::theme::ButtonState::Inactive,
                ),
            ],
        );
        assert_eq!(buf[(w - 1, 1)].symbol(), "│");
        assert_eq!(buf[(w - 2, 1)].symbol(), " ");
    }

    #[test]
    fn test_gecho_action_emits_rpc_call() {
        let mut screen = LiveDashboardScreen::new();
        screen.status = ConnectionStatus::Connected;

        // Press 'g' to open gecho dialog
        assert!(screen.handle_key(make_key(KeyCode::Char('g'))));
        assert!(screen.gecho_dialog_open);
        assert!(screen.modal_overlay_active());

        // Type "Server rebooting soon"
        for c in "Server rebooting soon".chars() {
            screen.handle_key(make_key(KeyCode::Char(c)));
        }
        assert_eq!(screen.gecho_input, "Server rebooting soon");

        // Press Enter to submit
        assert!(screen.handle_key(make_key(KeyCode::Enter)));
        assert!(!screen.gecho_dialog_open);

        // Verify emitted ScreenAction
        match screen.take_action() {
            ScreenAction::RpcCall {
                method,
                params,
                description,
            } => {
                assert_eq!(method, "imm.gecho");
                assert_eq!(params["message"], "Server rebooting soon");
                assert!(description.contains("Global Echo"));
            }
            other => panic!("Expected RpcCall, got {other:?}"),
        }
    }

    #[test]
    fn test_kick_action_emits_rpc_call() {
        let mut screen = LiveDashboardScreen::new();
        let telemetry = SpadeTelemetry {
            status: "connected".into(),
            uptime_secs: 100,
            memory_used_bytes: 1000,
            total_memory_bytes: 2000,
            wal_size_bytes: 0,
            dirty_entities: 0,
            pulse_drift_ms: 0.0,
            room_count: 10,
            mob_count: 5,
            item_count: 5,
            game_time: "12:00".into(),
            season: "Spring".into(),
            weather: "Clear".into(),
            rhai_timers: 0,
            players: vec![OnlinePlayerInfo {
                name: "Gimli".into(),
                level: 10,
                class: "Warrior".into(),
                race: "Dwarf".into(),
                room: "vale_start".into(),
                idle_secs: 12,
                protocol: "Telnet".into(),
            }],
            logs: vec![],
        };
        screen.update_telemetry(telemetry, ConnectionStatus::Connected, 5);

        // Press 'k' to open kick dialog for selected player
        assert!(screen.handle_key(make_key(KeyCode::Char('k'))));
        assert!(screen.kick_dialog_open);
        assert_eq!(screen.kick_target.as_deref(), Some("Gimli"));
        assert!(screen.modal_overlay_active());

        // Press Enter to confirm kick
        assert!(screen.handle_key(make_key(KeyCode::Enter)));
        assert!(!screen.kick_dialog_open);

        // Verify emitted ScreenAction
        match screen.take_action() {
            ScreenAction::RpcCall {
                method,
                params,
                description,
            } => {
                assert_eq!(method, "imm.force_command");
                assert_eq!(params["player_name"], "Gimli");
                assert_eq!(params["command"], "quit");
                assert_eq!(params["confirm"], true);
                assert!(description.contains("Kick Player Gimli"));
            }
            other => panic!("Expected RpcCall, got {other:?}"),
        }
    }

    #[test]
    fn test_shutdown_action_emits_rpc_call() {
        let mut screen = LiveDashboardScreen::new();
        screen.status = ConnectionStatus::Connected;

        // Press 's' to open shutdown dialog
        assert!(screen.handle_key(make_key(KeyCode::Char('s'))));
        assert!(screen.shutdown_dialog_open);
        assert!(screen.modal_overlay_active());

        // Type delay "5"
        screen.handle_key(make_key(KeyCode::Char('5')));
        assert_eq!(screen.shutdown_delay_input, "5");

        // Press Enter to confirm
        assert!(screen.handle_key(make_key(KeyCode::Enter)));
        assert!(!screen.shutdown_dialog_open);

        // Verify emitted ScreenAction
        match screen.take_action() {
            ScreenAction::RpcCall {
                method,
                params,
                description,
            } => {
                assert_eq!(method, "imm.shutdown");
                assert_eq!(params["confirm"], true);
                assert_eq!(params["delay_mins"], 5);
                assert!(description.contains("Server Shutdown"));
            }
            other => panic!("Expected RpcCall, got {other:?}"),
        }
    }

    #[test]
    fn test_dashboard_pane_focus_cycling_and_number_shortcuts() {
        let mut screen = LiveDashboardScreen::new();
        assert_eq!(screen.focused_pane, DashboardPane::Players);

        // Tab cycles forward: Players -> Logs -> Metrics -> Players
        assert!(screen.handle_key(make_key(KeyCode::Tab)));
        assert_eq!(screen.focused_pane, DashboardPane::Logs);

        assert!(screen.handle_key(make_key(KeyCode::Tab)));
        assert_eq!(screen.focused_pane, DashboardPane::Metrics);

        assert!(screen.handle_key(make_key(KeyCode::Tab)));
        assert_eq!(screen.focused_pane, DashboardPane::Players);

        // BackTab cycles backward
        assert!(screen.handle_key(make_key(KeyCode::BackTab)));
        assert_eq!(screen.focused_pane, DashboardPane::Metrics);

        // Number keys jump directly to panes
        assert!(screen.handle_key(make_key(KeyCode::Char('3'))));
        assert_eq!(screen.focused_pane, DashboardPane::Logs);

        assert!(screen.handle_key(make_key(KeyCode::Char('1'))));
        assert_eq!(screen.focused_pane, DashboardPane::Metrics);

        assert!(screen.handle_key(make_key(KeyCode::Char('2'))));
        assert_eq!(screen.focused_pane, DashboardPane::Players);
    }

    #[test]
    fn test_connect_dialog_multi_field_navigation_and_reconnect_action() {
        let mut screen = LiveDashboardScreen::new();

        // Press 'c' to open connect dialog
        assert!(screen.handle_key(make_key(KeyCode::Char('c'))));
        assert!(screen.connect_dialog.is_open);
        assert_eq!(screen.connect_dialog.active_field, ConnectField::Host);
        assert!(screen.modal_overlay_active());

        // Clear default host and type "mud.oxide.org"
        screen.connect_dialog.host.clear();
        for c in "mud.oxide.org".chars() {
            screen.handle_key(make_key(KeyCode::Char(c)));
        }
        assert_eq!(screen.connect_dialog.host, "mud.oxide.org");

        // Tab to Port
        assert!(screen.handle_key(make_key(KeyCode::Tab)));
        assert_eq!(screen.connect_dialog.active_field, ConnectField::Port);
        screen.connect_dialog.port.clear();
        for c in "9000".chars() {
            screen.handle_key(make_key(KeyCode::Char(c)));
        }
        assert_eq!(screen.connect_dialog.port, "9000");

        // Tab to TLS and toggle on
        assert!(screen.handle_key(make_key(KeyCode::Tab)));
        assert_eq!(screen.connect_dialog.active_field, ConnectField::Tls);
        assert!(!screen.connect_dialog.tls);
        assert!(screen.handle_key(make_key(KeyCode::Char(' '))));
        assert!(screen.connect_dialog.tls);

        // Tab to ApiKey and type secret
        assert!(screen.handle_key(make_key(KeyCode::Tab)));
        assert_eq!(screen.connect_dialog.active_field, ConnectField::ApiKey);
        for c in "adminkey123".chars() {
            screen.handle_key(make_key(KeyCode::Char(c)));
        }
        assert_eq!(screen.connect_dialog.api_key, "adminkey123");

        // Tab to SaveDefault
        assert!(screen.handle_key(make_key(KeyCode::Tab)));
        assert_eq!(
            screen.connect_dialog.active_field,
            ConnectField::SaveDefault
        );
        assert!(screen.connect_dialog.save_default);

        // Submit form
        assert!(screen.handle_key(make_key(KeyCode::Enter)));
        assert!(!screen.connect_dialog.is_open);

        // Verify emitted ScreenAction::Reconnect
        match screen.take_action() {
            ScreenAction::Reconnect {
                host,
                port,
                tls,
                api_key,
                save_default,
                ..
            } => {
                assert_eq!(host, "mud.oxide.org");
                assert_eq!(port, 9000);
                assert!(tls);
                assert_eq!(api_key.as_deref(), Some("adminkey123"));
                assert!(save_default);
            }
            other => panic!("Expected Reconnect action, got {other:?}"),
        }
    }

    #[test]
    fn test_connect_dialog_url_auto_parse_on_reconnect() {
        let mut screen = LiveDashboardScreen::new();
        screen.open_connect_dialog("localhost", 4000, false, None);
        assert!(screen.connect_dialog.is_open);

        // Type full URL into host field
        screen.connect_dialog.host = "wss://game.domain.com:8443/ws/spade".into();

        // Submit
        assert!(screen.handle_key(make_key(KeyCode::Enter)));
        assert!(!screen.connect_dialog.is_open);

        // Verify auto-parsed parameters
        match screen.take_action() {
            ScreenAction::Reconnect {
                host,
                port,
                tls,
                url,
                ..
            } => {
                assert_eq!(host, "game.domain.com");
                assert_eq!(port, 8443);
                assert!(tls);
                assert_eq!(url.as_deref(), Some("wss://game.domain.com:8443/ws/spade"));
            }
            other => panic!("Expected Reconnect action, got {other:?}"),
        }
    }

    #[test]
    fn test_mouse_interaction_with_connect_dialog() {
        let mut screen = LiveDashboardScreen::new();
        screen.open_connect_dialog("127.0.0.1", 8080, false, Some("key123"));
        assert!(screen.connect_dialog.is_open);

        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf, None);
        assert!(screen.connect_dialog_rect.width > 0);

        let dialog = screen.connect_dialog_rect;

        // 1. Click on Port row (inner_y + 3)
        let click_port = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: dialog.x + 10,
            row: dialog.y + 4,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_port, area);
        assert_eq!(screen.connect_dialog.active_field, ConnectField::Port);

        // 2. Click on TLS row (inner_y + 5) -> toggles TLS
        assert!(!screen.connect_dialog.tls);
        let click_tls = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: dialog.x + 10,
            row: dialog.y + 6,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_tls, area);
        assert_eq!(screen.connect_dialog.active_field, ConnectField::Tls);
        assert!(screen.connect_dialog.tls);

        // 3. Click the right-aligned Connect chip (row: dialog.y + 13)
        let btn_row = dialog.y + 13;
        let connect_btn = dialog_button_rects(dialog, btn_row, &CONNECT_BUTTONS)[0];
        let cancel_btn = dialog_button_rects(dialog, btn_row, &CONNECT_BUTTONS)[1];
        let click_connect_btn = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: connect_btn.x + 3,
            row: btn_row,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_connect_btn, area);
        assert!(!screen.connect_dialog.is_open);

        match screen.take_action() {
            ScreenAction::Reconnect {
                host,
                port,
                tls,
                api_key,
                ..
            } => {
                assert_eq!(host, "127.0.0.1");
                assert_eq!(port, 8080);
                assert!(tls);
                assert_eq!(api_key.as_deref(), Some("key123"));
            }
            other => panic!("Expected Reconnect action from button click, got {other:?}"),
        }

        // 4. Click at the old centered ghost position is inert
        screen.open_connect_dialog("127.0.0.1", 8080, false, None);
        assert!(screen.connect_dialog.is_open);
        let click_ghost = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: dialog.x + 20,
            row: btn_row,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_ghost, area);
        assert!(screen.connect_dialog.is_open);
        assert!(matches!(screen.take_action(), ScreenAction::None));

        // 5. Click the Cancel chip closes without submitting
        let click_cancel_btn = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: cancel_btn.x + 2,
            row: btn_row,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_cancel_btn, area);
        assert!(!screen.connect_dialog.is_open);
        assert!(matches!(screen.take_action(), ScreenAction::None));

        // 6. Click outside dialog dismisses
        screen.open_connect_dialog("127.0.0.1", 8080, false, None);
        assert!(screen.connect_dialog.is_open);
        let click_outside = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_outside, area);
        assert!(!screen.connect_dialog.is_open);
    }

    #[test]
    fn test_mouse_chips_on_kick_dialog() {
        let mut screen = LiveDashboardScreen::new();
        screen.kick_target = Some("Player1".into());
        screen.kick_dialog_open = true;

        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf, None);
        assert!(screen.kick_dialog_rect.width > 0);

        let dialog = screen.kick_dialog_rect;
        let btn_row = dialog.y + 1 + dialog.height.saturating_sub(2) - 1;
        let rects = dialog_button_rects(dialog, btn_row, &KICK_BUTTONS);
        let click_at = |x: u16| MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: btn_row,
            modifiers: KeyModifiers::NONE,
        };

        screen.handle_mouse(click_at(rects[1].x + 2), area);
        assert!(!screen.kick_dialog_open);
        assert!(matches!(screen.take_action(), ScreenAction::None));

        screen.kick_target = Some("Player1".into());
        screen.kick_dialog_open = true;
        screen.handle_mouse(click_at(rects[0].x + 3), area);
        assert!(!screen.kick_dialog_open);
        match screen.take_action() {
            ScreenAction::RpcCall { method, params, .. } => {
                assert_eq!(method, "imm.force_command");
                assert_eq!(params["player_name"], "Player1");
            }
            other => panic!("Expected RpcCall kick action, got {other:?}"),
        }
    }

    #[test]
    fn test_mouse_interaction_with_dashboard_panes() {
        let mut screen = LiveDashboardScreen::new();
        let telemetry = SpadeTelemetry {
            status: "Running".into(),
            uptime_secs: 100,
            memory_used_bytes: 500,
            total_memory_bytes: 1000,
            wal_size_bytes: 0,
            dirty_entities: 0,
            pulse_drift_ms: 0.0,
            room_count: 5,
            mob_count: 2,
            item_count: 3,
            game_time: "10:00".into(),
            season: "Spring".into(),
            weather: "Clear".into(),
            rhai_timers: 0,
            players: vec![
                OnlinePlayerInfo {
                    name: "Player1".into(),
                    level: 1,
                    class: "Warrior".into(),
                    race: "Human".into(),
                    room: "start".into(),
                    idle_secs: 0,
                    protocol: "Telnet".into(),
                },
                OnlinePlayerInfo {
                    name: "Player2".into(),
                    level: 2,
                    class: "Mage".into(),
                    race: "Elf".into(),
                    room: "tower".into(),
                    idle_secs: 5,
                    protocol: "WebSocket".into(),
                },
            ],
            logs: vec![],
        };
        screen.update_telemetry(telemetry, ConnectionStatus::Connected, 10);

        let area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf, None);

        // Click on Logs pane
        let logs_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: screen.logs_rect.x + 2,
            row: screen.logs_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(logs_click, area);
        assert_eq!(screen.focused_pane, DashboardPane::Logs);

        // Click on Players pane row 1 (Player2)
        let player2_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: screen.players_rect.x + 5,
            row: screen.players_rect.y + 3,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(player2_click, area);
        assert_eq!(screen.focused_pane, DashboardPane::Players);
        assert_eq!(screen.table_state.selected(), Some(1));

        // Click on Metrics pane
        let metrics_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: screen.metrics_rect.x + 2,
            row: screen.metrics_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(metrics_click, area);
        assert_eq!(screen.focused_pane, DashboardPane::Metrics);

        // Click on Header opens connect dialog
        let header_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: screen.header_rect.x + 5,
            row: screen.header_rect.y + 1,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(header_click, area);
        assert!(screen.connect_dialog.is_open);
    }

    #[test]
    fn test_word_boundary_functions() {
        let s = "mud.oxide.org";
        // End of "mud.oxide.org" (13): word is "org" (indices 10..13)
        assert_eq!(prev_word_boundary(s, 13), 10);
        // At "." (10): punctuation is "." (indices 9..10)
        assert_eq!(prev_word_boundary(s, 10), 9);
        // At "oxide" (9): word is "oxide" (indices 4..9)
        assert_eq!(prev_word_boundary(s, 9), 4);
        // Start: 0
        assert_eq!(prev_word_boundary(s, 0), 0);

        // Forward
        assert_eq!(next_word_boundary(s, 0), 3); // "mud"
        assert_eq!(next_word_boundary(s, 3), 4); // "."
        assert_eq!(next_word_boundary(s, 4), 9); // "oxide"
        assert_eq!(next_word_boundary(s, 10), 13); // "org"
        assert_eq!(next_word_boundary(s, 13), 13);
    }

    #[test]
    fn test_text_field_os_shortcuts() {
        let mut screen = LiveDashboardScreen::new();
        screen.open_connect_dialog("mud.oxide.org", 8080, false, None);
        assert_eq!(screen.connect_dialog.host, "mud.oxide.org");
        assert_eq!(screen.connect_dialog.host_cursor, 13);

        // 1. Option+Backspace (Alt+Backspace) deletes word backward: "org"
        screen.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(screen.connect_dialog.host, "mud.oxide.");
        assert_eq!(screen.connect_dialog.host_cursor, 10);

        // 2. Ctrl+W deletes word/symbol backward: "."
        screen.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(screen.connect_dialog.host, "mud.oxide");
        assert_eq!(screen.connect_dialog.host_cursor, 9);

        // 3. Home / Cmd+Left moves to start
        screen.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::SUPER));
        assert_eq!(screen.connect_dialog.host_cursor, 0);

        // 4. Option+Right moves word forward
        screen.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
        assert_eq!(screen.connect_dialog.host_cursor, 3); // after "mud"

        // 5. Option+Delete (Alt+Delete) deletes word forward: "."
        screen.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::ALT));
        assert_eq!(screen.connect_dialog.host, "mudoxide");
        assert_eq!(screen.connect_dialog.host_cursor, 3);

        // 6. Cmd+Delete (Super+Delete) deletes to end of line: deletes "oxide"
        screen.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SUPER));
        assert_eq!(screen.connect_dialog.host, "mud");
        assert_eq!(screen.connect_dialog.host_cursor, 3);

        // 7. Cmd+Backspace (Super+Backspace) deletes to start of line: deletes "mud"
        screen.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::SUPER));
        assert_eq!(screen.connect_dialog.host, "");
        assert_eq!(screen.connect_dialog.host_cursor, 0);

        // 8. Type "hello world"
        for c in "hello world".chars() {
            screen.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(screen.connect_dialog.host, "hello world");
        assert_eq!(screen.connect_dialog.host_cursor, 11);

        // 9. Ctrl+U deletes entire line to start
        screen.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(screen.connect_dialog.host, "");
        assert_eq!(screen.connect_dialog.host_cursor, 0);

        // 10. Port field ignores non-digits
        screen.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(screen.connect_dialog.active_field, ConnectField::Port);
        screen.connect_dialog.port.clear();
        screen.connect_dialog.port_cursor = 0;
        for c in "abc890xyz!".chars() {
            screen.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(screen.connect_dialog.port, "890");
    }

    #[test]
    fn test_mouse_click_cursor_placement() {
        let mut screen = LiveDashboardScreen::new();
        screen.open_connect_dialog("example.com", 8080, false, None);

        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf, None);

        let dialog = screen.connect_dialog_rect;
        let inner_x = dialog.x + 2;
        let text_input_x = inner_x + 13 + 1;

        // Click on 4th character of Host ("m" in "example")
        let click_host = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: text_input_x + 4,
            row: dialog.y + 2,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_host, area);
        assert_eq!(screen.connect_dialog.active_field, ConnectField::Host);
        assert_eq!(screen.connect_dialog.host_cursor, 4);
    }
}
