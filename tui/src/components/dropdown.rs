use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

/// Draw the border block and fill interior with black background.
pub fn render_dropdown_box(buf: &mut Buffer, rect: Rect, border_style: Style) {
    if rect.width < 4 || rect.height < 3 {
        return;
    }
    ratatui::widgets::Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .style(border_style.bg(Color::Black));
    block.render(rect, buf);
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                if x > rect.x
                    && x < rect.x + rect.width - 1
                    && y > rect.y
                    && y < rect.y + rect.height - 1
                {
                    cell.set_char(' ');
                }
                cell.set_style(Style::default().bg(Color::Black).fg(Color::White));
            }
        }
    }
}

/// Apply full-row highlight background for a dropdown item row.
pub fn highlight_dropdown_row(buf: &mut Buffer, rect: Rect, y: u16) {
    for x in rect.x + 1..rect.x + rect.width - 1 {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_bg(Color::Indexed(240));
        }
    }
}

/// Style for a dropdown item label (highlighted or normal).
pub fn dropdown_item_style(highlighted: bool) -> Style {
    if highlighted {
        Style::default().fg(Color::White).bg(Color::Indexed(240))
    } else {
        Style::default().fg(Color::Indexed(245)).bg(Color::Black)
    }
}
