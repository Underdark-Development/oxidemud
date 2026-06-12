use tokio::sync::mpsc;

use mud_core::Entity;

/// Feature flags a connection may support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionFlag {
    Ansi,
    ExtendedColor,
    Blink,
}

/// Bitmask of [`ConnectionFlag`] values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionFlags(u8);

impl ConnectionFlags {
    pub const ANSI: u8 = 0x01;
    pub const EXTENDED_COLOR: u8 = 0x02;
    pub const BLINK: u8 = 0x04;

    pub fn new() -> Self {
        ConnectionFlags(Self::ANSI)
    }

    pub fn set(&mut self, flag: ConnectionFlag, value: bool) {
        let bit = match flag {
            ConnectionFlag::Ansi => Self::ANSI,
            ConnectionFlag::ExtendedColor => Self::EXTENDED_COLOR,
            ConnectionFlag::Blink => Self::BLINK,
        };
        if value {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }

    pub fn has(&self, flag: ConnectionFlag) -> bool {
        let bit = match flag {
            ConnectionFlag::Ansi => Self::ANSI,
            ConnectionFlag::ExtendedColor => Self::EXTENDED_COLOR,
            ConnectionFlag::Blink => Self::BLINK,
        };
        self.0 & bit != 0
    }
}

impl Default for ConnectionFlags {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Connection: Send {
    fn send(&mut self, text: &str);
    fn send_line(&mut self, text: &str);
    fn send_raw(&mut self, bytes: &[u8]);
    fn id(&self) -> u64;
    fn entity(&self) -> Option<Entity>;
    fn set_entity(&mut self, entity: Entity);
    fn disconnect(&mut self);
    fn flags(&self) -> ConnectionFlags;
    fn set_flags(&mut self, flags: ConnectionFlags);
}

type Output = Vec<u8>;

pub struct TelnetConnection {
    id: u64,
    entity: Option<Entity>,
    tx: Option<mpsc::UnboundedSender<Output>>,
    flags: ConnectionFlags,
}

impl TelnetConnection {
    pub fn new(id: u64) -> (Self, mpsc::UnboundedReceiver<Output>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn = TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
        };
        (conn, rx)
    }
}

impl Connection for TelnetConnection {
    fn send(&mut self, text: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(text.as_bytes().to_vec());
        }
    }

    fn send_line(&mut self, text: &str) {
        if let Some(tx) = &self.tx {
            let mut bytes = text.as_bytes().to_vec();
            bytes.extend_from_slice(b"\r\n");
            let _ = tx.send(bytes);
        }
    }

    fn send_raw(&mut self, bytes: &[u8]) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(bytes.to_vec());
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

    fn flags(&self) -> ConnectionFlags {
        self.flags
    }

    fn set_flags(&mut self, flags: ConnectionFlags) {
        self.flags = flags;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_flags_default() {
        let f = ConnectionFlags::new();
        assert!(f.has(ConnectionFlag::Ansi));
        assert!(!f.has(ConnectionFlag::ExtendedColor));
        assert!(!f.has(ConnectionFlag::Blink));
    }

    #[test]
    fn test_connection_flags_set() {
        let mut f = ConnectionFlags::new();
        f.set(ConnectionFlag::Ansi, false);
        assert!(!f.has(ConnectionFlag::Ansi));
        f.set(ConnectionFlag::Blink, true);
        assert!(f.has(ConnectionFlag::Blink));
    }

    #[test]
    fn test_telnet_connection_flags() {
        let (mut conn, _) = TelnetConnection::new(1);
        assert!(conn.flags().has(ConnectionFlag::Ansi));
        let mut flags = conn.flags();
        flags.set(ConnectionFlag::ExtendedColor, true);
        conn.set_flags(flags);
        assert!(conn.flags().has(ConnectionFlag::ExtendedColor));
    }
}
