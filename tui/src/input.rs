use crate::app::App;
use crate::components::CommandAction;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    app.clear_hover();

    // Command palette is open: route to command palette
    if app.command_palette_open {
        if key.code == KeyCode::Esc
            || (key.code == KeyCode::Char('p') && key.modifiers == KeyModifiers::CONTROL)
        {
            app.command_palette_open = false;
            return;
        }
        if let Some(action) = app.command_palette.handle_key(key) {
            app.command_palette_open = false;
            app.handle_command_action(action);
        }
        return;
    }

    // Global: Ctrl+P to toggle command palette
    if key.code == KeyCode::Char('p') && key.modifiers == KeyModifiers::CONTROL {
        app.command_palette_open = true;
        app.command_palette.reset();
        return;
    }

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

    // Alt+letter: open menus (excluding digits)
    if key.modifiers.contains(KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char(c) if !c.is_ascii_digit())
    {
        if let Some(action) = app.menu_bar.handle_key(key) {
            app.handle_command_action(action);
        }
        return;
    }

    // F1..F6: screen switching
    if let KeyCode::F(n) = key.code {
        if (1..=6).contains(&n) {
            app.switch_screen((n - 1) as usize);
            return;
        }
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

    // Ctrl shortcuts (reload, save)
    if key.modifiers == KeyModifiers::CONTROL {
        match key.code {
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
