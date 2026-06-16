use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

pub struct ContextMenu {
    pub items: Vec<(String, String)>,
    pub selected: usize,
    pub x: u16,
    pub y: u16,
}

impl ContextMenu {
    pub fn new(items: Vec<(String, String)>, x: u16, y: u16) -> Self {
        ContextMenu {
            items,
            selected: 0,
            x,
            y,
        }
    }

    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.items.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub fn selected_action(&self) -> &str {
        self.items
            .get(self.selected)
            .map(|(_, action)| action.as_str())
            .unwrap_or("")
    }
}

impl Widget for &ContextMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = self
            .items
            .iter()
            .map(|(label, _)| label.len())
            .max()
            .unwrap_or(15) as u16
            + 4;
        let height = self.items.len() as u16 + 2;

        let x = self.x.min(area.width.saturating_sub(width));
        let y = self.y.min(area.height.saturating_sub(height));

        let menu_area = Rect::new(area.x + x, area.y + y, width, height);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        for (i, (label, _)) in self.items.iter().enumerate() {
            let iy = inner.y + i as u16;
            if iy >= inner.y + inner.height {
                break;
            }
            let is_selected = i == self.selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let marker = if is_selected { "▸ " } else { "  " };
            let line = Line::from(Span::styled(format!("{}{}", marker, label), style));
            buf.set_line(inner.x, iy, &line, inner.width);
        }
    }
}
