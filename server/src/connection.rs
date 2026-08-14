use tokio::sync::mpsc;

use oxide_core::{AccessLevel, Entity};

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
    fn id(&self) -> &str;
    fn entity(&self) -> Option<Entity>;
    fn set_entity(&mut self, entity: Entity);
    fn disconnect(&mut self);
    fn is_disconnected(&self) -> bool;
    fn flags(&self) -> ConnectionFlags;
    fn set_flags(&mut self, flags: ConnectionFlags);
    fn access_level(&self) -> AccessLevel {
        AccessLevel::Player
    }
    fn set_access_level(&mut self, _level: AccessLevel) {}

    /// Player's preferred screen width in columns (0 = no wrap).
    fn screen_width(&self) -> u16 {
        0
    }

    fn set_screen_width(&mut self, _width: u16) {}

    /// Player's terminal type if negotiated (e.g. "xterm-256color").
    fn terminal_type(&self) -> Option<String> {
        None
    }

    fn set_terminal_type(&mut self, _term_type: String) {}

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
    id: String,
    entity: Option<Entity>,
    tx: Option<mpsc::UnboundedSender<Output>>,
    flags: ConnectionFlags,
    screen_width: u16,
    access_level: AccessLevel,
    terminal_type: Option<String>,
}

impl TelnetConnection {
    pub fn new(id: String) -> (Self, mpsc::UnboundedReceiver<Output>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn = TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            screen_width: 80,
            access_level: AccessLevel::Player,
            terminal_type: None,
        };
        (conn, rx)
    }

    pub fn new_with_tx(id: String, tx: mpsc::UnboundedSender<Output>) -> Self {
        TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            screen_width: 80,
            access_level: AccessLevel::Player,
            terminal_type: None,
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

    fn id(&self) -> &str {
        &self.id
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

    fn terminal_type(&self) -> Option<String> {
        self.terminal_type.clone()
    }

    fn set_terminal_type(&mut self, term_type: String) {
        self.terminal_type = Some(term_type);
    }

    fn access_level(&self) -> AccessLevel {
        self.access_level
    }

    fn set_access_level(&mut self, level: AccessLevel) {
        self.access_level = level;
    }

    fn output_sender(&self) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        self.tx.clone()
    }
}

// ---------------------------------------------------------------------------
// WsConnection
// ---------------------------------------------------------------------------

pub struct WsConnection {
    id: String,
    entity: Option<Entity>,
    tx: Option<mpsc::UnboundedSender<Output>>,
    flags: ConnectionFlags,
    screen_width: u16,
    access_level: AccessLevel,
    terminal_type: Option<String>,
}

impl WsConnection {
    pub fn new(id: String) -> (Self, mpsc::UnboundedReceiver<Output>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn = WsConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            screen_width: 80,
            access_level: AccessLevel::Player,
            terminal_type: Some("websocket".to_string()),
        };
        (conn, rx)
    }

    pub fn new_with_tx(id: String, tx: mpsc::UnboundedSender<Output>) -> Self {
        WsConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            screen_width: 80,
            access_level: AccessLevel::Player,
            terminal_type: Some("websocket".to_string()),
        }
    }
}

impl Connection for WsConnection {
    fn send(&mut self, text: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(text.as_bytes().to_vec());
        }
    }

    fn send_line(&mut self, text: &str) {
        if let Some(tx) = &self.tx {
            let mut bytes = text.as_bytes().to_vec();
            bytes.extend_from_slice(b"\n");
            let _ = tx.send(bytes);
        }
    }

    fn send_raw(&mut self, bytes: &[u8]) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(bytes.to_vec());
        }
    }

    fn id(&self) -> &str {
        &self.id
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

    fn terminal_type(&self) -> Option<String> {
        self.terminal_type.clone()
    }

    fn set_terminal_type(&mut self, term_type: String) {
        self.terminal_type = Some(term_type);
    }

    fn access_level(&self) -> AccessLevel {
        self.access_level
    }

    fn set_access_level(&mut self, level: AccessLevel) {
        self.access_level = level;
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
        let (conn, _) = TelnetConnection::new("1".to_string());
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
        let (mut conn, _) = TelnetConnection::new("1".to_string());
        assert!(conn.flags().has(ConnectionFlag::Ansi));
        let mut flags = conn.flags();
        flags.set(ConnectionFlag::ExtendedColor, true);
        conn.set_flags(flags);
        assert!(conn.flags().has(ConnectionFlag::ExtendedColor));
    }

    #[test]
    fn test_ws_connection_send() {
        let (mut conn, mut rx) = WsConnection::new("ws-1".to_string());
        assert_eq!(conn.id(), "ws-1");
        assert_eq!(conn.terminal_type(), Some("websocket".to_string()));

        conn.send_line("hello ws");
        let msg = rx.try_recv().unwrap();
        assert_eq!(String::from_utf8(msg).unwrap(), "hello ws\n");
    }
}
