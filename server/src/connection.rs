use tokio::sync::mpsc;

use mud_core::Entity;

// ---------------------------------------------------------------------------
// Connection State Machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Username,
    Password {
        username: &'static str,
        attempts: u8,
    },
    AccountCreateConfirm {
        username: &'static str,
    },
    AccountCreatePassword,
    AccountCreateConfirmPassword,
    CharacterSelect,
    Playing,
}

impl ConnectionState {
    pub fn is_playing(&self) -> bool {
        matches!(self, ConnectionState::Playing)
    }
}

/// Temporary buffer for character creation wizard data.
#[derive(Debug, Clone, Default)]
pub struct CharacterCreateBuffer {
    pub name: Option<String>,
    pub race: Option<String>,
    pub class: Option<String>,
}

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
// Connection Trait
// ---------------------------------------------------------------------------

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

    // Login state machine
    fn state(&self) -> ConnectionState;
    fn set_state(&mut self, state: ConnectionState);
    fn create_buffer(&mut self) -> &mut CharacterCreateBuffer;
    fn account_id(&self) -> Option<i64>;
    fn set_account_id(&mut self, id: i64);
    fn strikes(&self) -> u8;
    fn set_strikes(&mut self, n: u8);

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
    state: ConnectionState,
    create_buf: CharacterCreateBuffer,
    conn_account_id: Option<i64>,
    conn_strikes: u8,
}

impl TelnetConnection {
    pub fn new(id: u64) -> (Self, mpsc::UnboundedReceiver<Output>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn = TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            state: ConnectionState::Connected,
            create_buf: CharacterCreateBuffer::default(),
            conn_account_id: None,
            conn_strikes: 0,
        };
        (conn, rx)
    }

    pub fn new_with_tx(id: u64, tx: mpsc::UnboundedSender<Output>) -> Self {
        TelnetConnection {
            id,
            entity: None,
            tx: Some(tx),
            flags: ConnectionFlags::new(),
            state: ConnectionState::Connected,
            create_buf: CharacterCreateBuffer::default(),
            conn_account_id: None,
            conn_strikes: 0,
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

    fn flags(&self) -> ConnectionFlags {
        self.flags
    }

    fn set_flags(&mut self, flags: ConnectionFlags) {
        self.flags = flags;
    }

    fn state(&self) -> ConnectionState {
        self.state
    }

    fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }

    fn create_buffer(&mut self) -> &mut CharacterCreateBuffer {
        &mut self.create_buf
    }

    fn account_id(&self) -> Option<i64> {
        self.conn_account_id
    }

    fn set_account_id(&mut self, id: i64) {
        self.conn_account_id = Some(id);
    }

    fn strikes(&self) -> u8 {
        self.conn_strikes
    }

    fn set_strikes(&mut self, n: u8) {
        self.conn_strikes = n;
    }

    fn output_sender(&self) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        self.tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_default() {
        let (conn, _) = TelnetConnection::new(1);
        assert_eq!(conn.state(), ConnectionState::Connected);
    }

    #[test]
    fn test_connection_state_transition() {
        let (mut conn, _) = TelnetConnection::new(1);
        conn.set_state(ConnectionState::Username);
        assert_eq!(conn.state(), ConnectionState::Username);
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

    #[test]
    fn test_create_buffer() {
        let (mut conn, _) = TelnetConnection::new(1);
        let buf = conn.create_buffer();
        assert!(buf.name.is_none());
        buf.name = Some("Test".to_string());
        assert_eq!(conn.create_buffer().name.as_deref(), Some("Test"));
    }

    #[test]
    fn test_account_id() {
        let (mut conn, _) = TelnetConnection::new(1);
        assert!(conn.account_id().is_none());
        conn.set_account_id(42);
        assert_eq!(conn.account_id(), Some(42));
    }

    #[test]
    fn test_strikes() {
        let (mut conn, _) = TelnetConnection::new(1);
        assert_eq!(conn.strikes(), 0);
        conn.set_strikes(2);
        assert_eq!(conn.strikes(), 2);
    }

    #[test]
    fn test_is_playing() {
        assert!(!ConnectionState::Connected.is_playing());
        assert!(!ConnectionState::Username.is_playing());
        assert!(ConnectionState::Playing.is_playing());
    }
}
