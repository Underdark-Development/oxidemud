use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

pub struct TooltipPopup;

impl TooltipPopup {
    pub fn render(
        buf: &mut Buffer,
        screen_area: Rect,
        anchor_x: u16,
        anchor_y: u16,
        message: &str,
        is_toml_error: bool,
    ) {
        if message.is_empty() || screen_area.width < 10 || screen_area.height < 4 {
            return;
        }

        let title = if is_toml_error {
            " ⚠ TOML Error "
        } else {
            " ⚠ Validation Error "
        };
        let border_color = if is_toml_error {
            Color::LightRed
        } else {
            Color::Yellow
        };

        // Determine box width (max 60, min 25)
        let msg_len = message.len() + 6;
        let width = (msg_len as u16)
            .clamp(25, 60)
            .min(screen_area.width.saturating_sub(4));

        // Estimate height based on wrapped text length
        let inner_width = width.saturating_sub(4) as usize;
        let lines_needed = if inner_width > 0 {
            (message.len() / inner_width) + 1
        } else {
            1
        };
        let height = ((lines_needed + 2) as u16)
            .clamp(3, 8)
            .min(screen_area.height.saturating_sub(2));

        // Determine X position (aligned near anchor_x)
        let mut x = anchor_x.saturating_add(2);
        if x + width > screen_area.x + screen_area.width {
            x = screen_area.x + screen_area.width.saturating_sub(width);
        }

        // Determine Y position (default below anchor_y, flip above if near bottom)
        let mut y = anchor_y.saturating_add(1);
        if y + height > screen_area.y + screen_area.height {
            y = anchor_y.saturating_sub(height);
        }

        let popup_area = Rect::new(x, y, width, height);

        // 1. Clear background behind popup
        Widget::render(Clear, popup_area, buf);

        // 2. Render bordered box with title and message text
        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
            .block(block);

        Widget::render(paragraph, popup_area, buf);
    }
}
