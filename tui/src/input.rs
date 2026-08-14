use crate::app::App;
use crate::components::CommandAction;
use crate::screens::ScreenId;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    app.clear_hover();

    // Quit dialog is active: route all keys to quit dialog
    if app.quit_dialog.is_some() {
        if key.code == KeyCode::Esc {
            app.quit_dialog = None;
            return;
        }
        if let Some(ref mut dialog) = app.quit_dialog {
            if let Some(btn) = dialog.handle_key(key) {
                if btn == 0 {
                    app.quit_dialog = None;
                } else if btn == 1 {
                    app.quit_dialog = None;
                    app.handle_command_action(CommandAction::SaveEntity);
                    app.should_quit = true;
                } else if btn == 2 {
                    app.quit_dialog = None;
                    app.should_quit = true;
                }
            }
        }
        return;
    }

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
        if let Some(id) = ScreenId::from_fkey(n) {
            app.switch_screen(id);
            return;
        }
    }

    // Global: Ctrl+D to quit
    if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
        app.confirm_quit();
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
