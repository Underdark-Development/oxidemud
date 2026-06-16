use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

pub struct Modal {
    pub title: String,
    pub lines: Vec<String>,
    pub confirm_label: String,
    pub cancel_label: String,
    pub confirmed: Option<bool>,
}

impl Modal {
    pub fn new(title: String, lines: Vec<String>) -> Self {
        Modal {
            title,
            lines,
            confirm_label: "Enter: Confirm".to_string(),
            cancel_label: "Esc: Cancel".to_string(),
            confirmed: None,
        }
    }

    pub fn confirm(&mut self) {
        self.confirmed = Some(true);
    }

    pub fn cancel(&mut self) {
        self.confirmed = Some(false);
    }

    pub fn is_done(&self) -> bool {
        self.confirmed.is_some()
    }
}

impl Widget for &Modal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = (area.width as f32 * 0.7) as u16;
        let height = self.lines.len() as u16 + 4;
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;

        let modal_area = Rect::new(x, y, width, height);

        Clear.render(modal_area, buf);

        let inner = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                &self.title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Black));

        let mut content_lines: Vec<Line> = self
            .lines
            .iter()
            .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(Color::White))))
            .collect();

        content_lines.push(Line::from(""));
        content_lines.push(Line::from(vec![
            Span::styled(
                &self.confirm_label,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(&self.cancel_label, Style::default().fg(Color::DarkGray)),
        ]));

        let paragraph = Paragraph::new(content_lines)
            .block(inner)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        paragraph.render(modal_area, buf);
    }
}
