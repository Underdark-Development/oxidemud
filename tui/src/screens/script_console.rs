use std::fs;
use std::path::Path;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
};

use crate::screens::{Screen, ScreenAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    Console,
}

pub struct ScriptConsoleScreen {
    lines: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    editor_scroll_y: usize,
    editor_scroll_x: usize,
    console_logs: Vec<String>,
    console_scroll: usize,
    focus: Focus,
    action: ScreenAction,
}

impl Default for ScriptConsoleScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptConsoleScreen {
    pub fn new() -> Self {
        let mut screen = ScriptConsoleScreen {
            lines: vec![
                "// Press F5 to run this script or its test block".to_string(),
                "fn test_math() {".to_string(),
                "    let x = 2 + 3;".to_string(),
                "    if x != 5 { throw \"Math is broken\"; }".to_string(),
                "}".to_string(),
                "".to_string(),
                "let a = 10;".to_string(),
                "print(\"Script initialized.\");".to_string(),
            ],
            cursor_x: 0,
            cursor_y: 0,
            editor_scroll_y: 0,
            editor_scroll_x: 0,
            console_logs: vec![
                "Welcome to the Rhai Script Console.".to_string(),
                "Type your script above. Press Tab to switch panes, F5 to run.".to_string(),
            ],
            console_scroll: 0,
            focus: Focus::Editor,
            action: ScreenAction::None,
        };
        screen.ensure_cursor_visible();
        screen
    }

    fn log_output(&mut self, msg: String) {
        self.console_logs.push(msg);
        // Auto-scroll to bottom of console
        if self.console_logs.len() > 5 {
            self.console_scroll = self.console_logs.len() - 5;
        }
    }

    fn run_script(&mut self) {
        let code = self.lines.join("\n");
        self.log_output("--- Running script ---".to_string());

        let engine = oxide_scripting::ScriptEngine::new();

        // Check if there are any test functions
        let results = engine.run_tests(&code);
        if !results.is_empty() {
            self.log_output(format!("Found {} test functions:", results.len()));
            let mut failed = 0;
            for res in results {
                if res.success {
                    self.log_output(format!("  [OK]  {}", res.name));
                } else {
                    failed += 1;
                    self.log_output(format!(
                        "  [FAIL] {} — {}",
                        res.name,
                        res.error.as_deref().unwrap_or("unknown error")
                    ));
                }
            }
            if failed == 0 {
                self.log_output("All tests passed successfully!".to_string());
            } else {
                self.log_output(format!("{} tests failed.", failed));
            }
        } else {
            // No tests, just eval
            match engine.eval(&code) {
                Ok(_) => {
                    self.log_output("Execution completed successfully with no output.".to_string());
                }
                Err(e) => {
                    self.log_output(format!("Error: {}", e));
                }
            }
        }
    }

    fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_y];
        self.cursor_x = self.cursor_x.min(line.len());
        line.insert(self.cursor_x, c);
        self.cursor_x += 1;
        self.ensure_cursor_visible();
    }

    fn insert_newline(&mut self) {
        let line = &mut self.lines[self.cursor_y];
        self.cursor_x = self.cursor_x.min(line.len());
        let next_part = line.split_off(self.cursor_x);
        self.lines.insert(self.cursor_y + 1, next_part);
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.ensure_cursor_visible();
    }

    fn delete_prev_char(&mut self) {
        if self.cursor_x > 0 {
            let line = &mut self.lines[self.cursor_y];
            self.cursor_x = self.cursor_x.min(line.len());
            line.remove(self.cursor_x - 1);
            self.cursor_x -= 1;
            self.ensure_cursor_visible();
        } else if self.cursor_y > 0 {
            let current_line = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            let prev_line = &mut self.lines[self.cursor_y];
            self.cursor_x = prev_line.len();
            prev_line.push_str(&current_line);
            self.ensure_cursor_visible();
        }
    }

    fn delete_next_char(&mut self) {
        let line = &mut self.lines[self.cursor_y];
        self.cursor_x = self.cursor_x.min(line.len());
        if self.cursor_x < line.len() {
            line.remove(self.cursor_x);
        } else if self.cursor_y + 1 < self.lines.len() {
            let next_line = self.lines.remove(self.cursor_y + 1);
            self.lines[self.cursor_y].push_str(&next_line);
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
            self.ensure_cursor_visible();
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
            self.ensure_cursor_visible();
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
            self.ensure_cursor_visible();
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
            self.ensure_cursor_visible();
        }
    }

    fn move_cursor_right(&mut self) {
        let line_len = self.lines[self.cursor_y].len();
        if self.cursor_x < line_len {
            self.cursor_x += 1;
            self.ensure_cursor_visible();
        } else if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            self.cursor_x = 0;
            self.ensure_cursor_visible();
        }
    }

    fn ensure_cursor_visible(&mut self) {
        if self.cursor_y < self.editor_scroll_y {
            self.editor_scroll_y = self.cursor_y;
        }
    }
}

impl Screen for ScriptConsoleScreen {
    fn name(&self) -> &str {
        "Script Console"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.focus {
            Focus::Editor => match key.code {
                KeyCode::Tab => {
                    self.focus = Focus::Console;
                    true
                }
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    self.insert_char(c);
                    true
                }
                KeyCode::Enter => {
                    self.insert_newline();
                    true
                }
                KeyCode::Backspace => {
                    self.delete_prev_char();
                    true
                }
                KeyCode::Delete => {
                    self.delete_next_char();
                    true
                }
                KeyCode::Up => {
                    self.move_cursor_up();
                    true
                }
                KeyCode::Down => {
                    self.move_cursor_down();
                    true
                }
                KeyCode::Left => {
                    self.move_cursor_left();
                    true
                }
                KeyCode::Right => {
                    self.move_cursor_right();
                    true
                }
                KeyCode::F(9) => {
                    self.run_script();
                    true
                }
                _ => false,
            },
            Focus::Console => match key.code {
                KeyCode::Tab => {
                    self.focus = Focus::Editor;
                    true
                }
                KeyCode::Up => {
                    if self.console_scroll > 0 {
                        self.console_scroll -= 1;
                    }
                    true
                }
                KeyCode::Down => {
                    if self.console_scroll + 1 < self.console_logs.len() {
                        self.console_scroll += 1;
                    }
                    true
                }
                KeyCode::F(9) => {
                    self.run_script();
                    true
                }
                _ => false,
            },
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        let instr = " [Tab] Switch pane  [F9] Run script & tests  [Arrows] Move cursor / scroll ";
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

        let v_layout = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)]);
        let [top_area, bottom_area] = v_layout.areas(main_area);

        // Render editor pane
        let editor_focused = self.focus == Focus::Editor;
        let editor_border_style = if editor_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Indexed(240))
        };
        draw_border(buf, top_area, " Script Editor ", editor_border_style);

        let editor_inner = Rect::new(
            top_area.x + 1,
            top_area.y + 1,
            top_area.width.saturating_sub(2),
            top_area.height.saturating_sub(2),
        );

        // Adjust scroll view vertically
        let visible_height = editor_inner.height as usize;
        if self.cursor_y >= self.editor_scroll_y + visible_height {
            self.editor_scroll_y = self.cursor_y - visible_height + 1;
        }

        // Draw editor lines
        for i in 0..visible_height {
            let idx = self.editor_scroll_y + i;
            if idx >= self.lines.len() {
                break;
            }
            let line_str = &self.lines[idx];

            // Line number
            let num_style = Style::default().fg(Color::Indexed(242));
            let num_str = format!("{:>3} │ ", idx + 1);
            set_str_safe(
                buf,
                editor_inner,
                editor_inner.x as i32,
                (editor_inner.y + i as u16) as i32,
                &num_str,
                num_style,
            );

            // Highlight line
            let line = highlight_line(line_str);
            buf.set_line(
                editor_inner.x + 6,
                editor_inner.y + i as u16,
                &line,
                editor_inner.width.saturating_sub(6),
            );
        }

        // Draw visual cursor block
        if editor_focused {
            let cx =
                editor_inner.x as i32 + 6 + (self.cursor_x as i32 - self.editor_scroll_x as i32);
            let cy = editor_inner.y as i32 + (self.cursor_y as i32 - self.editor_scroll_y as i32);
            if cx >= editor_inner.x as i32 + 6
                && cx < (editor_inner.x + editor_inner.width) as i32
                && cy >= editor_inner.y as i32
                && cy < (editor_inner.y + editor_inner.height) as i32
            {
                if let Some(cell) = buf.cell_mut((cx as u16, cy as u16)) {
                    if cell.symbol() == " " || cell.symbol().is_empty() {
                        cell.set_char(' ');
                    }
                    cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
                }
            }
        }

        // Render console pane
        let console_focused = self.focus == Focus::Console;
        let console_border_style = if console_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Indexed(240))
        };
        draw_border(buf, bottom_area, " Console Output ", console_border_style);

        let console_inner = Rect::new(
            bottom_area.x + 1,
            bottom_area.y + 1,
            bottom_area.width.saturating_sub(2),
            bottom_area.height.saturating_sub(2),
        );

        let c_visible_height = console_inner.height as usize;
        if self.console_scroll + c_visible_height > self.console_logs.len() {
            self.console_scroll = self.console_logs.len().saturating_sub(c_visible_height);
        }

        for i in 0..c_visible_height {
            let idx = self.console_scroll + i;
            if idx >= self.console_logs.len() {
                break;
            }
            let log_line = &self.console_logs[idx];
            buf.set_string(
                console_inner.x,
                console_inner.y + i as u16,
                log_line,
                Style::default().fg(Color::Indexed(250)),
            );
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        let main_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );
        let v_layout = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)]);
        let [top_area, bottom_area] = v_layout.areas(main_area);

        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            if mouse.row >= top_area.y && mouse.row < top_area.y + top_area.height {
                self.focus = Focus::Editor;
            } else if mouse.row >= bottom_area.y && mouse.row < bottom_area.y + bottom_area.height {
                self.focus = Focus::Console;
            }
        }
    }

    fn load_script_file(&mut self, path: &Path) {
        if let Ok(content) = fs::read_to_string(path) {
            self.lines = content.lines().map(|s| s.to_string()).collect();
            if self.lines.is_empty() {
                self.lines.push(String::new());
            }
            self.cursor_x = 0;
            self.cursor_y = 0;
            self.editor_scroll_y = 0;
            self.editor_scroll_x = 0;
            self.focus = Focus::Editor;

            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            self.log_output(format!("--- Loaded script {} ---", file_name));
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }
}

fn highlight_line(line: &str) -> Line<'_> {
    use ratatui::text::Span;
    let mut spans = Vec::new();

    if line.trim().is_empty() {
        return Line::from(vec![Span::raw(line.to_string())]);
    }

    if line.trim().starts_with("//") {
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Indexed(245)),
        ));
    } else {
        let words = line.split_inclusive(|c: char| !c.is_alphanumeric() && c != '_');
        for word in words {
            let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            let style = match trimmed {
                "let" | "const" | "fn" | "if" | "else" | "while" | "for" | "in" | "return"
                | "import" | "as" => Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                "true" | "false" => Style::default().fg(Color::Green),
                _ if trimmed.chars().all(|c| c.is_numeric()) => Style::default().fg(Color::Green),
                _ => Style::default().fg(Color::White),
            };

            if word.contains('"') || word.contains('\'') {
                spans.push(Span::styled(
                    word.to_string(),
                    Style::default().fg(Color::Yellow),
                ));
            } else {
                spans.push(Span::styled(word.to_string(), style));
            }
        }
    }

    Line::from(spans)
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
