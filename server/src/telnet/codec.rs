use std::collections::VecDeque;
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

use super::constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    GotIac,
    GotWill,
    GotWont,
    GotDo,
    GotDont,
    Subneg,
    SubnegGotIac,
}

#[derive(Debug)]
pub struct TelnetReader<R> {
    inner: R,
    state: State,
    pending: [u8; 4096],
    pending_len: usize,
    pending_pos: usize,
    negotiations: VecDeque<Negotiation>,
    subneg_buf: Vec<u8>,
}

impl<R: AsyncRead + Unpin> TelnetReader<R> {
    pub fn new(inner: R) -> Self {
        TelnetReader {
            inner,
            state: State::Normal,
            pending: [0u8; 4096],
            pending_len: 0,
            pending_pos: 0,
            negotiations: VecDeque::new(),
            subneg_buf: Vec::new(),
        }
    }

    /// Drain all buffered client negotiations received since the last call.
    pub fn take_negotiations(&mut self) -> Vec<Negotiation> {
        self.negotiations.drain(..).collect()
    }

    fn process_byte(&mut self, b: u8) {
        match self.state {
            State::Normal => {
                if b == constants::IAC {
                    self.state = State::GotIac;
                } else {
                    self.pending[self.pending_len] = b;
                    self.pending_len += 1;
                }
            }
            State::GotIac => {
                match b {
                    constants::WILL => self.state = State::GotWill,
                    constants::WONT => self.state = State::GotWont,
                    constants::DO => self.state = State::GotDo,
                    constants::DONT => self.state = State::GotDont,
                    constants::SB => {
                        self.subneg_buf.clear();
                        self.state = State::Subneg;
                    }
                    constants::IAC => {
                        // Escaped 0xFF
                        self.pending[self.pending_len] = b;
                        self.pending_len += 1;
                        self.state = State::Normal;
                    }
                    // NOP, DM, BRK, IP, AO, AYT, EC, EL, GA — ignore
                    _ => self.state = State::Normal,
                }
            }
            State::GotWill => {
                let action = NegotiationAction::Will(b);
                self.negotiations.push_back(Negotiation { action });
                self.state = State::Normal;
            }
            State::GotWont => {
                let action = NegotiationAction::Wont(b);
                self.negotiations.push_back(Negotiation { action });
                self.state = State::Normal;
            }
            State::GotDo => {
                let action = NegotiationAction::Do(b);
                self.negotiations.push_back(Negotiation { action });
                self.state = State::Normal;
            }
            State::GotDont => {
                let action = NegotiationAction::Dont(b);
                self.negotiations.push_back(Negotiation { action });
                self.state = State::Normal;
            }
            State::Subneg => {
                if b == constants::IAC {
                    self.state = State::SubnegGotIac;
                } else {
                    self.subneg_buf.push(b);
                }
            }
            State::SubnegGotIac => {
                if b == constants::SE {
                    self.state = State::Normal;
                    if !self.subneg_buf.is_empty() {
                        let opt = self.subneg_buf[0];
                        let params = self.subneg_buf[1..].to_vec();
                        self.negotiations.push_back(Negotiation {
                            action: NegotiationAction::Subneg(opt, params),
                        });
                    }
                } else if b == constants::IAC {
                    // Escaped IAC inside subnegotiation, stay in subneg
                    self.subneg_buf.push(constants::IAC);
                    self.state = State::Subneg;
                } else {
                    // Wasn't SE or escaped IAC, go back to subneg and treat the IAC as data
                    self.subneg_buf.push(constants::IAC);
                    self.subneg_buf.push(b);
                    self.state = State::Subneg;
                }
            }
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for TelnetReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        loop {
            // If we have pending processed bytes, return them
            if self.pending_pos < self.pending_len {
                let available = self.pending_len - self.pending_pos;
                let to_copy = available.min(buf.remaining());
                buf.put_slice(&self.pending[self.pending_pos..self.pending_pos + to_copy]);
                self.pending_pos += to_copy;

                if self.pending_pos == self.pending_len {
                    self.pending_pos = 0;
                    self.pending_len = 0;
                }

                return Poll::Ready(Ok(()));
            }

            // Need to read more raw bytes
            let mut raw_buf = [0u8; 1024];
            let mut raw = ReadBuf::new(&mut raw_buf);
            match Pin::new(&mut self.inner).poll_read(cx, &mut raw) {
                Poll::Ready(Ok(())) => {
                    let filled = raw.filled();
                    if filled.is_empty() {
                        // EOF
                        return Poll::Ready(Ok(()));
                    }
                    for &b in filled {
                        self.process_byte(b);
                    }
                    // Loop back to return processed bytes
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationAction {
    Will(u8),
    Wont(u8),
    Do(u8),
    Dont(u8),
    Subneg(u8, Vec<u8>),
}

/// Represents a single telnet negotiation received from the client.
/// Extracted by the codec during processing.
#[derive(Debug, Clone)]
pub struct Negotiation {
    pub action: NegotiationAction,
}

/// Build raw IAC negotiation bytes to send to the client.
pub fn build_negotiation(action: NegotiationAction) -> Vec<u8> {
    match action {
        NegotiationAction::Will(o) => vec![constants::IAC, constants::WILL, o],
        NegotiationAction::Wont(o) => vec![constants::IAC, constants::WONT, o],
        NegotiationAction::Do(o) => vec![constants::IAC, constants::DO, o],
        NegotiationAction::Dont(o) => vec![constants::IAC, constants::DONT, o],
        NegotiationAction::Subneg(o, params) => {
            let mut v = vec![constants::IAC, constants::SB, o];
            v.extend_from_slice(&params);
            v.push(constants::IAC);
            v.push(constants::SE);
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt};

    #[tokio::test]
    async fn test_passthrough_plain_text() {
        let input = b"hello world\n";
        let reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "hello world\n");
    }

    #[tokio::test]
    async fn test_strips_iac_will() {
        // IAC WILL ECHO (3 bytes) followed by text
        let input = [255, 251, 1, b'h', b'i', b'\n'];
        let reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "hi\n");
    }

    #[tokio::test]
    async fn test_strips_iac_do() {
        let input = [255, 253, 3, b't', b'e', b's', b't', b'\n'];
        let reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "test\n");
    }

    #[tokio::test]
    async fn test_escaped_iac() {
        // IAC IAC should become a single 0xFF byte
        let input = [255, 255, b'\n'];
        let reader = TelnetReader::new(&input[..]);
        let mut buf = Vec::new();
        tokio::io::BufReader::new(reader)
            .read_to_end(&mut buf)
            .await
            .unwrap();
        assert_eq!(buf, vec![255, b'\n']);
    }

    #[tokio::test]
    async fn test_strips_subnegotiation() {
        // IAC SB NAWS ... IAC SE followed by text
        let mut input = Vec::new();
        input.extend_from_slice(&[255, 250, 31, 0, 80, 0, 24, 255, 240]); // NAWS
        input.extend_from_slice(b"data\n");
        let reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "data\n");
    }

    #[tokio::test]
    async fn test_multiple_iac_sequences() {
        let mut input = Vec::new();
        input.extend_from_slice(&[255, 251, 1]); // IAC WILL ECHO
        input.extend_from_slice(b"look");
        input.extend_from_slice(&[255, 252, 3]); // IAC WONT SGA
        input.extend_from_slice(b"\n");
        let reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "look\n");
    }

    #[tokio::test]
    async fn test_multiple_lines() {
        let input = b"first line\nsecond line\nthird line\n";
        let reader = TelnetReader::new(&input[..]);
        let mut buf_reader = tokio::io::BufReader::new(reader);
        let mut line = String::new();

        buf_reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "first line\n");

        line.clear();
        buf_reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "second line\n");

        line.clear();
        buf_reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "third line\n");
    }

    #[tokio::test]
    async fn test_naws_subnegotiation_parsing() {
        // IAC SB NAWS 0 80 0 24 IAC SE followed by text
        let mut input = Vec::new();
        input.extend_from_slice(&[255, 250, 31, 0, 80, 0, 24, 255, 240]);
        input.extend_from_slice(b"hello\n");
        let mut reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(&mut reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "hello\n");

        let negotiations = reader.take_negotiations();
        assert_eq!(negotiations.len(), 1);
        match &negotiations[0].action {
            NegotiationAction::Subneg(31, params) => {
                assert_eq!(params, &vec![0, 80, 0, 24]);
            }
            other => panic!("expected Subneg(31, [0, 80, 0, 24]), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_terminal_type_subnegotiation_parsing() {
        // IAC SB TERMINAL_TYPE IS xterm-256color IAC SE
        let mut input = Vec::new();
        input.extend_from_slice(&[255, 250, 24, 0]);
        input.extend_from_slice(b"xterm-256color");
        input.extend_from_slice(&[255, 240]);
        input.extend_from_slice(b"cmd\n");
        let mut reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(&mut reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "cmd\n");

        let negotiations = reader.take_negotiations();
        assert_eq!(negotiations.len(), 1);
        match &negotiations[0].action {
            NegotiationAction::Subneg(24, params) => {
                assert_eq!(params[0], 0);
                assert_eq!(std::str::from_utf8(&params[1..]).unwrap(), "xterm-256color");
            }
            other => panic!("expected Subneg(24, ...), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_subnegotiation_with_escaped_iac() {
        // IAC SB 99 1 2 255 255 3 IAC SE followed by text
        let mut input = Vec::new();
        input.extend_from_slice(&[255, 250, 99, 1, 2, 255, 255, 3, 255, 240]);
        input.extend_from_slice(b"done\n");
        let mut reader = TelnetReader::new(&input[..]);
        let mut output = String::new();
        tokio::io::BufReader::new(&mut reader)
            .read_line(&mut output)
            .await
            .unwrap();
        assert_eq!(output, "done\n");

        let negotiations = reader.take_negotiations();
        assert_eq!(negotiations.len(), 1);
        match &negotiations[0].action {
            NegotiationAction::Subneg(99, params) => {
                assert_eq!(params, &vec![1, 2, 255, 3]);
            }
            other => panic!("expected Subneg(99, [1, 2, 255, 3]), got {:?}", other),
        }
    }
}
