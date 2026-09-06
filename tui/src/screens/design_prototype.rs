//! Design-system gallery (prototype build).
//!
//! Reached via the undocumented `--prototype` flag. Renders every shared spade
//! widget through the real `components/*` types at the current style so a
//! change to `Button`, `Dialog`, `Table`, etc. is visible across all of its
//! use cases and states after a rebuild. Throwaway code: it is not part of the
//! normal app flow and must not gain production responsibilities.

use crate::components::{
    command_sidebar::CommandSidebar, Badge, BadgeKind, Button, CommandAction, ContextMenu, Dialog,
    Form, FormField, RowBadges, ScrollState, Table, TooltipPopup, Tree, TreeNode,
};
use crate::screens::Screen;
use crate::theme::{self, ButtonState, ButtonVariant, DialogTone};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Gauge, Sparkline, Widget},
};

const SECTION_PAD: u16 = 2;
const ROW_GAP: u16 = 1;
const ROW_HEIGHTS: [u16; 7] = [12u16, 12, 16, 12, 14, 14, 33];

pub struct DesignPrototypeScreen {
    scroll: ScrollState,
    menu_bar: crate::components::menu_bar::MenuBar,
    sidebar: CommandSidebar,
}

impl DesignPrototypeScreen {
    pub fn new() -> Self {
        let mut menu_bar = crate::components::menu_bar::MenuBar::new();
        menu_bar.open_menu = Some(1);
        let mut sidebar = CommandSidebar::new();
        sidebar.update_context(
            Some(("Goblin".to_string(), "mob".to_string())),
            &[
                ("Inspect".into(), CommandAction::LookMobDetail),
                ("Edit".into(), CommandAction::EditEntity),
                ("Duplicate".into(), CommandAction::DuplicateEntity),
                ("Delete".into(), CommandAction::DeleteEntity),
            ],
        );
        DesignPrototypeScreen {
            scroll: ScrollState::new(),
            menu_bar,
            sidebar,
        }
    }

    fn section(buf: &mut Buffer, area: Rect, title: &str) {
        let w = area.width.saturating_sub(2);
        let dashes = "─".repeat(w.saturating_sub((title.len() + 6) as u16) as usize);
        let line = format!("─── {title} {dashes}");
        buf.set_string(
            area.x + 1,
            area.y,
            &line,
            Style::default()
                .fg(theme::FG_MUTED)
                .add_modifier(Modifier::BOLD),
        );
    }

    fn cell_area(area: Rect, col: u16, row_origin: u16, col_w: u16, row_h: u16) -> Rect {
        let cell_w = col_w.saturating_sub(SECTION_PAD) / 2;
        let x = area.x + SECTION_PAD / 2 + col * cell_w.max(1);
        Rect::new(x, row_origin, cell_w, row_h)
    }

    fn row_origin(area: Rect, row: u16, offset: u16) -> u16 {
        let mut y = area.y.saturating_sub(offset);
        for h in &ROW_HEIGHTS[..row as usize] {
            y += h + ROW_GAP;
        }
        y
    }

    fn row_height(row: usize) -> u16 {
        ROW_HEIGHTS[row]
    }

    fn content_height() -> u16 {
        ROW_HEIGHTS.iter().sum::<u16>() + (ROW_HEIGHTS.len() as u16 - 1) * ROW_GAP
    }
}

impl Default for DesignPrototypeScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for DesignPrototypeScreen {
    fn name(&self) -> &str {
        "Design System Gallery"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => false,
            KeyCode::Up => {
                self.scroll.scroll_up(1);
                true
            }
            KeyCode::Down => {
                self.scroll.scroll_down(1);
                true
            }
            KeyCode::PageUp => {
                self.scroll.page_up();
                true
            }
            KeyCode::PageDown => {
                self.scroll.page_down();
                true
            }
            _ => true,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, _area: Rect) {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.scroll.scroll_down(3),
            MouseEventKind::ScrollUp => self.scroll.scroll_up(3),
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, mouse_pos: Option<(u16, u16)>) {
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(theme::BG);
                }
            }
        }

        self.scroll.total_lines = Self::content_height() as usize;
        self.scroll.visible_lines = area.height.saturating_sub(2) as usize;
        let offset = self.scroll.offset as u16;
        let col_w = area.width.saturating_sub(SECTION_PAD);

        let full_cell = |row: u16| {
            Rect::new(
                area.x + 1,
                Self::row_origin(area, row, offset),
                col_w,
                Self::row_height(row as usize),
            )
        };
        let cell = |col: u16, row: u16| {
            let origin = Self::row_origin(area, row, offset);
            let h = Self::row_height(row as usize);
            Self::cell_area(area, col, origin, col_w, h)
        };

        let c00 = cell(0, 0);
        let c10 = cell(1, 0);
        let c01 = cell(0, 1);
        let c11 = cell(1, 1);
        let c02 = cell(0, 2);
        let c12 = cell(1, 2);
        let c03 = cell(0, 3);
        let c13 = cell(1, 3);
        let c04 = cell(0, 4);
        let c14 = cell(1, 4);
        let c05 = cell(0, 5);
        let c15 = cell(1, 5);
        let c06 = full_cell(6);

        Self::draw_cell(area, buf, c00, |b, r| self.render_buttons(b, r));
        Self::draw_cell(area, buf, c10, |b, r| self.render_status(b, r));
        Self::draw_cell(area, buf, c01, |b, r| self.render_tabs(b, r));
        Self::draw_cell(area, buf, c11, |b, r| self.render_form(b, r));
        Self::draw_cell(area, buf, c02, |b, r| self.render_table(b, r, mouse_pos));
        Self::draw_cell(area, buf, c12, |b, r| self.render_tree(b, r, mouse_pos));
        Self::draw_cell(area, buf, c03, |b, r| self.render_gauges(b, r));
        Self::draw_cell(area, buf, c13, |b, r| self.render_menus(b, r));
        Self::draw_cell(area, buf, c04, |b, r| {
            self.sidebar.render(r, b, true, mouse_pos);
        });
        Self::draw_cell(area, buf, c14, |b, r| self.render_context_menu(b, r));
        Self::draw_cell(area, buf, c05, |b, r| self.render_tooltips(b, r));
        Self::draw_cell(area, buf, c15, |b, r| {
            self.render_logs(b, r, mouse_pos);
        });
        Self::draw_cell(area, buf, c06, |b, r| self.render_dialogs(b, r));

        self.render_help_bar(buf, area);
    }
}

impl DesignPrototypeScreen {
    fn draw_cell<F>(frame: Rect, buf: &mut Buffer, cell: Rect, mut draw: F)
    where
        F: FnMut(&mut Buffer, Rect),
    {
        if cell.width < 1 || cell.height < 1 {
            return;
        }
        let x0 = cell.x.max(frame.x);
        let y0 = cell.y.max(frame.y);
        let x1 = cell.right().min(frame.right());
        let y1 = cell.bottom().min(frame.bottom());
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let mut off = Buffer::empty(cell);
        for y in cell.y..cell.bottom() {
            for x in cell.x..cell.right() {
                off[(x, y)].set_bg(theme::BG);
            }
        }
        draw(&mut off, cell);
        for y in y0..y1 {
            for x in x0..x1 {
                let src = &off[(x, y)];
                buf[(x, y)] = src.clone();
            }
        }
    }

    fn render_help_bar(&self, buf: &mut Buffer, area: Rect) {
        let y = area.y + area.height.saturating_sub(1);
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_bg(theme::PANEL);
            }
        }
        let text = " ↑/↓ scroll · PgUp/PgDn page · wheel scroll · q/Esc quit ";
        buf.set_string(
            area.x,
            y,
            text,
            Style::default().fg(theme::PRIMARY).bg(theme::PANEL),
        );
    }

    // -----------------------------------------------------------------------
    // Row 0 — Buttons | Status Badges
    // -----------------------------------------------------------------------

    fn render_buttons(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Buttons");
        let variants = [
            ButtonVariant::Default,
            ButtonVariant::Destructive,
            ButtonVariant::Neutral,
            ButtonVariant::Warning,
        ];
        let mut x = area.x + 1;
        for variant in variants {
            let label = format!("{variant:?}");
            let btn = Button::new(label, variant);
            btn.render(buf, x, area.y + 2, ButtonState::Active);
            btn.render(buf, x, area.y + 3, ButtonState::Inactive);
            x += btn.width() + 2;
        }
        Button::new("Disabled", ButtonVariant::Disabled).render(
            buf,
            x,
            area.y + 2,
            ButtonState::Inactive,
        );
    }

    fn render_dialogs(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Dialogs");
        let tones = [
            (DialogTone::Info, "Info"),
            (DialogTone::Confirm, "Confirm"),
            (DialogTone::Destructive, "Destructive"),
        ];
        for (i, (tone, label)) in tones.iter().enumerate() {
            let sub = Rect::new(
                area.x + 2,
                area.y + 2 + i as u16 * 10,
                area.width.saturating_sub(4).max(1),
                10,
            );
            let mut d = Dialog::new(
                *tone,
                label,
                "A short confirmation message.",
                &["Cancel".to_string(), "OK".to_string()],
            );
            d.render(sub, buf, None);
        }
    }

    // -----------------------------------------------------------------------
    // Row 1 — Tabs | Form Fields
    // -----------------------------------------------------------------------

    fn render_tabs(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Tabs");
        let mut tabs = crate::components::Tabs::new(
            vec![
                "Overview".into(),
                "Details".into(),
                "History".into(),
                "Settings".into(),
            ],
            vec![(), (), (), ()],
        );
        tabs.select(1);
        let sub = Rect::new(area.x + 1, area.y + 2, area.width.saturating_sub(2), 2);
        (&tabs).render(sub, buf);
    }

    fn render_form(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Form Fields");
        let mut form = Form::new(vec![
            FormField::new(
                "Name".into(),
                "Goblin".into(),
                crate::components::FieldType::Text,
            ),
            FormField::new(
                "Level".into(),
                "12".into(),
                crate::components::FieldType::Number,
            ),
            FormField::new(
                "Alive".into(),
                "true".into(),
                crate::components::FieldType::Bool,
            ),
            FormField::new(
                "Diet".into(),
                "Omnivore".into(),
                crate::components::FieldType::Choice(vec![
                    "Herbivore".into(),
                    "Carnivore".into(),
                    "Omnivore".into(),
                ]),
            ),
            FormField::new(
                "HP Dice".into(),
                "2d6".into(),
                crate::components::FieldType::DiceNotation,
            ),
        ]);
        form.set_error(4, "invalid dice notation".into());
        form.focus = 0;
        let sub = Rect::new(
            area.x + 1,
            area.y + 2,
            area.width.saturating_sub(2),
            form.fields.len() as u16 + 1,
        );
        (&form).render(sub, buf);
    }

    // -----------------------------------------------------------------------
    // Row 2 — Table + Row Chips | Tree
    // -----------------------------------------------------------------------

    fn render_table(&self, buf: &mut Buffer, area: Rect, mouse_pos: Option<(u16, u16)>) {
        Self::section(buf, area, "Table + Row Chips");
        let mut table = Table::new(vec!["Name".into(), "Type".into(), "Lvl".into()]);
        table.column_widths = vec![
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(5),
        ];
        for (name, kind, lvl) in [
            ("Goblin", "mob", "3"),
            ("Sword of Light", "item", "1"),
            ("Forest Gate", "room", "0"),
            ("Warlord", "mob", "9"),
            ("Iron Shield", "item", "2"),
        ] {
            table.add_row(vec![name.into(), kind.into(), lvl.into()]);
        }
        table.set_row_badges(
            3,
            RowBadges {
                col: 0,
                gap: 3,
                badges: vec![
                    Badge {
                        text: "+ Add Entry",
                        kind: BadgeKind::AddEntry,
                    },
                    Badge {
                        text: "▲",
                        kind: BadgeKind::MoveUp,
                    },
                    Badge {
                        text: "▼",
                        kind: BadgeKind::MoveDown,
                    },
                    Badge {
                        text: "Clear",
                        kind: BadgeKind::Clear,
                    },
                    Badge {
                        text: "✕",
                        kind: BadgeKind::Remove,
                    },
                ],
            },
        );
        table.selected = Some(2);
        table.hovered = mouse_pos.map(|_| 1);
        table.scroll.visible_lines = area.height.saturating_sub(3) as usize;
        let sub = Rect::new(
            area.x + 1,
            area.y + 2,
            area.width.saturating_sub(2),
            area.height.saturating_sub(3),
        );
        table.render_table(sub, buf);
    }

    fn render_tree(&self, buf: &mut Buffer, area: Rect, mouse_pos: Option<(u16, u16)>) {
        Self::section(buf, area, "Tree");
        let mut r1 = TreeNode::new("Forest".into(), ());
        r1.add_child(TreeNode::new("Forest Gate".into(), ()));
        r1.add_child(TreeNode::new("Clearing".into(), ()));
        r1.collapsed = true;
        let mut r2 = TreeNode::new("Caves".into(), ());
        r2.add_child(TreeNode::new("Warlord Lair".into(), ()));
        r2.dirty = true;
        let mut root = TreeNode::new("Wildlands".into(), ());
        root.add_child(r1);
        root.add_child(r2);
        let mut tree: Tree<()> = Tree::new(vec![root]);
        tree.indent = 2;
        tree.selected = Some(1);
        tree.hovered = mouse_pos.map(|_| 2);
        tree.search_filter = Some("lair".into());
        tree.scroll.visible_lines = area.height.saturating_sub(3) as usize;
        let sub = Rect::new(
            area.x + 1,
            area.y + 2,
            area.width.saturating_sub(2),
            area.height.saturating_sub(3),
        );
        (&tree).render(sub, buf);
    }

    // -----------------------------------------------------------------------
    // Row 3 — Gauges + Sparkline | Menu Bar + Dropdown
    // -----------------------------------------------------------------------

    fn render_gauges(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Gauges + Sparkline");
        for (i, (pct, color)) in [
            (35u16, theme::DANGER),
            (65, theme::WARNING),
            (90, theme::PRIMARY),
        ]
        .into_iter()
        .enumerate()
        {
            let g = Gauge::default()
                .gauge_style(Style::default().fg(color).bg(theme::BG_DARK))
                .ratio(f64::from(pct) / 100.0)
                .label(format!("{pct}%"));
            let rect = Rect::new(
                area.x + 1,
                area.y + 2 + i as u16 * 2,
                area.width.saturating_sub(4) / 2,
                1,
            );
            g.render(rect, buf);
        }
        let data = vec![1u64, 3, 2, 5, 4, 7, 6, 9, 8, 12, 11, 14, 13, 16, 15, 18];
        let sp = Sparkline::default()
            .data(&data)
            .style(Style::default().fg(theme::PRIMARY).bg(theme::BG_DARK))
            .max(20);
        let rect = Rect::new(
            area.x + area.width / 2,
            area.y + 2,
            area.width.saturating_sub(2) / 2,
            6,
        );
        sp.render(rect, buf);
    }

    fn render_status(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Status Badges");
        let badges = [
            (" offline ", theme::FG_MUTED),
            (" online ● ", theme::POSITIVE),
            (" connecting ◐ ", theme::WARNING),
            (" disconnected ○ ", theme::DANGER),
            (" split ", theme::PRIMARY),
        ];
        for (i, (label, color)) in badges.iter().enumerate() {
            buf.set_string(
                area.x + 1,
                area.y + 2 + i as u16 * 2,
                label,
                Style::default()
                    .fg(theme::BG)
                    .bg(*color)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Row 4 — Command Sidebar | Context Menu
    // -----------------------------------------------------------------------

    fn render_menus(&mut self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Menu Bar + Dropdown");
        let top = Rect::new(area.x + 1, area.y + 2, area.width.saturating_sub(2), 1);
        self.menu_bar.render_top_bar(buf, top, "Entities Editor");
        let drop = Rect::new(
            area.x + 1,
            area.y + 3,
            area.width.saturating_sub(2),
            area.height.saturating_sub(4),
        );
        self.menu_bar.render_dropdowns(buf, drop);
    }

    // -----------------------------------------------------------------------
    // Row 5 — Tooltips | Logs + Scrollbar
    // -----------------------------------------------------------------------

    fn render_context_menu(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Context Menu");
        let items = vec![
            ("Inspect".to_string(), "inspect".to_string()),
            ("Edit".to_string(), "edit".to_string()),
            ("Delete".to_string(), "delete".to_string()),
        ];
        let mut menu = ContextMenu::new(items, area.x + 2, area.y + 3);
        menu.selected = 1;
        (&menu).render(area, buf);
    }

    fn render_tooltips(&self, buf: &mut Buffer, area: Rect) {
        Self::section(buf, area, "Tooltips");
        TooltipPopup::render(
            buf,
            area,
            area.x + 4,
            area.y + 4,
            "toml: expected `key = value`",
            true,
        );
        TooltipPopup::render(
            buf,
            area,
            area.x + 4,
            area.y + 10,
            "validation: health out of range",
            false,
        );
    }

    // -----------------------------------------------------------------------
    // Row 6 — Dialogs (full width)
    // -----------------------------------------------------------------------

    fn render_logs(&self, buf: &mut Buffer, area: Rect, _mouse_pos: Option<(u16, u16)>) {
        Self::section(buf, area, "Logs + Scrollbar");
        let logs = [
            "[INFO] Server started on :8080",
            "[RPC SUCCESS] imm.gecho: ok",
            "[RPC ERROR] imm.shutdown failed: timeout",
            "[NETWORK] Telemetry stream connected",
            "[WARN] high memory usage",
            "[INFO] world saved",
        ];
        let list_h = 6.min(area.height.saturating_sub(3) as usize);
        for (i, line) in logs.iter().enumerate().take(list_h) {
            let y = area.y + 2 + i as u16;
            let fg = if line.contains("ERROR") {
                theme::DANGER
            } else if line.contains("WARN") {
                theme::WARNING
            } else if line.contains("SUCCESS") {
                theme::POSITIVE
            } else {
                theme::FG
            };
            buf.set_string(area.x + 1, y, line, Style::default().fg(fg));
        }
        let ss = ScrollState {
            offset: 2,
            visible_lines: list_h,
            total_lines: 12,
        };
        let sb = Rect::new(area.x + area.width - 1, area.y + 2, 1, list_h as u16);
        (&ss).render(sb, buf);
    }
}

/// Launch the design-system prototype gallery. Returns when the user quits.
pub async fn run_prototype() -> color_eyre::Result<()> {
    let mut terminal = crate::app::init_terminal(true)?;
    let _guard = PrototypeGuard;
    let mut event_loop = crate::event::EventLoop::new()?;
    let mut screen = DesignPrototypeScreen::new();

    loop {
        terminal.draw(|f| screen.render(f.area(), f.buffer_mut(), None))?;
        match event_loop.next().await? {
            crate::event::Event::Key(key) => {
                if !screen.handle_key(key) {
                    break;
                }
            }
            crate::event::Event::Mouse(mouse) => {
                let size = terminal.size()?;
                let area = Rect::new(0, 0, size.width, size.height);
                screen.handle_mouse(mouse, area);
            }
            crate::event::Event::Resize(_, _) | crate::event::Event::Tick => {}
        }
    }
    terminal.show_cursor()?;
    Ok(())
}

struct PrototypeGuard;

impl Drop for PrototypeGuard {
    fn drop(&mut self) {
        let _ = crate::app::restore_terminal();
    }
}
