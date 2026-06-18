use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

#[derive(Clone)]
pub struct ScrollState {
    pub offset: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
}

impl ScrollState {
    pub fn new() -> Self {
        ScrollState {
            offset: 0,
            visible_lines: 0,
            total_lines: 0,
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = self.total_lines.saturating_sub(self.visible_lines);
        self.offset = self.offset.saturating_add(amount).min(max);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.offset = self.offset.saturating_sub(amount);
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.visible_lines);
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.visible_lines);
    }

    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.offset = self.total_lines.saturating_sub(self.visible_lines);
    }

    pub fn percentage(&self) -> Option<u8> {
        let max_offset = self.total_lines.saturating_sub(self.visible_lines);
        if max_offset == 0 {
            return None;
        }
        Some((self.offset * 100 / max_offset) as u8)
    }

    pub fn is_scrolled(&self) -> bool {
        self.offset > 0
    }

    pub fn thumb(&self, area_height: usize) -> (usize, usize) {
        if self.total_lines == 0 || self.total_lines <= self.visible_lines {
            return (0, 0);
        }
        let max_offset = self.total_lines.saturating_sub(self.visible_lines);
        let thumb_height = (self.visible_lines * area_height / self.total_lines).max(1);
        let thumb_pos = self.offset * (area_height.saturating_sub(thumb_height)) / max_offset;
        (thumb_pos, thumb_height)
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &ScrollState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 1 || area.height < 1 {
            return;
        }
        if self.total_lines <= self.visible_lines {
            return;
        }
        let height = area.height as usize;
        let (thumb_pos, thumb_height) = self.thumb(height);

        for y in 0..height {
            let cell = &mut buf[(area.x, area.y + y as u16)];
            cell.set_symbol(" ");
            if y >= thumb_pos && y < thumb_pos + thumb_height {
                cell.set_bg(Color::Indexed(240));
            } else {
                cell.set_bg(Color::Indexed(236));
            }
        }
    }
}
