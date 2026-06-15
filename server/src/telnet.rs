pub mod constants {
    pub const IAC: u8 = 255;
    pub const WILL: u8 = 251;
    pub const WONT: u8 = 252;
    pub const DO: u8 = 253;
    pub const DONT: u8 = 254;
    pub const SB: u8 = 250;
    pub const SE: u8 = 240;

    pub const ECHO: u8 = 1;
    pub const SUPPRESS_GO_AHEAD: u8 = 3;
    pub const STATUS: u8 = 5;
    pub const LINEMODE: u8 = 34;
    pub const NAWS: u8 = 31;
    pub const TERMINAL_TYPE: u8 = 24;
    pub const NEW_ENVIRON: u8 = 39;
    pub const MSSP: u8 = 70;
    pub const MXP: u8 = 91;
    pub const GMCP: u8 = 201;
    pub const MCCP1: u8 = 85;
    pub const MCCP2: u8 = 86;
    pub const MCCP3: u8 = 87;
}

pub mod codec;

fn iac_bytes(cmd: u8, option: u8) -> [u8; 3] {
    [constants::IAC, cmd, option]
}

pub fn negotiate_will(option: u8) -> [u8; 3] {
    iac_bytes(constants::WILL, option)
}

pub fn negotiate_wont(option: u8) -> [u8; 3] {
    iac_bytes(constants::WONT, option)
}

pub fn negotiate_do(option: u8) -> [u8; 3] {
    iac_bytes(constants::DO, option)
}

pub fn negotiate_dont(option: u8) -> [u8; 3] {
    iac_bytes(constants::DONT, option)
}

pub fn negotiate_echo() -> [u8; 3] {
    negotiate_will(constants::ECHO)
}

pub fn negotiate_no_echo() -> [u8; 3] {
    negotiate_wont(constants::ECHO)
}

pub fn negotiate_suppress_go_ahead() -> [u8; 3] {
    negotiate_will(constants::SUPPRESS_GO_AHEAD)
}

/// Sent to every new connection: server won't echo (local echo on),
/// server will suppress go-ahead (line-at-a-time mode).
pub const INITIAL_NEGOTIATION: [u8; 6] = [
    constants::IAC,
    constants::WONT,
    constants::ECHO,
    constants::IAC,
    constants::WILL,
    constants::SUPPRESS_GO_AHEAD,
];
