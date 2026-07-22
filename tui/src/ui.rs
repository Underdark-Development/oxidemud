use crate::app::App;
use crate::screens::ScreenId;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    Frame,
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(2),
    ]);
    let [top_area, sep_area, main_area, status_area] = layout.areas(area);

    // --- Top bar (menu bar) ---
    let screen_name = app.active_screen().name().to_string();
    app.menu_bar
        .render_top_bar(frame.buffer_mut(), top_area, &screen_name);

    // --- Separator ---
    let buf = frame.buffer_mut();
    for x in sep_area.x..sep_area.x + sep_area.width {
        if let Some(cell) = buf.cell_mut((x, sep_area.y)) {
            cell.set_char('─');
            cell.set_fg(Color::Indexed(245));
            cell.set_bg(Color::Indexed(236));
        }
    }

    // --- Main area ---
    for y in main_area.y..main_area.y + main_area.height {
        for x in main_area.x..main_area.x + main_area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_bg(Color::Black);
            }
        }
    }

    let (content_area, sidebar_area) = if app.sidebar_visible {
        let h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(36)]);
        let [c, s] = h.areas(main_area);
        (c, Some(s))
    } else {
        (main_area, None)
    };

    let mouse_pos = app.mouse_pos;
    let sidebar_focused = app.sidebar_focused;
    app.active_screen_mut().set_sidebar_focused(sidebar_focused);
    app.active_screen_mut().render(content_area, buf, mouse_pos);

    if let Some(sidebar_area) = sidebar_area {
        for y in sidebar_area.y..sidebar_area.y + sidebar_area.height {
            for x in sidebar_area.x..sidebar_area.x + sidebar_area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(Color::Indexed(236));
                }
            }
        }

        let sidebar_title_fg = if app.sidebar_focused {
            Color::White
        } else {
            Color::Indexed(245)
        };
        let sidebar_title = if app.active_screen == ScreenId::RoomGrid {
            " Attributes "
        } else {
            " Commands "
        };
        buf.set_string(
            sidebar_area.x,
            sidebar_area.y,
            sidebar_title,
            Style::default()
                .fg(sidebar_title_fg)
                .bg(Color::Indexed(236)),
        );

        let inner_area = ratatui::layout::Rect::new(
            sidebar_area.x,
            sidebar_area.y + 1,
            sidebar_area.width,
            sidebar_area.height.saturating_sub(1),
        );

        let ctx = app.active_screen().selection_context();
        let contextual = app.active_screen().contextual_commands();

        // Pass room details if on Room Grid screen
        let selected_room = if app.active_screen == ScreenId::RoomGrid {
            ctx.as_ref().and_then(|c| {
                app.registry
                    .areas
                    .values()
                    .find_map(|a| a.rooms.get(&c.id))
                    .cloned()
            })
        } else {
            None
        };
        app.command_sidebar.room_details = selected_room;

        app.command_sidebar.update_context(
            ctx.as_ref().map(|c| (c.name.clone(), c.category.clone())),
            &contextual,
        );
        app.command_sidebar
            .render(inner_area, buf, app.sidebar_focused, mouse_pos);
    }

    // --- Command Palette Overlay ---
    if app.command_palette_open {
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(cell.style().add_modifier(ratatui::style::Modifier::DIM));
                    if cell.fg == Color::White {
                        cell.set_fg(Color::Indexed(242));
                    }
                }
            }
        }
        let mouse_pos = app.mouse_pos;
        app.command_palette.render(area, buf, mouse_pos);
    }

    // --- Menu dropdown overlays (render last, on top of everything) ---
    app.menu_bar.render_dropdowns(buf, area);

    // --- Status bar ---
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

    let status_msg = app
        .status_message
        .as_ref()
        .filter(|(_, ts)| ts.elapsed() < std::time::Duration::from_secs(5))
        .map(|(msg, _)| msg.as_str());

    for y in status_area.y..status_area.y + status_area.height {
        for x in status_area.x..status_area.x + status_area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_bg(Color::Indexed(236));
            }
        }
    }

    // Status line 0: mode (reverse) + connection info + unsaved count
    let mode_y = status_area.y;
    let mut cursor_x = status_area.x;

    // Leading normal-bg space — matches APP_NAME convention in menu_bar
    buf.set_string(
        cursor_x,
        mode_y,
        " ",
        Style::default().fg(Color::White).bg(Color::Indexed(236)),
    );
    cursor_x += 1;

    let padded_mode = format!(" {} ", mode_label);
    buf.set_string(
        cursor_x,
        mode_y,
        &padded_mode,
        Style::default().fg(Color::White).bg(Color::Indexed(236)),
    );
    for (i, _) in padded_mode.char_indices() {
        if let Some(cell) = buf.cell_mut((cursor_x + i as u16, mode_y)) {
            cell.set_fg(Color::Black);
            cell.set_bg(Color::White);
        }
    }
    cursor_x += padded_mode.len() as u16;

    if !connection_info.is_empty() {
        let conn_text = format!(" |{connection_info}");
        buf.set_string(
            cursor_x,
            mode_y,
            &conn_text,
            Style::default()
                .fg(Color::Indexed(245))
                .bg(Color::Indexed(236)),
        );
        cursor_x += conn_text.len() as u16;
    }

    let unsaved = app.active_screen().unsaved_count();
    if unsaved > 0 {
        let plural = if unsaved == 1 { "change" } else { "changes" };
        let unsaved_text = format!(" {unsaved} unsaved {plural} ");
        let unsaved_x =
            (status_area.x + status_area.width).saturating_sub(unsaved_text.len() as u16);
        if unsaved_x >= cursor_x {
            buf.set_string(
                unsaved_x,
                mode_y,
                &unsaved_text,
                Style::default().fg(Color::White).bg(Color::Indexed(236)),
            );
        }
    }

    // Status line 1: action feedback
    if let Some(msg) = status_msg {
        buf.set_string(
            status_area.x,
            status_area.y + 1,
            format!(" {msg}"),
            Style::default().fg(Color::Yellow).bg(Color::Indexed(236)),
        );
    }
}
