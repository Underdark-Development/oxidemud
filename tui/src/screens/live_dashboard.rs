use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Gauge, List, ListItem, Paragraph, Row, Sparkline, Table, TableState, Widget,
    },
};

use crate::network::{ConnectionStatus, SpadeTelemetry};
use crate::screens::{Screen, ScreenAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardView {
    Dashboard,
    FullLog,
}

pub struct LiveDashboardScreen {
    action: ScreenAction,
    pub view_mode: DashboardView,
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
    pub connect_dialog_open: bool,
    pub connect_input: String,
}

impl LiveDashboardScreen {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        LiveDashboardScreen {
            action: ScreenAction::None,
            view_mode: DashboardView::Dashboard,
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
            connect_dialog_open: false,
            connect_input: "127.0.0.1:8080".to_string(),
        }
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
}

impl Default for LiveDashboardScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for LiveDashboardScreen {
    fn name(&self) -> &str {
        "Live Dashboard"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.connect_dialog_open {
            match key.code {
                KeyCode::Esc => {
                    self.connect_dialog_open = false;
                    return true;
                }
                KeyCode::Enter => {
                    self.connect_dialog_open = false;
                    self.status = ConnectionStatus::Connecting;
                    return true;
                }
                KeyCode::Char(c) => {
                    self.connect_input.push(c);
                    return true;
                }
                KeyCode::Backspace => {
                    self.connect_input.pop();
                    return true;
                }
                _ => return true,
            }
        }

        if self.status == ConnectionStatus::Disconnected {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Enter => {
                    self.connect_dialog_open = true;
                    return true;
                }
                _ => return false,
            }
        }

        if self.gecho_dialog_open {
            match key.code {
                KeyCode::Esc => {
                    self.gecho_dialog_open = false;
                    self.gecho_input.clear();
                    return true;
                }
                KeyCode::Enter => {
                    if !self.gecho_input.trim().is_empty() {
                        self.logs.push(format!("[GECHO] {}", self.gecho_input));
                        self.gecho_input.clear();
                    }
                    self.gecho_dialog_open = false;
                    return true;
                }
                KeyCode::Char(c) => {
                    self.gecho_input.push(c);
                    return true;
                }
                KeyCode::Backspace => {
                    self.gecho_input.pop();
                    return true;
                }
                _ => return true,
            }
        }

        match key.code {
            KeyCode::Tab => {
                self.view_mode = match self.view_mode {
                    DashboardView::Dashboard => DashboardView::FullLog,
                    DashboardView::FullLog => DashboardView::Dashboard,
                };
                true
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                self.gecho_dialog_open = true;
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
            KeyCode::Up | KeyCode::Char('k') => {
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

        if self.status == ConnectionStatus::Disconnected {
            self.render_offline_pane(area, buf);
            return;
        }

        match self.view_mode {
            DashboardView::Dashboard => self.render_dashboard(area, buf),
            DashboardView::FullLog => self.render_full_log(area, buf),
        }

        if self.gecho_dialog_open {
            self.render_gecho_dialog(area, buf);
        }

        if self.connect_dialog_open {
            self.render_connect_dialog(area, buf);
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl LiveDashboardScreen {
    fn render_dashboard(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(8),
                Constraint::Length(1),
            ])
            .split(area);

        // 1. Header Bar
        let status_str = match self.status {
            ConnectionStatus::Connected => "● ONLINE",
            ConnectionStatus::Connecting => "◌ CONNECTING",
            ConnectionStatus::Disconnected => "○ OFFLINE",
        };
        let status_color = match self.status {
            ConnectionStatus::Connected => Color::Green,
            ConnectionStatus::Connecting => Color::Yellow,
            ConnectionStatus::Disconnected => Color::Red,
        };

        let uptime = self.telemetry.as_ref().map(|t| t.uptime_secs).unwrap_or(0);
        let uptime_str = format!(
            "{}h {}m {}s",
            uptime / 3600,
            (uptime % 3600) / 60,
            uptime % 60
        );

        let header_text = format!(
            " Status: {} | Ping: {}ms | Uptime: {} | DB WAL: OK ",
            status_str, self.ping_ms, uptime_str
        );
        let header_block = Block::default()
            .borders(Borders::ALL)
            .title(" SPADE LIVE DASHBOARD ")
            .style(Style::default().fg(Color::Cyan));
        Paragraph::new(header_text)
            .block(header_block)
            .style(Style::default().fg(status_color))
            .render(chunks[0], buf);

        // 2. Middle Row (Metrics Left, Players Right)
        let mid_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(chunks[1]);

        // Left Panel: System & Database Metrics
        let sys_block = Block::default()
            .borders(Borders::ALL)
            .title(" SYSTEM & DATABASE METRICS ")
            .style(Style::default().fg(Color::Cyan));
        let sys_area = mid_chunks[0];
        sys_block.render(sys_area, buf);

        let inner_sys = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), // Memory Gauge
                Constraint::Length(2), // Players Gauge
                Constraint::Length(2), // Sparkline Tick Drift
                Constraint::Min(4),    // Stats List
            ])
            .split(sys_area);

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
        let mem_label = if mem_total > 0 {
            format!(
                "{:.1} MB / {:.1} GB ({:.1}%)",
                mem_used_mb,
                mem_total_gb,
                mem_ratio * 100.0
            )
        } else {
            "Awaiting Telemetry...".to_string()
        };

        Gauge::default()
            .block(Block::default().title("Memory Usage"))
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Indexed(236)))
            .ratio(mem_ratio)
            .label(mem_label)
            .render(inner_sys[0], buf);

        let players_count = self
            .telemetry
            .as_ref()
            .map(|t| t.players.len())
            .unwrap_or(0);
        let players_ratio = (players_count as f64 / 50.0).min(1.0);
        let players_label = format!("{} / 50 Active", players_count);

        Gauge::default()
            .block(Block::default().title("Active Players"))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Indexed(236)))
            .ratio(players_ratio)
            .label(players_label)
            .render(inner_sys[1], buf);

        Sparkline::default()
            .block(Block::default().title("Game Tick Drift (ms)"))
            .data(&self.drift_history)
            .style(Style::default().fg(Color::Yellow))
            .render(inner_sys[2], buf);

        let rooms = self.telemetry.as_ref().map(|t| t.room_count).unwrap_or(0);
        let mobs = self.telemetry.as_ref().map(|t| t.mob_count).unwrap_or(0);
        let items = self.telemetry.as_ref().map(|t| t.item_count).unwrap_or(0);
        let wal_kb = self
            .telemetry
            .as_ref()
            .map(|t| t.wal_size_bytes / 1024)
            .unwrap_or(0);
        let dirty = self
            .telemetry
            .as_ref()
            .map(|t| t.dirty_entities)
            .unwrap_or(0);
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
            "SQLite WAL Size   : {} KB\n\
             Dirty Entity Queue: {} pending\n\
             Active Entities   : {} Rooms, {} Mobs, {} Items\n\
             Game Clock & Season: {} ({} {})",
            wal_kb, dirty, rooms, mobs, items, game_time, season, weather
        );
        Paragraph::new(stats_text)
            .style(Style::default().fg(Color::White))
            .render(inner_sys[3], buf);

        // Right Panel: Online Players Table
        let player_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" ONLINE PLAYERS ({}) ", players_count))
            .style(Style::default().fg(Color::Cyan));

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
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(player_block)
        .row_highlight_style(Style::default().bg(Color::Indexed(238)).fg(Color::Cyan));

        StatefulWidget::render(table, mid_chunks[1], buf, &mut self.table_state);

        // 3. Bottom Panel: Mini Event Log Feed
        let log_block = Block::default()
            .borders(Borders::ALL)
            .title(" RECENT SERVER LOGS (Press Tab for Full Stream) ")
            .style(Style::default().fg(Color::Cyan));

        let items: Vec<ListItem> = self
            .logs
            .iter()
            .rev()
            .take(5)
            .rev()
            .map(|l| ListItem::new(l.as_str()).style(Style::default().fg(Color::Indexed(250))))
            .collect();

        Widget::render(List::new(items).block(log_block), chunks[2], buf);

        // 4. Keybindings Footer Bar
        let footer_text = " [Tab] Toggle Full Logs | [G] Global Echo | [R] Reboot | [K] Kick Player | [C] Reconnect | [Q] Quit ";
        Paragraph::new(footer_text)
            .style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
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
            .title(" SERVER LOG VIEW - UNEDITED STREAM ")
            .style(Style::default().fg(Color::Cyan));

        Paragraph::new(header_text)
            .block(header_block)
            .style(Style::default().fg(Color::Yellow))
            .render(chunks[0], buf);

        let log_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let items: Vec<ListItem> = self
            .logs
            .iter()
            .rev()
            .take(chunks[1].height.saturating_sub(2) as usize)
            .rev()
            .map(|l| {
                let style = if l.contains("ERROR") {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else if l.contains("WARN") {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(l.as_str()).style(style)
            })
            .collect();

        Widget::render(List::new(items).block(log_block), chunks[1], buf);

        let footer_text = " [Tab] Back to Dashboard | [Space] Pause Stream | [Q] Quit ";
        Paragraph::new(footer_text)
            .style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .render(chunks[2], buf);
    }

    fn render_gecho_dialog(&self, area: Rect, buf: &mut Buffer) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" BROADCAST GLOBAL ECHO (gecho) ")
            .style(Style::default().fg(Color::Yellow).bg(Color::Indexed(235)));

        block.render(dialog_area, buf);

        let text = format!(
            "Enter announcement to broadcast:\n\n > {}_",
            self.gecho_input
        );
        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };
        Paragraph::new(text)
            .style(Style::default().fg(Color::White).bg(Color::Indexed(235)))
            .render(inner, buf);
    }

    fn render_offline_pane(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" SPADE LIVE DASHBOARD ")
            .style(Style::default().fg(Color::Cyan));
        block.render(area, buf);

        let msg =
            "Not connected to an active MUD server.\n\nPress [C] or [Enter] to connect to server";
        let inner_y = area.y + area.height / 2 - 1;
        let p = Paragraph::new(msg)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::Indexed(245)));

        let center_area = Rect {
            x: area.x + 2,
            y: inner_y,
            width: area.width.saturating_sub(4),
            height: 4,
        };
        p.render(center_area, buf);

        if self.connect_dialog_open {
            self.render_connect_dialog(area, buf);
        }
    }

    fn render_connect_dialog(&self, area: Rect, buf: &mut Buffer) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" CONNECT TO MUD SERVER ")
            .style(Style::default().fg(Color::Cyan).bg(Color::Indexed(235)));

        block.render(dialog_area, buf);

        let text = format!(
            "Enter server address (host:port):\n\n > {}_",
            self.connect_input
        );
        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };
        Paragraph::new(text)
            .style(Style::default().fg(Color::White).bg(Color::Indexed(235)))
            .render(inner, buf);
    }
}

use ratatui::widgets::StatefulWidget;
