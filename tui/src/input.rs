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

    // Screen-level modal overlay (help, preview, confirm dialog): route all
    // keys to the screen and suppress global shortcuts.
    if app.active_screen().modal_overlay_active() {
        app.active_screen_mut().handle_key(key);
        return;
    }

    // Global: Ctrl+P / Ctrl+Shift+P / Cmd+Shift+P to toggle command palette
    if (key.code == KeyCode::Char('p') || key.code == KeyCode::Char('P'))
        && (key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::SUPER))
    {
        app.command_palette_open = true;
        app.command_palette.reset();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Mode};
    use crate::config::Config;
    use crate::config_file::SpadeConfig;

    fn test_app() -> App {
        let cli = Config {
            mode: Mode::Offline,
            connect_host: None,
            connect_port: None,
            subcommand: None,
        };
        let config = SpadeConfig {
            content_path: std::env::temp_dir()
                .join("spade-nonexistent-content")
                .to_string_lossy()
                .into_owned(),
            ..Default::default()
        };
        App::new(cli, config)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn open_menu(app: &mut App, hotkey: char) {
        app.menu_bar
            .handle_key(KeyEvent::new(KeyCode::Char(hotkey), KeyModifiers::ALT));
        assert!(app.menu_bar.open_menu.is_some());
    }

    #[test]
    fn open_menu_swallows_ctrl_p() {
        let mut app = test_app();
        open_menu(&mut app, 'w');

        handle_key(&mut app, ctrl(KeyCode::Char('p')));

        assert!(!app.command_palette_open);
        assert!(app.menu_bar.open_menu.is_some());
    }

    #[test]
    fn open_menu_swallows_global_shortcuts() {
        let mut app = test_app();
        open_menu(&mut app, 'v');

        handle_key(&mut app, key(KeyCode::F(2)));
        handle_key(&mut app, ctrl(KeyCode::Char('d')));
        handle_key(&mut app, ctrl(KeyCode::Char('s')));

        assert_eq!(app.active_screen, ScreenId::Entities);
        assert!(app.quit_dialog.is_none());
        assert!(app.menu_bar.open_menu.is_some());
    }

    #[test]
    fn screen_modal_swallows_global_shortcuts() {
        let mut app = test_app();
        app.active_screen_mut().handle_key(key(KeyCode::Char('?')));
        assert!(app.active_screen().modal_overlay_active());

        handle_key(&mut app, ctrl(KeyCode::Char('p')));
        handle_key(&mut app, key(KeyCode::F(2)));
        handle_key(&mut app, ctrl(KeyCode::Char('d')));
        handle_key(&mut app, ctrl(KeyCode::Char('b')));

        assert!(!app.command_palette_open);
        assert_eq!(app.active_screen, ScreenId::Entities);
        assert!(app.quit_dialog.is_none());
        assert!(app.sidebar_visible);
        assert!(app.active_screen().modal_overlay_active());
    }

    #[test]
    fn esc_closes_screen_modal() {
        let mut app = test_app();
        app.active_screen_mut().handle_key(key(KeyCode::Char('?')));
        assert!(app.active_screen().modal_overlay_active());

        handle_key(&mut app, key(KeyCode::Esc));

        assert!(!app.active_screen().modal_overlay_active());
    }

    #[test]
    fn no_overlay_global_shortcuts_work() {
        let mut app = test_app();

        handle_key(&mut app, ctrl(KeyCode::Char('p')));
        assert!(app.command_palette_open);

        // Palette captures subsequent keys; Esc closes it.
        handle_key(&mut app, key(KeyCode::Char('x')));
        assert!(app.command_palette_open);
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.command_palette_open);

        handle_key(&mut app, key(KeyCode::F(2)));
        assert_eq!(app.active_screen, ScreenId::RoomGrid);

        handle_key(&mut app, ctrl(KeyCode::Char('b')));
        assert!(!app.sidebar_visible);
    }
}
