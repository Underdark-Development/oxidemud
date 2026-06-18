use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct Tabs<T> {
    pub titles: Vec<String>,
    pub active: usize,
    pub data: Vec<T>,
}

impl<T> Tabs<T> {
    pub fn new(titles: Vec<String>, data: Vec<T>) -> Self {
        Tabs {
            titles,
            active: 0,
            data,
        }
    }

    pub fn select(&mut self, index: usize) {
        if index < self.titles.len() {
            self.active = index;
        }
    }

    pub fn next(&mut self) {
        if !self.titles.is_empty() {
            self.active = (self.active + 1) % self.titles.len();
        }
    }

    pub fn prev(&mut self) {
        if self.titles.is_empty() {
            return;
        }
        self.active = if self.active == 0 {
            self.titles.len() - 1
        } else {
            self.active - 1
        };
    }

    pub fn active_data(&self) -> Option<&T> {
        self.data.get(self.active)
    }

    pub fn active_mut(&mut self) -> Option<&mut T> {
        self.data.get_mut(self.active)
    }
}

impl<T> Widget for &Tabs<T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || self.titles.is_empty() {
            return;
        }

        let tab_width = area.width / self.titles.len() as u16;

        for (i, title) in self.titles.iter().enumerate() {
            let is_active = i == self.active;
            let x = area.x + (i as u16 * tab_width);
            let tab_area = Rect::new(x, area.y, tab_width, 1);

            let style = if is_active {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Indexed(245))
            };

            let padding = tab_width.saturating_sub(title.len() as u16 + 2) / 2;
            let padded = format!(" {}{} ", " ".repeat(padding as usize), title);

            let line = Line::from(Span::styled(padded, style));
            buf.set_line(tab_area.x, tab_area.y, &line, tab_width);

            if is_active {
                let underline = Line::from(Span::styled(
                    "─".repeat(tab_width as usize),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                buf.set_line(tab_area.x, tab_area.y + 1, &underline, tab_width);
            }
        }
    }
}
