use crate::app::App;
use crate::components::CommandAction;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Menu is open: route everything to menu bar
    if app.menu_bar.open_menu.is_some() {
        if key.code == KeyCode::Esc {
            app.menu_bar.close_all();
            return;
        }
        if let Some(action) = app.menu_bar.handle_key(key) {
            app.handle_command_action(action);
        }
        return;
    }

    // Alt+letter: open menus
    if key.modifiers == KeyModifiers::ALT && matches!(key.code, KeyCode::Char(_)) {
        if let Some(action) = app.menu_bar.handle_key(key) {
            app.handle_command_action(action);
        }
        return;
    }

    // Global: Ctrl+D to quit
    if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
        app.should_quit = true;
        return;
    }

    // Global: Ctrl+B to toggle sidebar
    if key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL {
        app.sidebar_visible = !app.sidebar_visible;
        if app.sidebar_visible {
            app.sidebar_focused = true;
        }
        return;
    }

    // Ctrl shortcuts (screen switching, reload, save)
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
            KeyCode::Char('s') => {
                app.handle_command_action(CommandAction::SaveEntity);
                return;
            }
            _ => {}
        }
    }

    // Sidebar focused: dispatch to sidebar
    if app.sidebar_visible && app.sidebar_focused {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                app.sidebar_focused = false;
                app.active_screen_mut().sidebar_focus_lost(
                    key.code == KeyCode::BackTab || key.modifiers == KeyModifiers::SHIFT,
                );
                return;
            }
            KeyCode::Esc => {
                app.sidebar_focused = false;
                return;
            }
            KeyCode::Char('/') => {
                app.handle_command_action(CommandAction::ToggleSearch);
                app.sidebar_focused = false;
                return;
            }
            KeyCode::Char('r') => {
                app.handle_command_action(CommandAction::ReloadContent);
                return;
            }
            KeyCode::Char('?') => {
                app.handle_command_action(CommandAction::ToggleHelp);
                app.sidebar_focused = false;
                return;
            }
            _ => {
                if let Some(action) = app.command_sidebar.handle_key(key) {
                    app.sidebar_focused = false;
                    app.handle_command_action(action);
                }
                return;
            }
        }
    }

    // Tab/BackTab in main area: forward to screen first
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        let handled = app.active_screen_mut().handle_key(key);
        if !handled && app.sidebar_visible {
            app.sidebar_focused = true;
        }
        return;
    }

    // Forward to active screen
    app.active_screen_mut().handle_key(key);
}
