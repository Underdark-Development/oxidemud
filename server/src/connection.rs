use tokio::sync::mpsc;

use mud_core::Entity;

use crate::telnet::{negotiate_echo, negotiate_no_echo};

// ---------------------------------------------------------------------------
// Connection Flags
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Connection Trait — pure transport
// ---------------------------------------------------------------------------

pub trait Connection: Send {
    fn send(&mut self, text: &str);
    fn send_line(&mut self, text: &str);
    fn send_raw(&mut self, bytes: &[u8]);
    fn id(&self) -> u64;
    fn entity(&self) -> Option<Entity>;
    fn set_entity(&mut self, entity: Entity);
    fn disconnect(&mut self);
    fn is_disconnected(&self) -> bool;
    fn flags(&self) -> ConnectionFlags;
    fn set_flags(&mut self, flags: ConnectionFlags);

    /// Player's preferred screen width in columns (0 = no wrap).
    fn screen_width(&self) -> u16 {
        0
    }

    fn set_screen_width(&mut self, _width: u16) {}

    /// Enable or disable server-side echo via telnet IAC WILL/WONT ECHO.
    /// WILL ECHO = server echoes (client hides input) — used for passwords.
    /// WONT ECHO = client does local echo — used for normal gameplay.
    fn set_echo(&mut self, echo_on: bool) {
        let bytes = if echo_on {
            negotiate_echo()
        } else {
            negotiate_no_echo()
        };
        self.send_raw(&bytes);
    }

    /// Returns a clone of the output sender, if available.
    fn output_sender(&self) -> Option<mpsc::UnboundedSender<Vec<u8>>>;
}

// ---------------------------------------------------------------------------
// TelnetConnection
// ---------------------------------------------------------------------------

type Output = Vec<u8>;

pub struct TelnetConnection {
    id: u64,
    entity: Option<Entity>,
    tx: Option<mpsc::UnboundedSender<Output>>,
    flags: ConnectionFlags,
    screen_width: u16,
}

impl TelnetConnection {
    pub fn new(id: u64) -> (Self, mpsc::UnboundedReceiver<Output>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn = TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            screen_width: 80,
        };
        (conn, rx)
    }

    pub fn new_with_tx(id: u64, tx: mpsc::UnboundedSender<Output>) -> Self {
        TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            screen_width: 80,
        }
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

    fn is_disconnected(&self) -> bool {
        self.tx.is_none()
    }

    fn flags(&self) -> ConnectionFlags {
        self.flags
    }

    fn set_flags(&mut self, flags: ConnectionFlags) {
        self.flags = flags;
    }

    fn screen_width(&self) -> u16 {
        self.screen_width
    }

    fn set_screen_width(&mut self, width: u16) {
        self.screen_width = width;
    }

    fn output_sender(&self) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        self.tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_entity_default() {
        let (conn, _) = TelnetConnection::new(1);
        assert!(conn.entity().is_none());
    }

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
