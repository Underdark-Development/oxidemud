use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseEvent},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
};

use crate::components::CommandAction;
use crate::screens::{Screen, ScreenAction};

pub struct LiveDashboardScreen {
    cpu_usage: f32,
    memory_mb: f32,
    tick_rate_ms: f32,
    pending_flushes: usize,
    active_connections: usize,
    logs: Vec<String>,
    log_scroll: usize,
    fullscreen_logs: bool,
    last_update: Instant,
    start_time: Instant,
    action: ScreenAction,
}

impl LiveDashboardScreen {
    pub fn new() -> Self {
        let mut screen = LiveDashboardScreen {
            cpu_usage: 5.4,
            memory_mb: 32.1,
            tick_rate_ms: 20.0,
            pending_flushes: 0,
            active_connections: 1,
            logs: vec![
                "[INFO] MUD game server initialized successfully.".to_string(),
                "[INFO] SQLite database connection opened (WAL mode enabled).".to_string(),
                "[INFO] Telnet listener started on port 4000.".to_string(),
            ],
            log_scroll: 0,
            fullscreen_logs: false,
            last_update: Instant::now(),
            start_time: Instant::now(),
            action: ScreenAction::None,
        };
        screen.generate_mock_log();
        screen
    }

    fn generate_mock_log(&mut self) {
        let mock_templates = &[
            "[INFO] Database flushed 4 dirty entities in 2.1ms",
            "[DEBUG] AI system: ticked 24 active mobs",
            "[INFO] Player 'therealklanni' connected from 127.0.0.1",
            "[DEBUG] Scheduler: combat tick interval triggered",
            "[INFO] Weather System: zone 'forest' changed to raining",
            "[INFO] DB WAL checkpoint completed (12 pages written)",
            "[DEBUG] Regen System: updated health/stamina for 1 active player",
            "[INFO] Combat: 'goblin_grunt' dealt 4 physical damage to 'therealklanni'",
            "[INFO] Loot: 'therealklanni' looted 'iron_dagger_1' from 'goblin_grunt'",
            "[DEBUG] Connections cleanup: 0 idle connections dropped",
        ];

        let index = (self.memory_mb as usize + self.logs.len()) % mock_templates.len();
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let formatted = format!("[uptime: {:5.1}s] {}", elapsed, mock_templates[index]);

        self.logs.push(formatted);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    fn update_stats(&mut self) {
        let elapsed = self.last_update.elapsed();
        if elapsed.as_secs_f32() >= 1.0 {
            // Jitter the mock stats slightly
            let cpu_mod = (elapsed.subsec_nanos() % 7) as f32 - 3.0; // [-3.0, 3.0]
            self.cpu_usage = (self.cpu_usage + cpu_mod * 0.5).clamp(2.0, 25.0);

            let mem_mod = (elapsed.subsec_nanos() % 5) as f32 - 2.0; // [-2.0, 2.0]
            self.memory_mb = (self.memory_mb + mem_mod * 0.1).clamp(28.0, 45.0);

            let tick_mod = (elapsed.subsec_nanos() % 9) as f32 - 4.5;
            self.tick_rate_ms = (20.0 + tick_mod * 0.2).clamp(18.0, 22.5);

            self.pending_flushes = (elapsed.subsec_nanos() % 3) as usize;
            self.active_connections = 1 + (elapsed.subsec_nanos() % 3) as usize;

            self.generate_mock_log();
            self.last_update = Instant::now();
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
        match key.code {
            KeyCode::Char(' ') => {
                self.fullscreen_logs = !self.fullscreen_logs;
                true
            }
            KeyCode::Up => {
                if self.log_scroll > 0 {
                    self.log_scroll -= 1;
                }
                true
            }
            KeyCode::Down => {
                if self.log_scroll + 1 < self.logs.len() {
                    self.log_scroll += 1;
                }
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        self.update_stats();

        // Draw instructions bar
        let instr = " [Space] Toggle fullscreen logs  [Arrows] Scroll log stream ";
        let instr_style = Style::default()
            .fg(Color::Indexed(245))
            .bg(Color::Indexed(236));
        set_str_safe(buf, area, area.x as i32, area.y as i32, instr, instr_style);
        for x in (area.x + instr.len() as u16)..area.x + area.width {
            set_char_safe(buf, area, x as i32, area.y as i32, ' ', instr_style);
        }

        let main_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );

        if self.fullscreen_logs {
            // Only draw logs
            draw_border(
                buf,
                main_area,
                " System Logs (Fullscreen) ",
                Style::default().fg(Color::Green),
            );
            let inner = Rect::new(
                main_area.x + 1,
                main_area.y + 1,
                main_area.width.saturating_sub(2),
                main_area.height.saturating_sub(2),
            );

            let visible = inner.height as usize;
            if self.log_scroll + visible > self.logs.len() {
                self.log_scroll = self.logs.len().saturating_sub(visible);
            }

            for i in 0..visible {
                let idx = self.log_scroll + i;
                if idx >= self.logs.len() {
                    break;
                }
                buf.set_string(
                    inner.x,
                    inner.y + i as u16,
                    &self.logs[idx],
                    Style::default().fg(Color::Indexed(250)),
                );
            }
        } else {
            // Split layout
            let h_layout =
                Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]);
            let [left_area, right_area] = h_layout.areas(main_area);

            // Left: Stats
            draw_border(
                buf,
                left_area,
                " System Stats ",
                Style::default().fg(Color::Indexed(240)),
            );
            let left_inner = Rect::new(
                left_area.x + 1,
                left_area.y + 1,
                left_area.width.saturating_sub(2),
                left_area.height.saturating_sub(2),
            );

            let text_style = Style::default().fg(Color::White);
            let bold_label = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);

            // Draw CPU Stats
            let mut y = left_inner.y;
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32,
                y as i32,
                "CPU Usage:",
                bold_label,
            );
            let cpu_str = format!(" {:.1}%", self.cpu_usage);
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32 + 12,
                y as i32,
                &cpu_str,
                text_style,
            );
            y += 1;
            let cpu_pct = (self.cpu_usage / 100.0).clamp(0.0, 1.0);
            draw_progress_bar(
                buf,
                left_inner,
                left_inner.x as i32,
                y as i32,
                left_inner.width as usize,
                cpu_pct,
                Color::Green,
            );

            // Draw Memory Stats
            y += 2;
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32,
                y as i32,
                "Memory:",
                bold_label,
            );
            let mem_str = format!(" {:.1} MB", self.memory_mb);
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32 + 12,
                y as i32,
                &mem_str,
                text_style,
            );
            y += 1;
            let mem_pct = (self.memory_mb / 64.0).clamp(0.0, 1.0); // scale against 64MB
            draw_progress_bar(
                buf,
                left_inner,
                left_inner.x as i32,
                y as i32,
                left_inner.width as usize,
                mem_pct,
                Color::Cyan,
            );

            // Other indicators
            y += 2;
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32,
                y as i32,
                "Tick Rate:",
                bold_label,
            );
            let rate_str = format!(" {:.1} ms (target 20ms)", self.tick_rate_ms);
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32 + 12,
                y as i32,
                &rate_str,
                text_style,
            );

            y += 1.max(visible_adjusted_y(left_inner.height));
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32,
                y as i32,
                "DB Flushes:",
                bold_label,
            );
            let flush_str = format!(" {} pending", self.pending_flushes);
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32 + 12,
                y as i32,
                &flush_str,
                text_style,
            );

            y += 1.max(visible_adjusted_y(left_inner.height));
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32,
                y as i32,
                "Telnet Conns:",
                bold_label,
            );
            let conn_str = format!(" {} active", self.active_connections);
            set_str_safe(
                buf,
                left_inner,
                left_inner.x as i32 + 12,
                y as i32,
                &conn_str,
                text_style,
            );

            // Right: Logs
            draw_border(
                buf,
                right_area,
                " System Logs ",
                Style::default().fg(Color::Indexed(240)),
            );
            let right_inner = Rect::new(
                right_area.x + 1,
                right_area.y + 1,
                right_area.width.saturating_sub(2),
                right_area.height.saturating_sub(2),
            );

            let visible = right_inner.height as usize;
            if self.log_scroll + visible > self.logs.len() {
                self.log_scroll = self.logs.len().saturating_sub(visible);
            }

            for i in 0..visible {
                let idx = self.log_scroll + i;
                if idx >= self.logs.len() {
                    break;
                }
                buf.set_string(
                    right_inner.x,
                    right_inner.y + i as u16,
                    &self.logs[idx],
                    Style::default().fg(Color::Indexed(250)),
                );
            }
        }
    }

    fn handle_mouse(&mut self, _mouse: MouseEvent, _area: Rect) {}

    fn contextual_commands(&self) -> Vec<(String, CommandAction)> {
        vec![(
            "Toggle Fullscreen Logs".to_string(),
            CommandAction::EditEntity,
        )]
    }

    fn handle_command_action(&mut self, action: &CommandAction) -> Result<bool, String> {
        match action {
            CommandAction::EditEntity => {
                self.fullscreen_logs = !self.fullscreen_logs;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }
}

fn visible_adjusted_y(height: u16) -> u16 {
    if height > 15 {
        2
    } else {
        1
    }
}

fn draw_progress_bar(
    buf: &mut Buffer,
    area: Rect,
    x: i32,
    y: i32,
    width: usize,
    pct: f32,
    color: Color,
) {
    let filled = (width as f32 * pct).round() as usize;
    let filled = filled.min(width);

    let filled_str = "█".repeat(filled);
    let empty_str = "░".repeat(width - filled);

    set_str_safe(buf, area, x, y, &filled_str, Style::default().fg(color));
    set_str_safe(
        buf,
        area,
        x + filled as i32,
        y,
        &empty_str,
        Style::default().fg(Color::Indexed(240)),
    );
}

fn draw_border(buf: &mut Buffer, area: Rect, title: &str, style: Style) {
    if area.width < 2 || area.height < 2 {
        return;
    }

    // Horizontal borders
    for x in area.x..area.x + area.width {
        set_char_safe(buf, area, x as i32, area.y as i32, '─', style);
        set_char_safe(
            buf,
            area,
            x as i32,
            (area.y + area.height - 1) as i32,
            '─',
            style,
        );
    }
    // Vertical borders
    for y in area.y..area.y + area.height {
        set_char_safe(buf, area, area.x as i32, y as i32, '│', style);
        set_char_safe(
            buf,
            area,
            (area.x + area.width - 1) as i32,
            y as i32,
            '│',
            style,
        );
    }
    // Corners
    set_char_safe(buf, area, area.x as i32, area.y as i32, '┌', style);
    set_char_safe(
        buf,
        area,
        (area.x + area.width - 1) as i32,
        area.y as i32,
        '┐',
        style,
    );
    set_char_safe(
        buf,
        area,
        area.x as i32,
        (area.y + area.height - 1) as i32,
        '└',
        style,
    );
    set_char_safe(
        buf,
        area,
        (area.x + area.width - 1) as i32,
        (area.y + area.height - 1) as i32,
        '┘',
        style,
    );

    // Title
    if title.len() < area.width as usize - 2 {
        let x = area.x + 1;
        set_str_safe(
            buf,
            area,
            x as i32,
            area.y as i32,
            title,
            style.add_modifier(Modifier::BOLD),
        );
    }
}

fn set_char_safe(buf: &mut Buffer, area: Rect, x: i32, y: i32, ch: char, style: Style) {
    if x >= area.x as i32
        && x < (area.x + area.width) as i32
        && y >= area.y as i32
        && y < (area.y + area.height) as i32
    {
        if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
            cell.set_char(ch);
            cell.set_style(style);
        }
    }
}

fn set_str_safe(buf: &mut Buffer, area: Rect, x: i32, y: i32, s: &str, style: Style) {
    for (i, ch) in s.chars().enumerate() {
        set_char_safe(buf, area, x + i as i32, y, ch, style);
    }
}
