pub mod entities;
pub mod entity_inspector;
pub mod validation_panel;

use mud_core::templates::TemplateRegistry;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyEvent, MouseEvent},
    layout::Rect,
};

use crate::components::CommandAction;

#[derive(Debug, Clone)]
pub enum ScreenAction {
    None,
    Inspect(String, String),
    LoadScript(std::path::PathBuf),
}

/// Information about the currently selected entity in a screen.
#[derive(Debug, Clone)]
pub struct EntityContext {
    pub category: String,
    pub id: String,
    pub name: String,
}

pub trait Screen {
    fn name(&self) -> &str;
    fn handle_key(&mut self, key: KeyEvent) -> bool;
    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>);
    fn handle_mouse(&mut self, _mouse: MouseEvent, _area: Rect) {}
    fn reload(&mut self) {}

    /// Called each frame with the sidebar focus state so the screen can adjust its visual focus.
    fn set_sidebar_focused(&mut self, _focused: bool) {}

    /// Called when the sidebar releases focus back to the main area.
    /// `backward` is true if Shift+Tab was pressed, false for plain Tab.
    fn sidebar_focus_lost(&mut self, _backward: bool) {}

    /// Returns the number of unsaved changes in this screen.
    fn unsaved_count(&self) -> usize {
        0
    }

    /// Returns the currently selected entity, if any.
    fn selection_context(&self) -> Option<EntityContext> {
        None
    }

    /// Returns contextual commands for the current selection.
    /// Each entry is (display_label, CommandAction).
    fn contextual_commands(&self) -> Vec<(String, CommandAction)> {
        Vec::new()
    }

    /// Handle a command action from the sidebar. Returns Ok(true) if handled,
    /// Ok(false) if not handled, Err(msg) if handled but failed.
    fn handle_command_action(&mut self, _action: &CommandAction) -> Result<bool, String> {
        Ok(false)
    }

    fn take_action(&mut self) -> ScreenAction {
        ScreenAction::None
    }

    fn registry(&self) -> Option<&TemplateRegistry> {
        None
    }

    fn update_registry(&mut self, _registry: &TemplateRegistry) {}

    fn inspect_entity(&mut self, _category: &str, _id: &str) {}

    fn load_script_file(&mut self, _path: &std::path::Path) {}
}

pub mod file_browser;
pub mod live_dashboard;
pub mod room_grid;
pub mod script_console;

pub const SCREEN_TITLES: &[&str] = &[
    "Entities Editor",
    "Room Grid",
    "Validation Panel",
    "File Browser",
    "Script Console",
    "Live Dashboard",
];

pub struct PlaceholderScreen {
    name: String,
}

impl PlaceholderScreen {
    pub fn new(name: &str) -> Self {
        PlaceholderScreen {
            name: name.to_string(),
        }
    }
}

impl Screen for PlaceholderScreen {
    fn name(&self) -> &str {
        &self.name
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn reload(&mut self) {}

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        let msg = format!(" {} — coming soon ", self.name);
        let x = area.x + (area.width.saturating_sub(msg.len() as u16)) / 2;
        let y = area.y + area.height / 2;
        if y < area.y + area.height {
            buf.set_string(
                x,
                y,
                &msg,
                ratatui::style::Style::default().fg(ratatui::style::Color::Indexed(245)),
            );
        }
    }
}
