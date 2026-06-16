use ratatui::crossterm::event::{self as crossterm, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    Resize(u16, u16),
    Mouse(MouseEvent),
}

pub struct EventLoop {
    rx: mpsc::Receiver<Event>,
}

impl EventLoop {
    pub fn new() -> color_eyre::Result<Self> {
        let (tx, rx) = mpsc::channel(256);

        std::thread::spawn(move || loop {
            if crossterm::poll(Duration::from_millis(50)).unwrap_or(false) {
                match crossterm::read().unwrap_or(CrosstermEvent::Resize(0, 0)) {
                    CrosstermEvent::Key(key) => {
                        let _ = tx.blocking_send(Event::Key(key));
                    }
                    CrosstermEvent::Mouse(mouse) => {
                        let _ = tx.blocking_send(Event::Mouse(mouse));
                    }
                    CrosstermEvent::Resize(w, h) => {
                        let _ = tx.blocking_send(Event::Resize(w, h));
                    }
                    _ => {}
                }
            }
        });

        Ok(EventLoop { rx })
    }

    pub async fn next(&mut self) -> color_eyre::Result<Event> {
        tokio::select! {
            event = self.rx.recv() => {
                event.ok_or_else(|| color_eyre::eyre::eyre!("event channel closed"))
            }
            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                Ok(Event::Tick)
            }
        }
    }
}
