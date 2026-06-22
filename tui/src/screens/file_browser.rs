use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};

use crate::components::CommandAction;
use crate::components::{Tree, TreeNode};
use crate::screens::{Screen, ScreenAction};

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Preview,
}

pub struct FileBrowserScreen {
    content_path: PathBuf,
    roots: Vec<TreeNode<FileNode>>,
    tree: Tree<FileNode>,
    selected_file_content: Option<String>,
    right_scroll: usize,
    focus: Focus,
    last_click: Option<(usize, Instant)>,
    action: ScreenAction,
}

impl FileBrowserScreen {
    pub fn new(content_path: PathBuf) -> Self {
        let roots = build_file_tree(&content_path);
        let mut tree = Tree::new(roots.clone());
        if !tree.flatten().is_empty() {
            tree.selected = Some(0);
        }
        let mut screen = FileBrowserScreen {
            content_path,
            roots,
            tree,
            selected_file_content: None,
            right_scroll: 0,
            focus: Focus::Tree,
            last_click: None,
            action: ScreenAction::None,
        };
        screen.load_selected_file();
        screen
    }

    fn load_selected_file(&mut self) {
        self.selected_file_content = None;
        self.right_scroll = 0;
        if let Some(node) = self.tree.selected_data() {
            if !node.is_dir {
                if let Ok(content) = fs::read_to_string(&node.path) {
                    self.selected_file_content = Some(content);
                }
            }
        }
    }

    fn trigger_open(&mut self) -> Result<bool, String> {
        let node = match self.tree.selected_data() {
            Some(n) => n,
            None => return Ok(false),
        };

        if node.is_dir {
            self.tree.toggle_selected();
            return Ok(true);
        }

        let ext = node.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "toml" {
            if let Some((category, id)) = resolve_template(&node.path, &self.content_path) {
                self.action = ScreenAction::Inspect(category, id);
                return Ok(true);
            }
        } else if ext == "rhai" {
            self.action = ScreenAction::LoadScript(node.path.clone());
            return Ok(true);
        }

        Ok(false)
    }
}

impl Screen for FileBrowserScreen {
    fn name(&self) -> &str {
        "File Browser"
    }

    fn reload(&mut self) {
        self.roots = build_file_tree(&self.content_path);
        self.tree = Tree::new(self.roots.clone());
        if !self.tree.flatten().is_empty() {
            self.tree.selected = Some(0);
        }
        self.load_selected_file();
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.focus {
            Focus::Tree => match key.code {
                KeyCode::Tab => {
                    self.focus = Focus::Preview;
                    true
                }
                KeyCode::Up => {
                    self.tree.select_prev();
                    self.load_selected_file();
                    true
                }
                KeyCode::Down => {
                    self.tree.select_next();
                    self.load_selected_file();
                    true
                }
                KeyCode::Right | KeyCode::Left => {
                    if let Some(idx) = self.tree.selected {
                        if let Some((_, node)) = self.tree.flatten().get(idx) {
                            if !node.is_leaf() {
                                self.tree.toggle_selected();
                            }
                        }
                    }
                    true
                }
                KeyCode::Enter => {
                    let _ = self.trigger_open();
                    true
                }
                _ => false,
            },
            Focus::Preview => match key.code {
                KeyCode::Tab => {
                    self.focus = Focus::Tree;
                    true
                }
                KeyCode::Up => {
                    if self.right_scroll > 0 {
                        self.right_scroll -= 1;
                    }
                    true
                }
                KeyCode::Down => {
                    if let Some(ref content) = self.selected_file_content {
                        let lines = content.lines().count();
                        if self.right_scroll + 5 < lines {
                            self.right_scroll += 1;
                        }
                    }
                    true
                }
                KeyCode::PageUp => {
                    self.right_scroll = self.right_scroll.saturating_sub(15);
                    true
                }
                KeyCode::PageDown => {
                    if let Some(ref content) = self.selected_file_content {
                        let lines = content.lines().count();
                        self.right_scroll = (self.right_scroll + 15).min(lines.saturating_sub(5));
                    }
                    true
                }
                _ => false,
            },
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        // Draw instructions bar
        let instr =
            " [Tab] Switch pane  [Arrows] Navigate tree / scroll preview  [Enter] Open file ";
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

        let h_layout = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]);
        let [left_area, right_area] = h_layout.areas(main_area);

        // Draw left tree borders
        let left_focused = self.focus == Focus::Tree;
        let left_border_style = if left_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Indexed(240))
        };
        draw_border(buf, left_area, " File Tree ", left_border_style);

        let left_inner = Rect::new(
            left_area.x + 1,
            left_area.y + 1,
            left_area.width.saturating_sub(2),
            left_area.height.saturating_sub(2),
        );
        self.tree.update_scroll(left_inner.height as usize);
        self.tree.render(left_inner, buf);

        // Draw right preview borders
        let right_focused = self.focus == Focus::Preview;
        let right_border_style = if right_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Indexed(240))
        };
        draw_border(buf, right_area, " Preview ", right_border_style);

        let right_inner = Rect::new(
            right_area.x + 1,
            right_area.y + 1,
            right_area.width.saturating_sub(2),
            right_area.height.saturating_sub(2),
        );

        if let Some(ref content) = self.selected_file_content {
            let is_toml = self
                .tree
                .selected_data()
                .and_then(|n| n.path.extension())
                .is_some_and(|ext| ext == "toml");

            let lines: Vec<&str> = content.lines().collect();
            let visible_lines = right_inner.height as usize;
            for i in 0..visible_lines {
                let idx = self.right_scroll + i;
                if idx >= lines.len() {
                    break;
                }
                let line_str = lines[idx];
                let line = highlight_line(line_str, is_toml);
                buf.set_line(
                    right_inner.x,
                    right_inner.y + i as u16,
                    &line,
                    right_inner.width,
                );
            }
        } else {
            let msg = " (No file loaded) ";
            let x = right_inner.x + (right_inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = right_inner.y + right_inner.height / 2;
            if y < right_inner.y + right_inner.height {
                buf.set_string(x, y, msg, Style::default().fg(Color::Indexed(245)));
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            let main_area = Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            );
            let h_layout =
                Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]);
            let [left_area, right_area] = h_layout.areas(main_area);

            if mouse.column >= left_area.x && mouse.column < left_area.x + left_area.width {
                self.focus = Focus::Tree;
                let local_y = mouse.row.saturating_sub(left_area.y + 1) as usize;
                let flat = self.tree.flatten();
                let clicked_idx = self.tree.scroll.offset + local_y;

                if clicked_idx < flat.len() {
                    let now = Instant::now();
                    let is_double_click = if let Some((last_idx, last_time)) = self.last_click {
                        last_idx == clicked_idx
                            && now.duration_since(last_time) < std::time::Duration::from_millis(500)
                    } else {
                        false
                    };

                    self.tree.selected = Some(clicked_idx);
                    self.load_selected_file();
                    self.last_click = Some((clicked_idx, now));

                    if is_double_click {
                        let _ = self.trigger_open();
                    }
                }
            } else if mouse.column >= right_area.x && mouse.column < right_area.x + right_area.width
            {
                self.focus = Focus::Preview;
            }
        } else if mouse.kind == MouseEventKind::ScrollUp {
            if self.focus == Focus::Preview {
                if self.right_scroll > 0 {
                    self.right_scroll -= 1;
                }
            } else {
                self.tree.scroll_up();
            }
        } else if mouse.kind == MouseEventKind::ScrollDown {
            if self.focus == Focus::Preview {
                if let Some(ref content) = self.selected_file_content {
                    let lines = content.lines().count();
                    if self.right_scroll + 5 < lines {
                        self.right_scroll += 1;
                    }
                }
            } else {
                self.tree.scroll_down();
            }
        }
    }

    fn contextual_commands(&self) -> Vec<(String, CommandAction)> {
        let node = match self.tree.selected_data() {
            Some(n) => n,
            None => return Vec::new(),
        };

        if node.is_dir {
            return Vec::new();
        }

        let ext = node.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "toml" {
            vec![(
                "Open in Entities Editor".to_string(),
                CommandAction::EditEntity,
            )]
        } else if ext == "rhai" {
            vec![(
                "Load in Script Console".to_string(),
                CommandAction::EditEntity,
            )]
        } else {
            Vec::new()
        }
    }

    fn handle_command_action(&mut self, action: &CommandAction) -> Result<bool, String> {
        match action {
            CommandAction::EditEntity => self.trigger_open(),
            _ => Ok(false),
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }
}

fn build_file_tree(dir: &Path) -> Vec<TreeNode<FileNode>> {
    let mut nodes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });

        for entry in entries {
            let path = entry.path();
            let is_dir = path.is_dir();
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();

            if file_name.starts_with('.') || file_name == "target" {
                continue;
            }

            let mut node = TreeNode::new(
                file_name,
                FileNode {
                    path: path.clone(),
                    is_dir,
                },
            );

            if is_dir {
                node.children = build_file_tree(&path);
                node.collapsed = true;
            }

            nodes.push(node);
        }
    }
    nodes
}

fn resolve_template(path: &Path, content_path: &Path) -> Option<(String, String)> {
    let rel = path.strip_prefix(content_path).ok()?;
    let mut components = rel.components();
    let first = components.next()?.as_os_str().to_str()?;

    if first == "areas" {
        let area_id = components.next()?.as_os_str().to_str()?;
        let next = components.next()?.as_os_str().to_str()?;
        if next == "rooms" {
            let room_file = components.next()?.as_os_str().to_str()?;
            let room_id = room_file.strip_suffix(".toml")?;
            Some(("rooms".to_string(), room_id.to_string()))
        } else if next == "area.toml" {
            Some(("areas".to_string(), area_id.to_string()))
        } else {
            None
        }
    } else {
        let file_name = components.next()?.as_os_str().to_str()?;
        let id = file_name.strip_suffix(".toml")?;
        Some((first.to_string(), id.to_string()))
    }
}

fn highlight_line(line: &str, is_toml: bool) -> Line<'_> {
    use ratatui::text::Span;
    let mut spans = Vec::new();

    if line.trim().is_empty() {
        return Line::from(vec![Span::raw(line.to_string())]);
    }

    if is_toml {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Indexed(245)),
            ));
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            spans.push(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if let Some((key, rest)) = line.split_once('=') {
            spans.push(Span::styled(
                key.to_string(),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::raw("="));

            let rest_trimmed = rest.trim();
            if (rest_trimmed.starts_with('"') && rest_trimmed.ends_with('"'))
                || (rest_trimmed.starts_with('\'') && rest_trimmed.ends_with('\''))
            {
                spans.push(Span::styled(
                    rest.to_string(),
                    Style::default().fg(Color::Yellow),
                ));
            } else if rest_trimmed == "true"
                || rest_trimmed == "false"
                || rest_trimmed
                    .chars()
                    .all(|c| c.is_numeric() || c == '.' || c == '-')
            {
                spans.push(Span::styled(
                    rest.to_string(),
                    Style::default().fg(Color::Green),
                ));
            } else {
                spans.push(Span::raw(rest.to_string()));
            }
        } else {
            spans.push(Span::raw(line.to_string()));
        }
    } else {
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
                    _ if trimmed.chars().all(|c| c.is_numeric()) => {
                        Style::default().fg(Color::Green)
                    }
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
