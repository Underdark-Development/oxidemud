use tokio::sync::mpsc;

use mud_core::Entity;

pub trait Connection: Send {
    fn send(&mut self, text: &str);
    fn send_line(&mut self, text: &str);
    fn id(&self) -> u64;
    fn entity(&self) -> Option<Entity>;
    fn set_entity(&mut self, entity: Entity);
    fn disconnect(&mut self);
}

pub struct TelnetConnection {
    id: u64,
    entity: Option<Entity>,
    tx: Option<mpsc::UnboundedSender<String>>,
}

impl TelnetConnection {
    pub fn new(id: u64) -> (Self, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn = TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
        };
        (conn, rx)
    }
}

impl Connection for TelnetConnection {
    fn send(&mut self, text: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(text.to_string());
        }
    }

    fn send_line(&mut self, text: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(format!("{text}\r\n"));
        }
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn entity(&self) -> Option<Entity> {
        self.entity
    }

    fn set_entity(&mut self, entity: Entity) {
        self.entity = Some(entity);
    }

    fn disconnect(&mut self) {
        self.tx.take();
    }
}
