use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Widget},
};

use crate::theme;

/// Draw the rounded border block and fill the interior with the canvas style.
pub fn render_dropdown_box(buf: &mut Buffer, rect: Rect, border_style: Style) {
    if rect.width < 4 || rect.height < 3 {
        return;
    }
    ratatui::widgets::Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(theme::canvas_style());
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
                cell.set_style(theme::canvas_style());
            }
        }
    }
}

/// Apply full-row selected background for a dropdown item row.
pub fn highlight_dropdown_row(buf: &mut Buffer, rect: Rect, y: u16) {
    for x in rect.x + 1..rect.x + rect.width - 1 {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_bg(theme::PRIMARY);
        }
    }
}

/// Style for a dropdown item label (highlighted or normal).
pub fn dropdown_item_style(highlighted: bool) -> Style {
    if highlighted {
        theme::selected_row()
    } else {
        Style::default().fg(theme::FG_MUTED).bg(theme::BG)
    }
}
