pub mod entity_inspector;
pub mod validation_panel;
pub mod world_tree;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyEvent, MouseEvent},
    layout::Rect,
};

#[derive(Debug, Clone)]
pub enum ScreenAction {
    None,
    Inspect(String, String),
}

pub trait Screen {
    fn name(&self) -> &str;
    fn handle_key(&mut self, key: KeyEvent);
    fn render(&mut self, area: Rect, buf: &mut Buffer);
    fn handle_mouse(&mut self, _mouse: MouseEvent, _area: Rect) {}
    fn reload(&mut self) {}
    fn take_action(&mut self) -> ScreenAction {
        ScreenAction::None
    }
}

pub const SCREEN_TITLES: &[&str] = &[
    "World Tree",
    "Template Editor",
    "Room Graph",
    "Entity Inspector",
    "Command Palette",
    "Live Dashboard",
    "Validation Panel",
    "File Browser",
    "Script Console",
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

    fn handle_key(&mut self, _key: KeyEvent) {}
    fn reload(&mut self) {}

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let msg = format!(" {} — coming soon ", self.name);
        let x = area.x + (area.width.saturating_sub(msg.len() as u16)) / 2;
        let y = area.y + area.height / 2;
        if y < area.y + area.height {
            buf.set_string(
                x,
                y,
                &msg,
                ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
            );
        }
    }
}
