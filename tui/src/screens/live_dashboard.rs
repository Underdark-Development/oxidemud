use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyEvent,
    layout::Rect,
    style::{Color, Style},
};

use crate::screens::{Screen, ScreenAction};

pub struct LiveDashboardScreen {
    action: ScreenAction,
}

impl LiveDashboardScreen {
    pub fn new() -> Self {
        LiveDashboardScreen {
            action: ScreenAction::None,
        }
    }
}

impl Default for LiveDashboardScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for LiveDashboardScreen {
    fn name(&self) -> &str {
        "Live Dashboard"
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        if area.width < 4 || area.height < 2 {
            return;
        }

        let msg = "Live Dashboard / Server Status not yet implemented";
        let x = area.x + (area.width.saturating_sub(msg.len() as u16)) / 2;
        let y = area.y + area.height / 2;
        if y < area.y + area.height {
            buf.set_string(x, y, msg, Style::default().fg(Color::Indexed(245)));
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }
}
