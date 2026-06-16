use crate::app::App;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
        app.should_quit = true;
        return;
    }

    if key.modifiers == KeyModifiers::CONTROL {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap_or(0) as usize;
                if idx > 0 {
                    app.switch_screen(idx - 1);
                }
                return;
            }
            KeyCode::Char('r') => {
                app.reload_content();
                return;
            }
            _ => {}
        }
    }

    app.active_screen_mut().handle_key(key);
}
