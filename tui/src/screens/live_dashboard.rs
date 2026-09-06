use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Sparkline, Table,
        TableState, Widget,
    },
};

use std::time::Instant;

use crate::components::Button;
use crate::network::{ConnectionStatus, OnlinePlayerInfo, SpadeTelemetry};
use crate::screens::{Screen, ScreenAction};
use crate::theme::{self, ButtonState, ButtonVariant};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFilterTab {
    #[default]
    All,
    Errors,
    Warnings,
    Info,
    NetworkRpc,
    Game,
}

impl LogFilterTab {
    pub fn all() -> &'static [LogFilterTab] {
        &[
            LogFilterTab::All,
            LogFilterTab::Errors,
            LogFilterTab::Warnings,
            LogFilterTab::Info,
            LogFilterTab::NetworkRpc,
            LogFilterTab::Game,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            LogFilterTab::All => "All",
            LogFilterTab::Errors => "Errors",
            LogFilterTab::Warnings => "Warnings",
            LogFilterTab::Info => "Info",
            LogFilterTab::NetworkRpc => "Network/RPC",
            LogFilterTab::Game => "Game",
        }
    }

    pub fn matches(self, line: &str) -> bool {
        let upper = line.to_uppercase();
        match self {
            LogFilterTab::All => true,
            LogFilterTab::Errors => {
                upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("CRITICAL")
            }
            LogFilterTab::Warnings => upper.contains("WARN"),
            LogFilterTab::Info => {
                upper.contains("INFO") && !upper.contains("WARN") && !upper.contains("ERROR")
            }
            LogFilterTab::NetworkRpc => {
                upper.contains("[NETWORK")
                    || upper.contains("[RPC")
                    || upper.contains("[WS")
                    || upper.contains("[TELNET")
                    || upper.contains("CONNECTION")
                    || upper.contains("DISCONNECT")
            }
            LogFilterTab::Game => {
                upper.contains("[COMBAT")
                    || upper.contains("[GAME")
                    || upper.contains("[ROOM")
                    || upper.contains("[PLAYER")
                    || upper.contains("[MOB")
                    || upper.contains("[QUEST")
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            LogFilterTab::All => LogFilterTab::Errors,
            LogFilterTab::Errors => LogFilterTab::Warnings,
            LogFilterTab::Warnings => LogFilterTab::Info,
            LogFilterTab::Info => LogFilterTab::NetworkRpc,
            LogFilterTab::NetworkRpc => LogFilterTab::Game,
            LogFilterTab::Game => LogFilterTab::All,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            LogFilterTab::All => LogFilterTab::Game,
            LogFilterTab::Errors => LogFilterTab::All,
            LogFilterTab::Warnings => LogFilterTab::Errors,
            LogFilterTab::Info => LogFilterTab::Warnings,
            LogFilterTab::NetworkRpc => LogFilterTab::Info,
            LogFilterTab::Game => LogFilterTab::NetworkRpc,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogSearchState {
    pub active: bool,
    pub query: String,
    pub cursor: usize,
    pub matches: Vec<usize>,
    pub selected_match: usize,
}

impl LogSearchState {
    pub fn update_matches(&mut self, filtered_logs: &[&String]) {
        self.matches.clear();
        let trimmed = self.query.trim();
        if trimmed.is_empty() {
            self.selected_match = 0;
            return;
        }
        let q = trimmed.to_lowercase();
        for (i, log) in filtered_logs.iter().enumerate() {
            if log.to_lowercase().contains(&q) {
                self.matches.push(i);
            }
        }
        if !self.matches.is_empty() && self.selected_match >= self.matches.len() {
            self.selected_match = self.matches.len() - 1;
        }
    }

    pub fn next_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        self.selected_match = (self.selected_match + 1) % self.matches.len();
        Some(self.matches[self.selected_match])
    }

    pub fn prev_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        self.selected_match = if self.selected_match == 0 {
            self.matches.len() - 1
        } else {
            self.selected_match - 1
        };
        Some(self.matches[self.selected_match])
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogStreamState {
    pub filter_tab: LogFilterTab,
    pub scroll_offset: usize,
    pub paused: bool,
    pub search: LogSearchState,
    pub toast: Option<(String, Instant)>,
    pub tab_rects: Vec<(LogFilterTab, Rect)>,
    pub pause_chip_rect: Rect,
    pub search_chip_rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterVitals {
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterAttributes {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

#[derive(Debug, Clone, Default)]
pub struct CharacterInspectorState {
    pub is_open: bool,
    pub player: Option<OnlinePlayerInfo>,
    pub health: Option<CharacterVitals>,
    pub mana: Option<CharacterVitals>,
    pub stamina: Option<CharacterVitals>,
    pub attributes: Option<CharacterAttributes>,
    pub advance_dialog_open: bool,
    pub advance_input: String,
    pub advance_cursor: usize,
    pub force_cmd_dialog_open: bool,
    pub force_cmd_input: String,
    pub force_cmd_cursor: usize,
    pub drawer_rect: Rect,
    pub button_rects: Vec<(&'static str, Rect)>,
    pub advance_dialog_rect: Rect,
    pub force_cmd_dialog_rect: Rect,
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
    pub log_stream: LogStreamState,
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
    pub character_inspector: CharacterInspectorState,
    pub last_player_click: Option<(usize, Instant)>,
    pub player_sort_column: usize,
    pub player_sort_ascending: bool,
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
            log_stream: LogStreamState::default(),
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
            character_inspector: CharacterInspectorState::default(),
            last_player_click: None,
            player_sort_column: 0,
            player_sort_ascending: true,
        }
    }

    pub fn open_character_inspector(&mut self, player: OnlinePlayerInfo) {
        let name = player.name.clone();
        self.character_inspector.player = Some(player);
        self.character_inspector.health = None;
        self.character_inspector.mana = None;
        self.character_inspector.stamina = None;
        self.character_inspector.attributes = None;
        self.character_inspector.is_open = true;

        // Query imm.stat
        self.action = ScreenAction::RpcCall {
            method: "imm.stat".into(),
            params: serde_json::json!({ "target_name": name }),
            description: format!("Character Stats for {name}"),
        };
    }

    pub fn close_character_inspector(&mut self) {
        self.character_inspector.is_open = false;
        self.character_inspector.advance_dialog_open = false;
        self.character_inspector.force_cmd_dialog_open = false;
    }

    pub fn handle_rpc_response(&mut self, desc: &str, val: &serde_json::Value) {
        if desc.starts_with("Character Stats for") || val.get("health").is_some() {
            if let Some(ref current_p) = self.character_inspector.player {
                if let Some(target) = val.get("target").and_then(|t| t.as_str()) {
                    if !target.eq_ignore_ascii_case(&current_p.name) {
                        return;
                    }
                }
            }

            if let Some(h) = val.get("health") {
                let cur = h.get("current").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let max = h.get("max").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                self.character_inspector.health = Some(CharacterVitals { current: cur, max });
            }
            if let Some(m) = val.get("mana") {
                let cur = m.get("current").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let max = m.get("max").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                self.character_inspector.mana = Some(CharacterVitals { current: cur, max });
            }
            if let Some(s) = val.get("stamina") {
                let cur = s.get("current").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let max = s.get("max").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                self.character_inspector.stamina = Some(CharacterVitals { current: cur, max });
            }
            if let Some(a) = val.get("attributes") {
                self.character_inspector.attributes = Some(CharacterAttributes {
                    strength: a.get("strength").and_then(|v| v.as_i64()).unwrap_or(10) as i32,
                    dexterity: a.get("dexterity").and_then(|v| v.as_i64()).unwrap_or(10) as i32,
                    constitution: a.get("constitution").and_then(|v| v.as_i64()).unwrap_or(10)
                        as i32,
                    intelligence: a.get("intelligence").and_then(|v| v.as_i64()).unwrap_or(10)
                        as i32,
                    wisdom: a.get("wisdom").and_then(|v| v.as_i64()).unwrap_or(10) as i32,
                    charisma: a.get("charisma").and_then(|v| v.as_i64()).unwrap_or(10) as i32,
                });
            }
        }
    }

    pub fn submit_heal(&mut self) {
        if let Some(ref p) = self.character_inspector.player {
            let name = p.name.clone();
            self.action = ScreenAction::RpcCall {
                method: "imm.heal".into(),
                params: serde_json::json!({ "target_name": name }),
                description: format!("Heal {name}"),
            };
        }
    }

    pub fn submit_revive(&mut self) {
        if let Some(ref p) = self.character_inspector.player {
            let name = p.name.clone();
            self.action = ScreenAction::RpcCall {
                method: "imm.revive".into(),
                params: serde_json::json!({ "target_name": name }),
                description: format!("Revive {name}"),
            };
        }
    }

    pub fn submit_advance(&mut self, target_level: u8) {
        if let Some(ref p) = self.character_inspector.player {
            let name = p.name.clone();
            self.action = ScreenAction::RpcCall {
                method: "imm.advance".into(),
                params: serde_json::json!({ "player_name": name, "target_level": target_level }),
                description: format!("Advance {name} to level {target_level}"),
            };
        }
        self.character_inspector.advance_dialog_open = false;
    }

    pub fn submit_force_command(&mut self, command: &str) {
        if let Some(ref p) = self.character_inspector.player {
            let name = p.name.clone();
            self.action = ScreenAction::RpcCall {
                method: "imm.force_command".into(),
                params: serde_json::json!({
                    "player_name": name,
                    "command": command,
                    "confirm": true,
                }),
                description: format!("Force command on {name}: {command}"),
            };
        }
        self.character_inspector.force_cmd_dialog_open = false;
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
            self.add_log(log);
        }

        self.telemetry = Some(telemetry);
        self.apply_player_sort();
    }

    pub fn handle_player_header_click(&mut self, col: usize) {
        if col == self.player_sort_column {
            self.player_sort_ascending = !self.player_sort_ascending;
        } else {
            self.player_sort_column = col.min(5);
            self.player_sort_ascending = true;
        }
        self.apply_player_sort();
    }

    pub fn apply_player_sort(&mut self) {
        if let Some(ref mut t) = self.telemetry {
            let selected_name = self
                .table_state
                .selected()
                .and_then(|idx| t.players.get(idx).map(|p| p.name.clone()));

            let col = self.player_sort_column;
            let asc = self.player_sort_ascending;

            t.players.sort_by(|a, b| {
                let cmp = match col {
                    0 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    1 => a
                        .level
                        .cmp(&b.level)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                    2 => a
                        .class
                        .to_lowercase()
                        .cmp(&b.class.to_lowercase())
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                    3 => a
                        .room
                        .to_lowercase()
                        .cmp(&b.room.to_lowercase())
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                    4 => a
                        .idle_secs
                        .cmp(&b.idle_secs)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                    5 => a
                        .protocol
                        .to_lowercase()
                        .cmp(&b.protocol.to_lowercase())
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                };
                if asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            });

            if let Some(name) = selected_name {
                if let Some(new_idx) = t.players.iter().position(|p| p.name == name) {
                    self.table_state.select(Some(new_idx));
                } else if !t.players.is_empty() {
                    let clamped = self
                        .table_state
                        .selected()
                        .unwrap_or(0)
                        .min(t.players.len() - 1);
                    self.table_state.select(Some(clamped));
                } else {
                    self.table_state.select(None);
                }
            }
        }
    }

    pub fn add_log(&mut self, log: String) {
        self.logs.push(log);
        if self.logs.len() > 5000 {
            self.logs.remove(0);
        }
        if !self.log_stream.paused && self.log_stream.scroll_offset == 0 {
            // Stay pinned to the bottom
            self.log_stream.scroll_offset = 0;
        }
        if !self.log_stream.search.query.trim().is_empty() {
            let filtered: Vec<&String> = self
                .logs
                .iter()
                .filter(|l| self.log_stream.filter_tab.matches(l))
                .collect();
            self.log_stream.search.update_matches(&filtered);
        }
    }

    pub fn scroll_log_up(&mut self, lines: usize) {
        let total = self
            .logs
            .iter()
            .filter(|l| self.log_stream.filter_tab.matches(l))
            .count();
        self.log_stream.scroll_offset = self
            .log_stream
            .scroll_offset
            .saturating_add(lines)
            .min(total);
        self.log_stream.paused = true;
        self.log_scroll_paused = true;
    }

    pub fn scroll_log_down(&mut self, lines: usize) {
        self.log_stream.scroll_offset = self.log_stream.scroll_offset.saturating_sub(lines);
        if self.log_stream.scroll_offset == 0 && !self.log_scroll_paused {
            self.log_stream.paused = false;
        }
    }

    pub fn scroll_log_to_top(&mut self) {
        let total = self
            .logs
            .iter()
            .filter(|l| self.log_stream.filter_tab.matches(l))
            .count();
        self.log_stream.scroll_offset = total;
        self.log_stream.paused = true;
        self.log_scroll_paused = true;
    }

    pub fn scroll_log_to_bottom(&mut self) {
        self.log_stream.scroll_offset = 0;
        self.log_stream.paused = false;
        self.log_scroll_paused = false;
    }

    pub fn center_on_filtered_index(
        &mut self,
        match_idx: usize,
        total: usize,
        visible_height: usize,
    ) {
        if total == 0 || visible_height == 0 {
            self.log_stream.scroll_offset = 0;
            return;
        }
        let half_h = visible_height / 2;
        let end = (match_idx + half_h + 1).min(total);
        self.log_stream.scroll_offset = total.saturating_sub(end);
        let max_scroll = total.saturating_sub(visible_height);
        self.log_stream.scroll_offset = self.log_stream.scroll_offset.min(max_scroll);
        self.log_stream.paused = true;
        self.log_scroll_paused = true;
    }

    pub fn jump_search_match(&mut self, forward: bool, visible_height: usize) {
        let filtered: Vec<&String> = self
            .logs
            .iter()
            .filter(|l| self.log_stream.filter_tab.matches(l))
            .collect();
        self.log_stream.search.update_matches(&filtered);
        let match_idx = if forward {
            self.log_stream.search.next_match()
        } else {
            self.log_stream.search.prev_match()
        };
        if let Some(idx) = match_idx {
            self.center_on_filtered_index(idx, filtered.len(), visible_height);
        }
    }

    pub fn export_logs(&mut self) {
        let filtered: Vec<&String> = self
            .logs
            .iter()
            .filter(|l| self.log_stream.filter_tab.matches(l))
            .collect();

        let count = filtered.len();
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("spade-logs-{}.log", now_secs);

        let content = filtered
            .iter()
            .map(|s| (*s).as_str())
            .collect::<Vec<_>>()
            .join("\n");
        match std::fs::write(&filename, content) {
            Ok(_) => {
                self.log_stream.toast = Some((
                    format!("Exported {} lines to {}", count, filename),
                    Instant::now(),
                ));
            }
            Err(e) => {
                self.log_stream.toast = Some((format!("Export failed: {}", e), Instant::now()));
            }
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
            || self.log_stream.search.active
            || self.character_inspector.is_open
            || self.character_inspector.advance_dialog_open
            || self.character_inspector.force_cmd_dialog_open
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

        if self.character_inspector.advance_dialog_open {
            match key.code {
                KeyCode::Esc => {
                    self.character_inspector.advance_dialog_open = false;
                    return true;
                }
                KeyCode::Enter => {
                    let lvl = self
                        .character_inspector
                        .advance_input
                        .trim()
                        .parse::<u8>()
                        .unwrap_or(1);
                    self.submit_advance(lvl);
                    return true;
                }
                _ => {
                    handle_text_field_key(
                        &mut self.character_inspector.advance_input,
                        &mut self.character_inspector.advance_cursor,
                        key,
                        true,
                    );
                    return true;
                }
            }
        }

        if self.character_inspector.force_cmd_dialog_open {
            match key.code {
                KeyCode::Esc => {
                    self.character_inspector.force_cmd_dialog_open = false;
                    return true;
                }
                KeyCode::Enter => {
                    let cmd = self.character_inspector.force_cmd_input.trim().to_string();
                    self.submit_force_command(&cmd);
                    return true;
                }
                _ => {
                    handle_text_field_key(
                        &mut self.character_inspector.force_cmd_input,
                        &mut self.character_inspector.force_cmd_cursor,
                        key,
                        false,
                    );
                    return true;
                }
            }
        }

        if self.character_inspector.is_open {
            match key.code {
                KeyCode::Esc => {
                    self.close_character_inspector();
                    return true;
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.submit_heal();
                    return true;
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.submit_revive();
                    return true;
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.character_inspector.advance_dialog_open = true;
                    self.character_inspector.advance_input = "50".into();
                    self.character_inspector.advance_cursor = 2;
                    return true;
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.character_inspector.force_cmd_dialog_open = true;
                    self.character_inspector.force_cmd_input.clear();
                    self.character_inspector.force_cmd_cursor = 0;
                    return true;
                }
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    if let Some(ref p) = self.character_inspector.player {
                        self.kick_target = Some(p.name.clone());
                        self.kick_dialog_open = true;
                    }
                    return true;
                }
                _ => return true,
            }
        }

        // Search Input Overlay
        if self.log_stream.search.active {
            match key.code {
                KeyCode::Esc => {
                    self.log_stream.search.active = false;
                    return true;
                }
                KeyCode::Enter => {
                    self.log_stream.search.active = false;
                    return true;
                }
                _ => {
                    if handle_text_field_key(
                        &mut self.log_stream.search.query,
                        &mut self.log_stream.search.cursor,
                        key,
                        false,
                    ) {
                        let filtered: Vec<&String> = self
                            .logs
                            .iter()
                            .filter(|l| self.log_stream.filter_tab.matches(l))
                            .collect();
                        self.log_stream.search.update_matches(&filtered);
                        if let Some(idx) = self.log_stream.search.matches.last().copied() {
                            self.center_on_filtered_index(idx, filtered.len(), 10);
                        }
                    }
                    return true;
                }
            }
        }

        // Dedicated Full Log Screen View
        if self.view_mode == DashboardView::FullLog {
            match key.code {
                KeyCode::Esc | KeyCode::Tab | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.view_mode = DashboardView::Dashboard;
                    return true;
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.view_mode = DashboardView::Dashboard;
                    return true;
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.export_logs();
                    return true;
                }
                KeyCode::Char(' ') => {
                    self.log_stream.paused = !self.log_stream.paused;
                    self.log_scroll_paused = self.log_stream.paused;
                    return true;
                }
                KeyCode::Char('/') => {
                    self.log_stream.search.active = true;
                    self.log_stream.search.cursor = self.log_stream.search.query.len();
                    return true;
                }
                KeyCode::Char('n') => {
                    self.jump_search_match(true, 15);
                    return true;
                }
                KeyCode::Char('N') => {
                    self.jump_search_match(false, 15);
                    return true;
                }
                KeyCode::Char('[') | KeyCode::Left => {
                    self.log_stream.filter_tab = self.log_stream.filter_tab.prev();
                    self.log_stream.scroll_offset = 0;
                    return true;
                }
                KeyCode::Char(']') | KeyCode::Right => {
                    self.log_stream.filter_tab = self.log_stream.filter_tab.next();
                    self.log_stream.scroll_offset = 0;
                    return true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_log_up(1);
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_log_down(1);
                    return true;
                }
                KeyCode::PageUp => {
                    self.scroll_log_up(10);
                    return true;
                }
                KeyCode::PageDown => {
                    self.scroll_log_down(10);
                    return true;
                }
                KeyCode::Home => {
                    self.scroll_log_to_top();
                    return true;
                }
                KeyCode::End => {
                    self.scroll_log_to_bottom();
                    return true;
                }
                _ => return false,
            }
        }

        // Export hotkey works anywhere in Dashboard
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            self.export_logs();
            return true;
        }

        // Log-pane specific keys when DashboardPane::Logs is focused
        if self.focused_pane == DashboardPane::Logs {
            match key.code {
                KeyCode::Char('/') => {
                    self.log_stream.search.active = true;
                    self.log_stream.search.cursor = self.log_stream.search.query.len();
                    return true;
                }
                KeyCode::Char(' ') => {
                    self.log_stream.paused = !self.log_stream.paused;
                    self.log_scroll_paused = self.log_stream.paused;
                    return true;
                }
                KeyCode::Char('n') => {
                    self.jump_search_match(true, 8);
                    return true;
                }
                KeyCode::Char('N') => {
                    self.jump_search_match(false, 8);
                    return true;
                }
                KeyCode::Char('[') => {
                    self.log_stream.filter_tab = self.log_stream.filter_tab.prev();
                    self.log_stream.scroll_offset = 0;
                    return true;
                }
                KeyCode::Char(']') => {
                    self.log_stream.filter_tab = self.log_stream.filter_tab.next();
                    self.log_stream.scroll_offset = 0;
                    return true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_log_up(1);
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_log_down(1);
                    return true;
                }
                KeyCode::PageUp => {
                    self.scroll_log_up(6);
                    return true;
                }
                KeyCode::PageDown => {
                    self.scroll_log_down(6);
                    return true;
                }
                KeyCode::Home => {
                    self.scroll_log_to_top();
                    return true;
                }
                KeyCode::End => {
                    self.scroll_log_to_bottom();
                    return true;
                }
                _ => {}
            }
        }

        // Global dashboard navigation
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
            KeyCode::Char('i') | KeyCode::Char('I') | KeyCode::Enter
                if self.focused_pane == DashboardPane::Players =>
            {
                if let Some(ref t) = self.telemetry {
                    if let Some(sel) = self.table_state.selected() {
                        if let Some(p) = t.players.get(sel) {
                            self.open_character_inspector(p.clone());
                            return true;
                        }
                    }
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') if self.focused_pane == DashboardPane::Players => {
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
            KeyCode::Up if self.focused_pane == DashboardPane::Players => {
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

    fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        match self.view_mode {
            DashboardView::Dashboard => self.render_dashboard(area, buf, mouse_pos),
            DashboardView::FullLog => self.render_full_log(area, buf, mouse_pos),
        }

        if self.gecho_dialog_open {
            self.render_gecho_dialog(area, buf, mouse_pos);
        }

        if self.kick_dialog_open {
            self.render_kick_dialog(area, buf, mouse_pos);
        }

        if self.shutdown_dialog_open {
            self.render_shutdown_dialog(area, buf, mouse_pos);
        }

        if self.connect_dialog.is_open {
            self.render_connect_dialog(area, buf, mouse_pos);
        }

        if self.character_inspector.is_open {
            self.render_character_inspector(area, buf, mouse_pos);
        }

        if self.character_inspector.advance_dialog_open {
            self.render_advance_dialog(area, buf, mouse_pos);
        }

        if self.character_inspector.force_cmd_dialog_open {
            self.render_force_cmd_dialog(area, buf, mouse_pos);
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

        // 3b. Advance Dialog
        if self.character_inspector.advance_dialog_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let r = self.character_inspector.advance_dialog_rect;
                if r.width > 0 {
                    if col < r.x || col >= r.x + r.width || row < r.y || row >= r.y + r.height {
                        self.character_inspector.advance_dialog_open = false;
                    } else if row == r.y + 1 + r.height.saturating_sub(2) - 1 {
                        let rects = dialog_button_rects(r, row, &ADVANCE_BUTTONS);
                        if let Some(rect) = rects.first() {
                            if col >= rect.x && col < rect.x + rect.width {
                                let lvl = self
                                    .character_inspector
                                    .advance_input
                                    .trim()
                                    .parse::<u8>()
                                    .unwrap_or(1);
                                self.submit_advance(lvl);
                            }
                        }
                        if let Some(rect) = rects.last() {
                            if col >= rect.x && col < rect.x + rect.width {
                                self.character_inspector.advance_dialog_open = false;
                            }
                        }
                    }
                }
            }
            return;
        }

        // 3c. Force Command Dialog
        if self.character_inspector.force_cmd_dialog_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let r = self.character_inspector.force_cmd_dialog_rect;
                if r.width > 0 {
                    if col < r.x || col >= r.x + r.width || row < r.y || row >= r.y + r.height {
                        self.character_inspector.force_cmd_dialog_open = false;
                    } else if row == r.y + 1 + r.height.saturating_sub(2) - 1 {
                        let rects = dialog_button_rects(r, row, &FORCE_CMD_BUTTONS);
                        if let Some(rect) = rects.first() {
                            if col >= rect.x && col < rect.x + rect.width {
                                let cmd =
                                    self.character_inspector.force_cmd_input.trim().to_string();
                                self.submit_force_command(&cmd);
                            }
                        }
                        if let Some(rect) = rects.last() {
                            if col >= rect.x && col < rect.x + rect.width {
                                self.character_inspector.force_cmd_dialog_open = false;
                            }
                        }
                    }
                }
            }
            return;
        }

        // 3d. Character Inspector Drawer
        if self.character_inspector.is_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let r = self.character_inspector.drawer_rect;
                if r.width > 0 {
                    if col < r.x || col >= r.x + r.width || row < r.y || row >= r.y + r.height {
                        self.close_character_inspector();
                        return;
                    }

                    for &(btn_id, rect) in &self.character_inspector.button_rects {
                        if col >= rect.x
                            && col < rect.x + rect.width
                            && row >= rect.y
                            && row < rect.y + rect.height
                        {
                            match btn_id {
                                "heal" => self.submit_heal(),
                                "revive" => self.submit_revive(),
                                "advance" => {
                                    self.character_inspector.advance_dialog_open = true;
                                    self.character_inspector.advance_input = "50".into();
                                    self.character_inspector.advance_cursor = 2;
                                }
                                "force_cmd" => {
                                    self.character_inspector.force_cmd_dialog_open = true;
                                    self.character_inspector.force_cmd_input.clear();
                                    self.character_inspector.force_cmd_cursor = 0;
                                }
                                "kick" => {
                                    if let Some(ref p) = self.character_inspector.player {
                                        self.kick_target = Some(p.name.clone());
                                        self.kick_dialog_open = true;
                                    }
                                }
                                "close" => self.close_character_inspector(),
                                _ => {}
                            }
                            return;
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

        match mouse.kind {
            MouseEventKind::ScrollUp
                if self.view_mode == DashboardView::FullLog || in_rect(self.logs_rect) =>
            {
                self.scroll_log_up(3);
                return;
            }
            MouseEventKind::ScrollDown
                if self.view_mode == DashboardView::FullLog || in_rect(self.logs_rect) =>
            {
                self.scroll_log_down(3);
                return;
            }
            _ => {}
        }

        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            if self.view_mode == DashboardView::FullLog || in_rect(self.logs_rect) {
                for &(tab, rect) in &self.log_stream.tab_rects {
                    if col >= rect.x
                        && col < rect.x + rect.width
                        && row >= rect.y
                        && row < rect.y + rect.height
                    {
                        self.log_stream.filter_tab = tab;
                        self.log_stream.scroll_offset = 0;
                        if self.view_mode != DashboardView::FullLog {
                            self.focused_pane = DashboardPane::Logs;
                        }
                        return;
                    }
                }
                let p_rect = self.log_stream.pause_chip_rect;
                if p_rect.width > 0
                    && col >= p_rect.x
                    && col < p_rect.x + p_rect.width
                    && row >= p_rect.y
                    && row < p_rect.y + p_rect.height
                {
                    self.log_stream.paused = !self.log_stream.paused;
                    self.log_scroll_paused = self.log_stream.paused;
                    if self.view_mode != DashboardView::FullLog {
                        self.focused_pane = DashboardPane::Logs;
                    }
                    return;
                }
                let s_rect = self.log_stream.search_chip_rect;
                if s_rect.width > 0
                    && col >= s_rect.x
                    && col < s_rect.x + s_rect.width
                    && row >= s_rect.y
                    && row < s_rect.y + s_rect.height
                {
                    self.log_stream.search.active = !self.log_stream.search.active;
                    if self.log_stream.search.active {
                        self.log_stream.search.cursor = self.log_stream.search.query.len();
                    }
                    if self.view_mode != DashboardView::FullLog {
                        self.focused_pane = DashboardPane::Logs;
                    }
                    return;
                }
            }

            if self.view_mode == DashboardView::Dashboard {
                if in_rect(self.players_rect) {
                    self.focused_pane = DashboardPane::Players;
                    let header_row = self.players_rect.y + 1;
                    let min_data_row = self.players_rect.y + 2;
                    let max_data_row =
                        self.players_rect.y + self.players_rect.height.saturating_sub(1);
                    if row == header_row {
                        let header_inner = Rect::new(
                            self.players_rect.x + 1,
                            header_row,
                            self.players_rect.width.saturating_sub(2),
                            1,
                        );
                        let col_areas = Layout::horizontal([
                            Constraint::Percentage(20),
                            Constraint::Percentage(10),
                            Constraint::Percentage(20),
                            Constraint::Percentage(25),
                            Constraint::Percentage(10),
                            Constraint::Percentage(15),
                        ])
                        .split(header_inner);
                        for (col_idx, r) in col_areas.iter().enumerate() {
                            if col >= r.x && col < r.x + r.width {
                                self.handle_player_header_click(col_idx);
                                break;
                            }
                        }
                    } else if row >= min_data_row && row < max_data_row {
                        let rel_y = row - min_data_row;
                        if let Some(ref t) = self.telemetry {
                            let sel_idx = rel_y as usize;
                            if sel_idx < t.players.len() {
                                self.table_state.select(Some(sel_idx));
                                if let Some((prev_idx, prev_time)) = self.last_player_click {
                                    if prev_idx == sel_idx && prev_time.elapsed().as_millis() < 400
                                    {
                                        if let Some(p) = t.players.get(sel_idx) {
                                            self.open_character_inspector(p.clone());
                                        }
                                        self.last_player_click = None;
                                        return;
                                    }
                                }
                                self.last_player_click = Some((sel_idx, Instant::now()));
                            }
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
    fn render_dashboard(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
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

        let mut headers = vec![
            "Player".to_string(),
            "Lvl".to_string(),
            "Class".to_string(),
            "Room".to_string(),
            "Idle".to_string(),
            "Proto".to_string(),
        ];
        if self.player_sort_column < headers.len() {
            headers[self.player_sort_column].push_str(if self.player_sort_ascending {
                " ▲"
            } else {
                " ▼"
            });
        }

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
            Row::new(headers).style(
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
        let inner_log = log_block.inner(chunks[2]);
        Widget::render(log_block, chunks[2], buf);

        if inner_log.height >= 2 {
            let filtered: Vec<&String> = self
                .logs
                .iter()
                .filter(|l| self.log_stream.filter_tab.matches(l))
                .collect();
            let total = filtered.len();

            let show_search_prompt = self.log_stream.search.active
                || (self.log_stream.toast.is_some()
                    && self
                        .log_stream
                        .toast
                        .as_ref()
                        .map(|(_, t)| t.elapsed().as_secs() < 4)
                        .unwrap_or(false));

            let log_constraints = if show_search_prompt && inner_log.height >= 3 {
                vec![
                    Constraint::Length(1), // Top control tabs
                    Constraint::Min(1),    // Log lines
                    Constraint::Length(1), // Search prompt or Toast
                ]
            } else {
                vec![
                    Constraint::Length(1), // Top control tabs
                    Constraint::Min(1),    // Log lines
                ]
            };

            let log_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(log_constraints)
                .split(inner_log);

            render_log_tab_controls(
                log_layout[0],
                buf,
                &mut self.log_stream,
                total,
                log_layout[1].height as usize,
                mouse_pos,
            );

            let visible_h = log_layout[1].height as usize;
            let max_scroll = total.saturating_sub(visible_h);
            self.log_stream.scroll_offset = self.log_stream.scroll_offset.min(max_scroll);

            let slice: &[&String] = if total == 0 {
                &[]
            } else if self.log_stream.scroll_offset == 0 {
                let start = total.saturating_sub(visible_h);
                &filtered[start..total]
            } else {
                let end = total - self.log_stream.scroll_offset;
                let start = end.saturating_sub(visible_h);
                &filtered[start..end]
            };

            let search_q = &self.log_stream.search.query;
            let items: Vec<ListItem> = slice
                .iter()
                .map(|line| ListItem::new(format_log_line(line, search_q)))
                .collect();

            Widget::render(List::new(items), log_layout[1], buf);

            if show_search_prompt && log_layout.len() > 2 {
                render_log_search_prompt(
                    log_layout[2],
                    buf,
                    &self.log_stream.search,
                    &self.log_stream.toast,
                );
            }
        }

        // --- 4. Footer Action Bar ---
        let footer_text = match self.focused_pane {
            DashboardPane::Logs => {
                " [Tab] Switch Pane • [L] Full Logs • [/] Search • [Space] Pause • [n/N] Match • [Ctrl+S] Export • [[]/[]] Tab • [C] Settings "
            }
            DashboardPane::Players => {
                " [Tab] Switch Pane • [↑↓] Select • [I/Enter] Inspect • [K] Kick • [G] Echo • [L] Logs • [S] Shutdown • [C] Settings "
            }
            DashboardPane::Metrics => {
                " [Tab] Switch Pane • [L] Full Logs • [G] Global Echo • [S] Shutdown • [C] Settings "
            }
        };
        Paragraph::new(footer_text)
            .style(Style::default().bg(theme::BG_DARK).fg(theme::FG_BRIGHT))
            .render(chunks[3], buf);
    }

    fn render_full_log(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        let show_search_prompt = self.log_stream.search.active
            || (self.log_stream.toast.is_some()
                && self
                    .log_stream
                    .toast
                    .as_ref()
                    .map(|(_, t)| t.elapsed().as_secs() < 4)
                    .unwrap_or(false));

        let constraints = if show_search_prompt {
            vec![
                Constraint::Length(3), // Header & Tab controls
                Constraint::Min(5),    // Full log stream
                Constraint::Length(1), // Search prompt or Toast
                Constraint::Length(1), // Footer bar
            ]
        } else {
            vec![
                Constraint::Length(3), // Header & Tab controls
                Constraint::Min(5),    // Full log stream
                Constraint::Length(1), // Footer bar
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        self.logs_rect = chunks[1];

        let filtered: Vec<&String> = self
            .logs
            .iter()
            .filter(|l| self.log_stream.filter_tab.matches(l))
            .collect();
        let total = filtered.len();

        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" SERVER LOG VIEW - DIAGNOSTIC STREAM ")
            .title_style(
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(theme::border_accent());
        let header_inner = header_block.inner(chunks[0]);
        Widget::render(header_block, chunks[0], buf);

        render_log_tab_controls(
            header_inner,
            buf,
            &mut self.log_stream,
            total,
            chunks[1].height.saturating_sub(2) as usize,
            mouse_pos,
        );

        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(theme::border_accent());
        let log_inner = log_block.inner(chunks[1]);
        Widget::render(log_block, chunks[1], buf);

        let visible_h = log_inner.height as usize;
        let max_scroll = total.saturating_sub(visible_h);
        self.log_stream.scroll_offset = self.log_stream.scroll_offset.min(max_scroll);

        let slice: &[&String] = if total == 0 {
            &[]
        } else if self.log_stream.scroll_offset == 0 {
            let start = total.saturating_sub(visible_h);
            &filtered[start..total]
        } else {
            let end = total - self.log_stream.scroll_offset;
            let start = end.saturating_sub(visible_h);
            &filtered[start..end]
        };

        let search_q = &self.log_stream.search.query;
        let items: Vec<ListItem> = slice
            .iter()
            .map(|line| ListItem::new(format_log_line(line, search_q)))
            .collect();

        Widget::render(List::new(items), log_inner, buf);

        let footer_area = if show_search_prompt {
            render_log_search_prompt(
                chunks[2],
                buf,
                &self.log_stream.search,
                &self.log_stream.toast,
            );
            chunks[3]
        } else {
            chunks[2]
        };

        let footer_text = " [Tab/Esc] Dashboard • [Space] Pause Stream • [/] Search • [n/N] Next/Prev Match • [Ctrl+S] Export • [[]/[]] Filter Tab ";
        Paragraph::new(footer_text)
            .style(
                Style::default()
                    .bg(theme::PRIMARY)
                    .fg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            )
            .render(footer_area, buf);
    }
}

pub fn split_highlight(
    text: &str,
    query: &str,
    base_style: Style,
    highlight_style: Style,
    spans: &mut Vec<Span<'static>>,
) {
    let trimmed_q = query.trim();
    if trimmed_q.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
        return;
    }
    let text_lower = text.to_lowercase();
    let q_lower = trimmed_q.to_lowercase();
    let mut cursor = 0;
    while let Some(rel_pos) = text_lower[cursor..].find(&q_lower) {
        let match_start = cursor + rel_pos;
        let match_end = match_start + q_lower.len();
        if match_start > cursor {
            spans.push(Span::styled(
                text[cursor..match_start].to_string(),
                base_style,
            ));
        }
        spans.push(Span::styled(
            text[match_start..match_end].to_string(),
            highlight_style,
        ));
        cursor = match_end;
    }
    if cursor < text.len() {
        spans.push(Span::styled(text[cursor..].to_string(), base_style));
    }
}

pub fn format_log_line(line: &str, search_query: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let highlight_style = Style::default()
        .bg(theme::PRIMARY)
        .fg(theme::BG)
        .add_modifier(Modifier::BOLD);

    let upper = line.to_uppercase();
    let is_error = upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("CRITICAL");
    let is_warn = !is_error && upper.contains("WARN");
    let is_info = !is_error && !is_warn && upper.contains("INFO");

    let trimmed = line.trim_start();
    let mut rest = trimmed;

    // 1. Check for bracketed or bare timestamp
    if rest.starts_with('[') {
        if let Some(close_bracket) = rest.find(']') {
            let inside = &rest[1..close_bracket];
            if inside.chars().any(|c| c.is_ascii_digit())
                && (inside.contains(':') || inside.contains('-'))
            {
                let tag = &rest[..=close_bracket];
                split_highlight(
                    tag,
                    search_query,
                    Style::default().fg(theme::FG_MUTED),
                    highlight_style,
                    &mut spans,
                );
                spans.push(Span::raw(" "));
                rest = rest[close_bracket + 1..].trim_start();
            }
        }
    } else if rest.len() >= 19
        && (rest.chars().nth(4) == Some('-') || rest.chars().nth(2) == Some(':'))
    {
        if let Some(space_idx) = rest.find(' ') {
            let ts = &rest[..space_idx];
            split_highlight(
                ts,
                search_query,
                Style::default().fg(theme::FG_MUTED),
                highlight_style,
                &mut spans,
            );
            spans.push(Span::raw(" "));
            rest = rest[space_idx + 1..].trim_start();
        }
    }

    // 2. Check for bracketed tag [TAG]
    if rest.starts_with('[') {
        if let Some(close_bracket) = rest.find(']') {
            let tag = &rest[..=close_bracket];
            let tag_style = if is_error {
                Style::default()
                    .fg(theme::DANGER)
                    .add_modifier(Modifier::BOLD)
            } else if is_warn {
                Style::default()
                    .fg(theme::WARNING)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD)
            };
            split_highlight(tag, search_query, tag_style, highlight_style, &mut spans);
            spans.push(Span::raw(" "));
            rest = rest[close_bracket + 1..].trim_start();
        }
    }

    // 3. Body text styling
    let body_style = if is_error {
        Style::default()
            .fg(theme::DANGER)
            .add_modifier(Modifier::BOLD)
    } else if is_warn {
        Style::default().fg(theme::WARNING)
    } else if is_info {
        Style::default().fg(theme::POSITIVE)
    } else {
        Style::default().fg(theme::FG_BRIGHT)
    };

    split_highlight(rest, search_query, body_style, highlight_style, &mut spans);

    Line::from(spans)
}

fn render_log_tab_controls(
    area: Rect,
    buf: &mut Buffer,
    log_stream: &mut LogStreamState,
    total_lines: usize,
    _visible_height: usize,
    mouse_pos: Option<(u16, u16)>,
) {
    if area.width < 10 || area.height < 1 {
        return;
    }
    log_stream.tab_rects.clear();

    // 1. Render Tabs on the left
    let mut x = area.x;
    for &tab in LogFilterTab::all() {
        let btn = Button::new(tab.label(), ButtonVariant::Default);
        let w = btn.width();
        if x + w > area.x + area.width {
            break;
        }
        let is_hovered =
            mouse_pos.is_some_and(|(col, row)| row == area.y && col >= x && col < x + w);
        let state = if tab == log_stream.filter_tab || is_hovered {
            ButtonState::Active
        } else {
            ButtonState::Inactive
        };
        let tab_rect = btn.render(buf, x, area.y, state);
        log_stream.tab_rects.push((tab, tab_rect));
        x += w + 2;
    }

    // 2. Right-aligned status chips
    let is_chip_hovered = |rx: u16, rw: u16| {
        mouse_pos.is_some_and(|(col, row)| row == area.y && col >= rx && col < rx + rw)
    };

    let pause_btn = if log_stream.paused {
        Button::new("⏸ PAUSED", ButtonVariant::Warning)
    } else {
        Button::new("● LIVE", ButtonVariant::Default)
    };
    let pause_w = pause_btn.width();

    let scroll_text = if total_lines == 0 {
        "Empty".to_string()
    } else if log_stream.scroll_offset == 0 {
        format!("{total_lines}/{total_lines}")
    } else {
        let current = total_lines.saturating_sub(log_stream.scroll_offset);
        format!("{current}/{total_lines} ↑{}L", log_stream.scroll_offset)
    };
    let scroll_chip = Button::new(scroll_text, ButtonVariant::Neutral);
    let scroll_w = scroll_chip.width();

    let search_text = if !log_stream.search.query.trim().is_empty() {
        let count = log_stream.search.matches.len();
        if count > 0 {
            format!(
                "Search (/): \"{}\" ({}/{})",
                log_stream.search.query,
                log_stream.search.selected_match + 1,
                count
            )
        } else {
            format!("Search (/): \"{}\" (0)", log_stream.search.query)
        }
    } else {
        "Search (/)".to_string()
    };
    let search_variant = if log_stream.search.active || !log_stream.search.query.trim().is_empty() {
        ButtonVariant::Default
    } else {
        ButtonVariant::Neutral
    };
    let search_btn = Button::new(search_text, search_variant);
    let search_w = search_btn.width();

    let total_right_w = pause_w + 2 + scroll_w + 2 + search_w;
    if area.width > total_right_w + 2 {
        let mut rx = area.x + area.width - pause_w;
        let p_hover = is_chip_hovered(rx, pause_w);
        let pause_state = if log_stream.paused || p_hover {
            ButtonState::Active
        } else {
            ButtonState::Inactive
        };
        log_stream.pause_chip_rect = pause_btn.render(buf, rx, area.y, pause_state);

        rx = rx.saturating_sub(scroll_w + 2);
        let _ = scroll_chip.render(buf, rx, area.y, ButtonState::Inactive);

        rx = rx.saturating_sub(search_w + 2);
        let s_hover = is_chip_hovered(rx, search_w);
        let search_state = if log_stream.search.active || s_hover {
            ButtonState::Active
        } else {
            ButtonState::Inactive
        };
        log_stream.search_chip_rect = search_btn.render(buf, rx, area.y, search_state);
    }
}

fn render_log_search_prompt(
    area: Rect,
    buf: &mut Buffer,
    search: &LogSearchState,
    toast: &Option<(String, Instant)>,
) {
    if area.width < 10 || area.height < 1 {
        return;
    }
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_char(' ');
            cell.set_style(Style::default().bg(theme::BG_DARK));
        }
    }

    if let Some((ref msg, instant)) = toast {
        if instant.elapsed().as_secs() < 4 {
            let toast_text = format!(" {msg} ");
            let toast_style = Style::default()
                .bg(theme::POSITIVE)
                .fg(theme::BG)
                .add_modifier(Modifier::BOLD);
            for (i, ch) in toast_text.chars().enumerate() {
                if let Some(cell) = buf.cell_mut((area.x + i as u16, area.y)) {
                    cell.set_char(ch);
                    cell.set_style(toast_style);
                }
            }
            return;
        }
    }

    let prompt_prefix = " > ";
    let prefix_style = Style::default()
        .fg(theme::PRIMARY)
        .add_modifier(Modifier::BOLD)
        .bg(theme::BG_DARK);
    for (i, ch) in prompt_prefix.chars().enumerate() {
        if let Some(cell) = buf.cell_mut((area.x + i as u16, area.y)) {
            cell.set_char(ch);
            cell.set_style(prefix_style);
        }
    }

    let input_x = area.x + prompt_prefix.len() as u16;
    let query = &search.query;
    let cursor = search.cursor;

    if query.is_empty() {
        let placeholder = "Search logs... (Enter to filter, Esc to dismiss, n/N for matches)";
        let ph_style = Style::default().fg(theme::FG_MUTED).bg(theme::BG_DARK);
        for (i, ch) in placeholder.chars().enumerate() {
            if input_x + i as u16 >= area.x + area.width {
                break;
            }
            if let Some(cell) = buf.cell_mut((input_x + i as u16, area.y)) {
                cell.set_char(ch);
                cell.set_style(ph_style);
            }
        }
        if let Some(cell) = buf.cell_mut((input_x, area.y)) {
            cell.set_char('█');
            cell.set_style(Style::default().fg(theme::PRIMARY).bg(theme::BG_DARK));
        }
    } else {
        let mut draw_x = input_x;
        for (byte_idx, ch) in query.char_indices() {
            if draw_x >= area.x + area.width {
                break;
            }
            let is_cursor = byte_idx == cursor;
            let style = if is_cursor {
                Style::default()
                    .bg(theme::PRIMARY)
                    .fg(theme::BG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::FG).bg(theme::BG_DARK)
            };
            if let Some(cell) = buf.cell_mut((draw_x, area.y)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
            draw_x += 1;
        }
        if cursor >= query.len() && draw_x < area.x + area.width {
            if let Some(cell) = buf.cell_mut((draw_x, area.y)) {
                cell.set_char('█');
                cell.set_style(Style::default().fg(theme::PRIMARY).bg(theme::BG_DARK));
            }
        }

        let match_info = if search.matches.is_empty() {
            " [No matches] ".to_string()
        } else {
            format!(
                " [Match {} of {}] ",
                search.selected_match + 1,
                search.matches.len()
            )
        };
        let info_w = match_info.chars().count() as u16;
        if area.width > info_w + 5 {
            let info_x = area.x + area.width - info_w;
            let info_style = if search.matches.is_empty() {
                Style::default().fg(theme::WARNING).bg(theme::BG_DARK)
            } else {
                Style::default()
                    .fg(theme::PRIMARY)
                    .bg(theme::BG_DARK)
                    .add_modifier(Modifier::BOLD)
            };
            for (i, ch) in match_info.chars().enumerate() {
                if let Some(cell) = buf.cell_mut((info_x + i as u16, area.y)) {
                    cell.set_char(ch);
                    cell.set_style(info_style);
                }
            }
        }
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

const CONNECT_BUTTONS: [(&str, theme::ButtonVariant); 2] = [
    ("Connect (Enter)", theme::ButtonVariant::Default),
    ("Cancel (Esc)", theme::ButtonVariant::Neutral),
];

const KICK_BUTTONS: [(&str, theme::ButtonVariant); 2] = [
    ("Confirm Kick (y)", theme::ButtonVariant::Destructive),
    ("Cancel (n)", theme::ButtonVariant::Neutral),
];

const GECHO_BUTTONS: [(&str, theme::ButtonVariant); 2] = [
    ("Send (Enter)", theme::ButtonVariant::Default),
    ("Cancel (Esc)", theme::ButtonVariant::Neutral),
];

const SHUTDOWN_BUTTONS: [(&str, theme::ButtonVariant); 2] = [
    ("Confirm (y)", theme::ButtonVariant::Destructive),
    ("Cancel (n)", theme::ButtonVariant::Neutral),
];

const ADVANCE_BUTTONS: [(&str, theme::ButtonVariant); 2] = [
    ("Advance (Enter)", theme::ButtonVariant::Default),
    ("Cancel (Esc)", theme::ButtonVariant::Neutral),
];

const FORCE_CMD_BUTTONS: [(&str, theme::ButtonVariant); 2] = [
    ("Execute (Enter)", theme::ButtonVariant::Default),
    ("Cancel (Esc)", theme::ButtonVariant::Neutral),
];

fn dialog_button_rects(
    dialog_area: Rect,
    row_y: u16,
    buttons: &[(&str, theme::ButtonVariant)],
) -> Vec<Rect> {
    let btn_gap = 2u16;
    let row_w: u16 = buttons
        .iter()
        .map(|(label, _)| label.chars().count() as u16 + 2)
        .sum::<u16>()
        + btn_gap * (buttons.len().saturating_sub(1)) as u16;
    let mut x = dialog_area.x + dialog_area.width.saturating_sub(row_w + 2);
    let mut rects = Vec::with_capacity(buttons.len());
    for (label, _) in buttons {
        let width = label.chars().count() as u16 + 2;
        rects.push(Rect::new(x, row_y, width, 1));
        x += width + btn_gap;
    }
    rects
}

fn render_dialog_buttons(
    buf: &mut Buffer,
    dialog_area: Rect,
    row_y: u16,
    buttons: &[(&str, theme::ButtonVariant)],
    default_active_idx: usize,
    mouse_pos: Option<(u16, u16)>,
) -> Vec<Rect> {
    let rects = dialog_button_rects(dialog_area, row_y, buttons);
    let any_hovered = mouse_pos.is_some_and(|(col, row)| {
        rects
            .iter()
            .any(|r| row == r.y && col >= r.x && col < r.x + r.width)
    });
    for (i, ((label, variant), rect)) in buttons.iter().zip(&rects).enumerate() {
        let is_hovered = mouse_pos
            .is_some_and(|(col, row)| row == rect.y && col >= rect.x && col < rect.x + rect.width);
        let state = if is_hovered || (!any_hovered && i == default_active_idx) {
            theme::ButtonState::Active
        } else {
            theme::ButtonState::Inactive
        };
        Button::new(*label, *variant).render(buf, rect.x, rect.y, state);
    }
    rects
}

impl LiveDashboardScreen {
    fn render_kick_dialog(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
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
        render_dialog_buttons(buf, dialog_area, row_y, &KICK_BUTTONS, 1, mouse_pos);
    }

    fn render_shutdown_dialog(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        mouse_pos: Option<(u16, u16)>,
    ) {
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
        render_dialog_buttons(buf, dialog_area, row_y, &SHUTDOWN_BUTTONS, 1, mouse_pos);
    }

    fn render_gecho_dialog(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
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
        render_dialog_buttons(buf, dialog_area, row_y, &GECHO_BUTTONS, 0, mouse_pos);
    }

    fn render_connect_dialog(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        mouse_pos: Option<(u16, u16)>,
    ) {
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

        // Footer buttons: Connect and Cancel (right-aligned)
        let btn_y = inner_y + 12;
        render_dialog_buttons(buf, dialog_area, btn_y, &CONNECT_BUTTONS, 0, mouse_pos);

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

    fn render_advance_dialog(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        mouse_pos: Option<(u16, u16)>,
    ) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        self.character_inspector.advance_dialog_rect = dialog_area;
        let target_name = self
            .character_inspector
            .player
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("Player");
        clear_and_fill_dialog(
            dialog_area,
            &format!("Advance {target_name} Level"),
            theme::PRIMARY,
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
            "Target level (1 - 50):",
            Style::default().fg(theme::FG).bg(theme::BG_DARK),
        );

        let prompt_x = inner.x;
        let value_x = prompt_x + 3;
        buf.set_string(prompt_x, inner.y + 1, " > ", theme::prompt_style());
        if self.character_inspector.advance_input.is_empty() {
            buf.set_string(value_x, inner.y + 1, "█", theme::prompt_style());
        } else {
            let cursor = self
                .character_inspector
                .advance_cursor
                .min(self.character_inspector.advance_input.len());
            let before = &self.character_inspector.advance_input[..cursor];
            let after = &self.character_inspector.advance_input[cursor..];
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
        render_dialog_buttons(buf, dialog_area, row_y, &ADVANCE_BUTTONS, 0, mouse_pos);
    }

    fn render_force_cmd_dialog(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        mouse_pos: Option<(u16, u16)>,
    ) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        self.character_inspector.force_cmd_dialog_rect = dialog_area;
        let target_name = self
            .character_inspector
            .player
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("Player");
        clear_and_fill_dialog(
            dialog_area,
            &format!("Force Command on {target_name}"),
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
            "Enter command to execute as target:",
            Style::default().fg(theme::FG).bg(theme::BG_DARK),
        );

        let prompt_x = inner.x;
        let value_x = prompt_x + 3;
        buf.set_string(prompt_x, inner.y + 1, " > ", theme::prompt_style());
        if self.character_inspector.force_cmd_input.is_empty() {
            buf.set_string(value_x, inner.y + 1, "█", theme::prompt_style());
        } else {
            let cursor = self
                .character_inspector
                .force_cmd_cursor
                .min(self.character_inspector.force_cmd_input.len());
            let before = &self.character_inspector.force_cmd_input[..cursor];
            let after = &self.character_inspector.force_cmd_input[cursor..];
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
        render_dialog_buttons(buf, dialog_area, row_y, &FORCE_CMD_BUTTONS, 0, mouse_pos);
    }

    fn render_character_inspector(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        mouse_pos: Option<(u16, u16)>,
    ) {
        let Some(ref p) = self.character_inspector.player else {
            return;
        };

        // Centered modal dialog per Spade UI/UX Design Guide
        let dialog_width = 56.min(area.width);
        let dialog_height = 26.min(area.height);
        let dialog_rect = Rect {
            x: area.x + (area.width.saturating_sub(dialog_width)) / 2,
            y: area.y + (area.height.saturating_sub(dialog_height)) / 2,
            width: dialog_width,
            height: dialog_height,
        };
        self.character_inspector.drawer_rect = dialog_rect;

        let title_str = format!("CHARACTER: {}", p.name);
        clear_and_fill_dialog(dialog_rect, &title_str, theme::PRIMARY, buf);

        if dialog_rect.width < 25 || dialog_rect.height < 15 {
            return;
        }

        let inner_x = dialog_rect.x + 2;
        let mut curr_y = dialog_rect.y + 1;
        let avail_w = dialog_rect.width.saturating_sub(4);

        // 1. Identity section
        buf.set_string(
            inner_x,
            curr_y,
            "Identity",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        );
        curr_y += 1;

        let id_style_label = Style::default().fg(theme::FG_MUTED);
        let id_style_val = Style::default().fg(theme::FG_BRIGHT);

        if avail_w >= 36 {
            buf.set_string(inner_x, curr_y, "Level: ", id_style_label);
            buf.set_string(inner_x + 7, curr_y, format!("{}", p.level), id_style_val);
            buf.set_string(inner_x + 14, curr_y, "Class: ", id_style_label);
            buf.set_string(inner_x + 21, curr_y, &p.class, id_style_val);
            curr_y += 1;

            buf.set_string(inner_x, curr_y, "Race:  ", id_style_label);
            buf.set_string(inner_x + 7, curr_y, &p.race, id_style_val);
            buf.set_string(inner_x + 14, curr_y, "Proto: ", id_style_label);
            buf.set_string(inner_x + 21, curr_y, &p.protocol, id_style_val);
            curr_y += 1;
        } else {
            let col2_x = inner_x + avail_w / 2;
            buf.set_string(inner_x, curr_y, "Lvl: ", id_style_label);
            buf.set_string(inner_x + 5, curr_y, format!("{}", p.level), id_style_val);
            buf.set_string(col2_x, curr_y, "Cls: ", id_style_label);
            let cls_max = (avail_w.saturating_sub(avail_w / 2 + 5) as usize).max(1);
            let cls_disp: String = p.class.chars().take(cls_max).collect();
            buf.set_string(col2_x + 5, curr_y, &cls_disp, id_style_val);
            curr_y += 1;

            buf.set_string(inner_x, curr_y, "Race: ", id_style_label);
            let race_max = ((avail_w / 2).saturating_sub(6) as usize).max(1);
            let race_disp: String = p.race.chars().take(race_max).collect();
            buf.set_string(inner_x + 6, curr_y, &race_disp, id_style_val);
            buf.set_string(col2_x, curr_y, "Proto: ", id_style_label);
            let proto_max = (avail_w.saturating_sub(avail_w / 2 + 7) as usize).max(1);
            let proto_disp: String = p.protocol.chars().take(proto_max).collect();
            buf.set_string(col2_x + 7, curr_y, &proto_disp, id_style_val);
            curr_y += 1;
        }

        buf.set_string(inner_x, curr_y, "Room:  ", id_style_label);
        let max_room_chars = (avail_w.saturating_sub(7) as usize).max(1);
        let room_disp = if p.room.chars().count() > max_room_chars {
            let truncated: String = p
                .room
                .chars()
                .take(max_room_chars.saturating_sub(1))
                .collect();
            format!("{truncated}…")
        } else {
            p.room.clone()
        };
        buf.set_string(inner_x + 7, curr_y, &room_disp, id_style_val);
        curr_y += 1;

        buf.set_string(inner_x, curr_y, "Idle:  ", id_style_label);
        buf.set_string(
            inner_x + 7,
            curr_y,
            format!("{}s", p.idle_secs),
            id_style_val,
        );
        curr_y += 2;

        // 2. Vitals gauges
        buf.set_string(
            inner_x,
            curr_y,
            "Vitals",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        );
        curr_y += 1;

        // HP Gauge
        let (hp_cur, hp_max) = self
            .character_inspector
            .health
            .map(|v| (v.current, v.max))
            .unwrap_or((100, 100));
        let hp_ratio = if hp_max > 0 {
            (hp_cur as f64 / hp_max as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let hp_color = if hp_ratio > 0.5 {
            theme::POSITIVE
        } else if hp_ratio > 0.25 {
            theme::WARNING
        } else {
            theme::DANGER
        };
        let hp_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(hp_color).bg(theme::PANEL))
            .ratio(hp_ratio)
            .label(format!(
                "HP: {hp_cur}/{hp_max} ({}%)",
                (hp_ratio * 100.0) as u32
            ));
        hp_gauge.render(Rect::new(inner_x, curr_y, avail_w, 1), buf);
        curr_y += 1;

        // Mana Gauge
        let (mp_cur, mp_max) = self
            .character_inspector
            .mana
            .map(|v| (v.current, v.max))
            .unwrap_or((50, 50));
        let mp_ratio = if mp_max > 0 {
            (mp_cur as f64 / mp_max as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mp_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(theme::PRIMARY).bg(theme::PANEL))
            .ratio(mp_ratio)
            .label(format!(
                "MP: {mp_cur}/{mp_max} ({}%)",
                (mp_ratio * 100.0) as u32
            ));
        mp_gauge.render(Rect::new(inner_x, curr_y, avail_w, 1), buf);
        curr_y += 1;

        // Stamina Gauge
        let (sp_cur, sp_max) = self
            .character_inspector
            .stamina
            .map(|v| (v.current, v.max))
            .unwrap_or((100, 100));
        let sp_ratio = if sp_max > 0 {
            (sp_cur as f64 / sp_max as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let sp_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(theme::WARNING).bg(theme::PANEL))
            .ratio(sp_ratio)
            .label(format!(
                "SP: {sp_cur}/{sp_max} ({}%)",
                (sp_ratio * 100.0) as u32
            ));
        sp_gauge.render(Rect::new(inner_x, curr_y, avail_w, 1), buf);
        curr_y += 2;

        // 3. Attributes Section
        buf.set_string(
            inner_x,
            curr_y,
            "Attributes",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        );
        curr_y += 1;

        let attrs = self
            .character_inspector
            .attributes
            .unwrap_or(CharacterAttributes {
                strength: 10,
                dexterity: 10,
                constitution: 10,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
            });

        let attr_lbl = Style::default().fg(theme::FG_MUTED);
        let attr_val = Style::default()
            .fg(theme::PRIMARY)
            .add_modifier(Modifier::BOLD);

        let col1_x = inner_x;
        let col2_x = inner_x + avail_w / 2;

        buf.set_string(col1_x, curr_y, "STR: ", attr_lbl);
        buf.set_string(
            col1_x + 5,
            curr_y,
            format!("{:<3}", attrs.strength),
            attr_val,
        );
        buf.set_string(col2_x, curr_y, "INT: ", attr_lbl);
        buf.set_string(
            col2_x + 5,
            curr_y,
            format!("{:<3}", attrs.intelligence),
            attr_val,
        );
        curr_y += 1;

        buf.set_string(col1_x, curr_y, "DEX: ", attr_lbl);
        buf.set_string(
            col1_x + 5,
            curr_y,
            format!("{:<3}", attrs.dexterity),
            attr_val,
        );
        buf.set_string(col2_x, curr_y, "WIS: ", attr_lbl);
        buf.set_string(col2_x + 5, curr_y, format!("{:<3}", attrs.wisdom), attr_val);
        curr_y += 1;

        buf.set_string(col1_x, curr_y, "CON: ", attr_lbl);
        buf.set_string(
            col1_x + 5,
            curr_y,
            format!("{:<3}", attrs.constitution),
            attr_val,
        );
        buf.set_string(col2_x, curr_y, "CHA: ", attr_lbl);
        buf.set_string(
            col2_x + 5,
            curr_y,
            format!("{:<3}", attrs.charisma),
            attr_val,
        );
        curr_y += 2;

        // 4. Immortal Actions Chips
        buf.set_string(
            inner_x,
            curr_y,
            "Immortal Actions",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        );
        curr_y += 1;

        self.character_inspector.button_rects.clear();

        let action_buttons = [
            (
                "heal",
                Button::new("Heal (h)", theme::ButtonVariant::Default),
            ),
            (
                "revive",
                Button::new("Revive (r)", theme::ButtonVariant::Default),
            ),
            (
                "advance",
                Button::new("Advance (a)", theme::ButtonVariant::Neutral),
            ),
            (
                "force_cmd",
                Button::new("Force Cmd (m)", theme::ButtonVariant::Neutral),
            ),
            (
                "kick",
                Button::new("Kick (k)", theme::ButtonVariant::Destructive),
            ),
        ];

        let mut x = inner_x;
        let mut row_y = curr_y;
        for (action, btn) in action_buttons {
            if x > inner_x && x + btn.width() > inner_x + avail_w {
                x = inner_x;
                row_y += 2;
            }
            let is_hovered = mouse_pos
                .is_some_and(|(col, row)| row == row_y && col >= x && col < x + btn.width());
            let state = if is_hovered {
                theme::ButtonState::Active
            } else {
                theme::ButtonState::Inactive
            };
            let rect = btn.render(buf, x, row_y, state);
            self.character_inspector.button_rects.push((action, rect));
            x += btn.width() + 2;
        }

        // 5. Standard dialog button footer: right-aligned Close button
        let footer_y = dialog_rect.y + dialog_rect.height.saturating_sub(2);
        let close_rects = render_dialog_buttons(
            buf,
            dialog_rect,
            footer_y,
            &[("Close (Esc)", theme::ButtonVariant::Neutral)],
            0,
            mouse_pos,
        );
        if let Some(&r) = close_rects.first() {
            self.character_inspector.button_rects.push(("close", r));
        }
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
                ("Connect (Enter)", crate::theme::ButtonVariant::Default),
                ("Cancel (Esc)", crate::theme::ButtonVariant::Neutral),
            ],
            0,
            None,
        );
        assert_eq!(buf[(w - 1, 1)].symbol(), "│");
        assert_eq!(buf[(w - 2, 1)].symbol(), " ");
    }

    #[test]
    fn test_buttons_conform_to_design_guide() {
        // Verify 2-space gap and 1-space padding per side in dialog buttons
        let dialog_area = Rect::new(0, 0, 80, 5);
        let rects = dialog_button_rects(dialog_area, 3, &CONNECT_BUTTONS);
        assert_eq!(rects.len(), 2);
        // Connect (Enter) is 15 chars + 2 padding = 17 width
        assert_eq!(rects[0].width, 17);
        // Cancel (Esc) is 12 chars + 2 padding = 14 width
        assert_eq!(rects[1].width, 14);
        // Exactly 2-space gap between buttons
        assert_eq!(rects[1].x - (rects[0].x + rects[0].width), 2);

        // Verify rendered buffer has NO bracket characters for buttons
        let mut buf = Buffer::empty(dialog_area);
        render_dialog_buttons(&mut buf, dialog_area, 3, &CONNECT_BUTTONS, 0, None);
        for x in dialog_area.x..dialog_area.x + dialog_area.width {
            let ch = buf[(x, 3)].symbol();
            assert_ne!(ch, "[", "Found bracket '[' in button row at x={x}");
            assert_ne!(ch, "]", "Found bracket ']' in button row at x={x}");
        }

        // Test hover state activation:
        // 1. Hovering Connect button (Default variant) activates PRIMARY bg
        let mut buf_hover0 = Buffer::empty(dialog_area);
        render_dialog_buttons(
            &mut buf_hover0,
            dialog_area,
            3,
            &CONNECT_BUTTONS,
            0,
            Some((rects[0].x, 3)),
        );
        let cell0 = &buf_hover0[(rects[0].x + 1, 3)];
        assert_eq!(cell0.style().bg, Some(theme::PRIMARY));

        // 2. Hovering Cancel button (Neutral variant) activates FG_MUTED bg
        let mut buf_hover1 = Buffer::empty(dialog_area);
        render_dialog_buttons(
            &mut buf_hover1,
            dialog_area,
            3,
            &CONNECT_BUTTONS,
            0,
            Some((rects[1].x, 3)),
        );
        let cell1 = &buf_hover1[(rects[1].x + 1, 3)];
        assert_eq!(cell1.style().bg, Some(theme::FG_MUTED));
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

    #[test]
    fn test_log_stream_severity_filter_tabs() {
        assert_eq!(LogFilterTab::all().len(), 6);
        assert_eq!(LogFilterTab::All.next(), LogFilterTab::Errors);
        assert_eq!(LogFilterTab::All.prev(), LogFilterTab::Game);

        let err_line = "[2026-09-06] [SERVER] ERROR Database connection timed out";
        let warn_line = "[2026-09-06] [SERVER] WARN Memory threshold exceeded";
        let info_line = "[2026-09-06] [SERVER] INFO World tick completed in 2ms";
        let net_line = "[2026-09-06] [NETWORK] New WebSocket client connected from 127.0.0.1";
        let combat_line = "[2026-09-06] [COMBAT] Player attacked Goblin for 15 damage";

        assert!(LogFilterTab::All.matches(err_line));
        assert!(LogFilterTab::All.matches(warn_line));
        assert!(LogFilterTab::All.matches(info_line));
        assert!(LogFilterTab::All.matches(net_line));
        assert!(LogFilterTab::All.matches(combat_line));

        assert!(LogFilterTab::Errors.matches(err_line));
        assert!(!LogFilterTab::Errors.matches(info_line));

        assert!(LogFilterTab::Warnings.matches(warn_line));
        assert!(!LogFilterTab::Warnings.matches(err_line));

        assert!(LogFilterTab::Info.matches(info_line));
        assert!(!LogFilterTab::Info.matches(err_line));

        assert!(LogFilterTab::NetworkRpc.matches(net_line));
        assert!(!LogFilterTab::NetworkRpc.matches(combat_line));

        assert!(LogFilterTab::Game.matches(combat_line));
        assert!(!LogFilterTab::Game.matches(net_line));
    }

    #[test]
    fn test_log_stream_scrollback_and_pause() {
        let mut screen = LiveDashboardScreen::new();
        for i in 0..50 {
            screen.add_log(format!("Log message line {i}"));
        }
        assert_eq!(screen.logs.len(), 50);
        assert_eq!(screen.log_stream.scroll_offset, 0);
        assert!(!screen.log_stream.paused);

        // Scroll up
        screen.scroll_log_up(5);
        assert_eq!(screen.log_stream.scroll_offset, 5);
        assert!(screen.log_stream.paused);

        // Scroll down
        screen.scroll_log_down(2);
        assert_eq!(screen.log_stream.scroll_offset, 3);
        assert!(screen.log_stream.paused);

        // Scroll to bottom
        screen.scroll_log_to_bottom();
        assert_eq!(screen.log_stream.scroll_offset, 0);
        assert!(!screen.log_stream.paused);

        // Scroll to top
        screen.scroll_log_to_top();
        assert_eq!(screen.log_stream.scroll_offset, 50);
        assert!(screen.log_stream.paused);
    }

    #[test]
    fn test_log_search_matching_and_navigation() {
        let mut screen = LiveDashboardScreen::new();
        screen.add_log("alpha error 1".into());
        screen.add_log("beta warning 2".into());
        screen.add_log("gamma error 3".into());
        screen.add_log("delta info 4".into());
        screen.add_log("epsilon error 5".into());

        // Focus logs pane and press '/' to open search
        screen.focused_pane = DashboardPane::Logs;
        screen.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(screen.log_stream.search.active);

        // Type "error"
        for c in "error".chars() {
            screen.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(screen.log_stream.search.query, "error");
        assert_eq!(screen.log_stream.search.matches.len(), 3); // lines 0, 2, 4

        // Press 'n' to navigate matches
        screen.jump_search_match(true, 5);
        assert_eq!(screen.log_stream.search.selected_match, 1);

        // Press 'N' to navigate backwards
        screen.jump_search_match(false, 5);
        assert_eq!(screen.log_stream.search.selected_match, 0);

        // Commit search with Enter
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!screen.log_stream.search.active);
        assert_eq!(screen.log_stream.search.query, "error");
    }

    #[test]
    fn test_format_log_line_highlighting() {
        let line = "[2026-09-06] [NETWORK] ERROR connection failed";
        let formatted = format_log_line(line, "fail");
        // Spans: timestamp, tag, level/body with query match
        assert!(!formatted.spans.is_empty());
        let full_text: String = formatted.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(full_text.contains("connection failed"));

        // Match span should have bold styling
        let match_span = formatted.spans.iter().find(|s| s.content == "fail");
        assert!(match_span.is_some());
    }

    #[test]
    fn test_log_mouse_scroll_and_tab_clicks() {
        let mut screen = LiveDashboardScreen::new();
        for i in 0..30 {
            screen.add_log(format!("[NETWORK] Client packet {i}"));
        }

        let area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf, None);

        // Verify tabs were registered in tab_rects
        assert!(!screen.log_stream.tab_rects.is_empty());

        // Click on 2nd tab (Errors)
        let errors_tab_rect = screen.log_stream.tab_rects[1].1;
        let click_errors = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: errors_tab_rect.x + 1,
            row: errors_tab_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_errors, area);
        assert_eq!(screen.log_stream.filter_tab, LogFilterTab::Errors);

        // Click back on 1st tab (All) to have logs to scroll
        let all_tab_rect = screen.log_stream.tab_rects[0].1;
        let click_all = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: all_tab_rect.x + 1,
            row: all_tab_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_all, area);
        assert_eq!(screen.log_stream.filter_tab, LogFilterTab::All);

        // Mouse Wheel Scroll Up over logs_rect
        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: screen.logs_rect.x + 2,
            row: screen.logs_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(scroll_up, area);
        assert_eq!(screen.log_stream.scroll_offset, 3);
        assert!(screen.log_stream.paused);

        // Mouse Wheel Scroll Down
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: screen.logs_rect.x + 2,
            row: screen.logs_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(scroll_down, area);
        assert_eq!(screen.log_stream.scroll_offset, 0);

        // Click on pause chip toggles pause
        screen.log_stream.paused = false;
        screen.log_scroll_paused = false;
        let pause_rect = screen.log_stream.pause_chip_rect;
        if pause_rect.width > 0 {
            let click_pause = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: pause_rect.x + 1,
                row: pause_rect.y,
                modifiers: KeyModifiers::NONE,
            };
            screen.handle_mouse(click_pause, area);
            assert!(screen.log_stream.paused);

            screen.handle_mouse(click_pause, area);
            assert!(!screen.log_stream.paused);
        }
    }

    #[test]
    fn test_log_export_to_file() {
        let mut screen = LiveDashboardScreen::new();
        screen.add_log("Line 1 test export".into());
        screen.add_log("Line 2 test export".into());

        screen.export_logs();
        assert!(screen.log_stream.toast.is_some());
        let toast = screen.log_stream.toast.as_ref().unwrap().0.clone();
        assert!(toast.contains("Exported 2 lines to spade-logs-"));

        // Extract filename and verify file exists, then delete it
        if let Some(pos) = toast.find("spade-logs-") {
            let filename = &toast[pos..];
            assert!(std::path::Path::new(filename).exists());
            let _ = std::fs::remove_file(filename);
        }
    }

    #[test]
    fn test_character_inspector_open_and_close() {
        let mut screen = LiveDashboardScreen::new();
        let player = OnlinePlayerInfo {
            name: "Gandalf".into(),
            level: 50,
            room: "Wizard Tower".into(),
            idle_secs: 12,
            class: "Mage".into(),
            race: "Human".into(),
            protocol: "Telnet".into(),
        };

        // Open inspector
        screen.open_character_inspector(player);
        assert!(screen.character_inspector.is_open);
        assert_eq!(
            screen
                .character_inspector
                .player
                .as_ref()
                .map(|p| p.name.as_str()),
            Some("Gandalf")
        );
        match screen.take_action() {
            ScreenAction::RpcCall { method, params, .. } => {
                assert_eq!(method, "imm.stat");
                assert_eq!(
                    params.get("target_name").and_then(|v| v.as_str()),
                    Some("Gandalf")
                );
            }
            other => panic!("Expected RpcCall imm.stat, got {:?}", other),
        }

        // Close inspector via Esc
        assert!(screen.modal_overlay_active());
        let consumed = screen.handle_key(make_key(KeyCode::Esc));
        assert!(consumed);
        assert!(!screen.character_inspector.is_open);
        assert!(!screen.modal_overlay_active());
    }

    #[test]
    fn test_character_inspector_rpc_response_parsing() {
        let mut screen = LiveDashboardScreen::new();
        let player = OnlinePlayerInfo {
            name: "Conan".into(),
            level: 30,
            room: "Arena".into(),
            idle_secs: 5,
            class: "Warrior".into(),
            race: "Human".into(),
            protocol: "WebSocket".into(),
        };
        screen.open_character_inspector(player);

        let json_val = serde_json::json!({
            "target": "Conan",
            "health": { "current": 250, "max": 300 },
            "mana": { "current": 40, "max": 100 },
            "stamina": { "current": 180, "max": 200 },
            "attributes": {
                "strength": 18,
                "dexterity": 15,
                "constitution": 17,
                "intelligence": 10,
                "wisdom": 12,
                "charisma": 14
            }
        });

        screen.handle_rpc_response("Character Stats for Conan", &json_val);

        assert_eq!(
            screen.character_inspector.health,
            Some(CharacterVitals {
                current: 250,
                max: 300
            })
        );
        assert_eq!(
            screen.character_inspector.mana,
            Some(CharacterVitals {
                current: 40,
                max: 100
            })
        );
        assert_eq!(
            screen.character_inspector.stamina,
            Some(CharacterVitals {
                current: 180,
                max: 200
            })
        );
        assert_eq!(
            screen.character_inspector.attributes,
            Some(CharacterAttributes {
                strength: 18,
                dexterity: 15,
                constitution: 17,
                intelligence: 10,
                wisdom: 12,
                charisma: 14,
            })
        );
    }

    #[test]
    fn test_character_inspector_imm_actions() {
        let mut screen = LiveDashboardScreen::new();
        let player = OnlinePlayerInfo {
            name: "Legolas".into(),
            level: 40,
            room: "Lothlorien".into(),
            idle_secs: 0,
            class: "Ranger".into(),
            race: "Elf".into(),
            protocol: "TLS-WebSocket".into(),
        };
        screen.open_character_inspector(player);
        let _ = screen.take_action();

        // 1. Heal
        let consumed = screen.handle_key(make_key(KeyCode::Char('h')));
        assert!(consumed);
        match screen.take_action() {
            ScreenAction::RpcCall { method, params, .. } => {
                assert_eq!(method, "imm.heal");
                assert_eq!(
                    params.get("target_name").and_then(|v| v.as_str()),
                    Some("Legolas")
                );
            }
            other => panic!("Expected RpcCall imm.heal, got {:?}", other),
        }

        // 2. Revive
        let consumed = screen.handle_key(make_key(KeyCode::Char('r')));
        assert!(consumed);
        match screen.take_action() {
            ScreenAction::RpcCall { method, params, .. } => {
                assert_eq!(method, "imm.revive");
                assert_eq!(
                    params.get("target_name").and_then(|v| v.as_str()),
                    Some("Legolas")
                );
            }
            other => panic!("Expected RpcCall imm.revive, got {:?}", other),
        }

        // 3. Advance level dialog
        let consumed = screen.handle_key(make_key(KeyCode::Char('a')));
        assert!(consumed);
        assert!(screen.character_inspector.advance_dialog_open);
        // Type level "45"
        screen.character_inspector.advance_input = "45".into();
        let consumed = screen.handle_key(make_key(KeyCode::Enter));
        assert!(consumed);
        assert!(!screen.character_inspector.advance_dialog_open);
        match screen.take_action() {
            ScreenAction::RpcCall { method, params, .. } => {
                assert_eq!(method, "imm.advance");
                assert_eq!(
                    params.get("player_name").and_then(|v| v.as_str()),
                    Some("Legolas")
                );
                assert_eq!(
                    params.get("target_level").and_then(|v| v.as_u64()),
                    Some(45)
                );
            }
            other => panic!("Expected RpcCall imm.advance, got {:?}", other),
        }

        // 4. Force command dialog
        let consumed = screen.handle_key(make_key(KeyCode::Char('m')));
        assert!(consumed);
        assert!(screen.character_inspector.force_cmd_dialog_open);
        screen.character_inspector.force_cmd_input = "dance".into();
        let consumed = screen.handle_key(make_key(KeyCode::Enter));
        assert!(consumed);
        assert!(!screen.character_inspector.force_cmd_dialog_open);
        match screen.take_action() {
            ScreenAction::RpcCall { method, params, .. } => {
                assert_eq!(method, "imm.force_command");
                assert_eq!(
                    params.get("player_name").and_then(|v| v.as_str()),
                    Some("Legolas")
                );
                assert_eq!(
                    params.get("command").and_then(|v| v.as_str()),
                    Some("dance")
                );
                assert_eq!(params.get("confirm").and_then(|v| v.as_bool()), Some(true));
            }
            other => panic!("Expected RpcCall imm.force_command, got {:?}", other),
        }
    }

    #[test]
    fn test_character_inspector_mouse_and_render() {
        let mut screen = LiveDashboardScreen::new();
        let player = OnlinePlayerInfo {
            name: "Frodo".into(),
            level: 15,
            room: "Bag End".into(),
            idle_secs: 2,
            class: "Burglar".into(),
            race: "Hobbit".into(),
            protocol: "Telnet".into(),
        };
        screen.open_character_inspector(player);
        let _ = screen.take_action();

        let area = Rect::new(0, 0, 100, 40);
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf, None);

        // Verify drawer rendered
        assert!(screen.character_inspector.drawer_rect.width > 0);
        assert!(!screen.character_inspector.button_rects.is_empty());

        // Find "heal" button rect and click it
        let heal_rect = screen
            .character_inspector
            .button_rects
            .iter()
            .find(|(id, _)| *id == "heal")
            .map(|(_, r)| *r)
            .expect("heal button rect");
        let click_heal = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: heal_rect.x,
            row: heal_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_heal, area);
        match screen.take_action() {
            ScreenAction::RpcCall { method, .. } => assert_eq!(method, "imm.heal"),
            other => panic!("Expected RpcCall imm.heal, got {:?}", other),
        }

        // Clicking outside drawer closes it
        let click_outside = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: screen.character_inspector.drawer_rect.x.saturating_sub(5),
            row: screen.character_inspector.drawer_rect.y + 5,
            modifiers: KeyModifiers::NONE,
        };
        screen.handle_mouse(click_outside, area);
        assert!(!screen.character_inspector.is_open);
    }
}
