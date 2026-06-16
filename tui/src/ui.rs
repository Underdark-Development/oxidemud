use crate::app::App;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]);
    let [tabs_area, main_area, status_area] = layout.areas(area);

    let mode_label = match app.mode {
        crate::app::Mode::Offline => "offline",
        crate::app::Mode::Online => "online",
        crate::app::Mode::Split => "split",
    };

    let connection_info = match app.mode {
        crate::app::Mode::Offline | crate::app::Mode::Split => String::new(),
        crate::app::Mode::Online => {
            format!(" {}:{} ", app.connection_host, app.connection_port)
        }
    };

    let title = format!(
        " spade — {mode_label}{connection_info} [Ctrl+1-9: screens, Ctrl+R: reload, Ctrl+D: quit] "
    );

    frame.render_widget(
        Block::default()
            .title(title)
            .style(Style::default().fg(Color::DarkGray)),
        tabs_area,
    );

    let buf = frame.buffer_mut();
    app.active_screen_mut().render(main_area, buf);

    let status_bar = Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {} ", app.active_screen().name()),
            Style::default().fg(Color::White),
        ),
        ratatui::text::Span::styled(connection_info, Style::default().fg(Color::Cyan)),
    ]))
    .style(Style::default().bg(Color::Blue));

    frame.render_widget(status_bar, status_area);
}
