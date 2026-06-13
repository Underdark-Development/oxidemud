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
                    constants::SB => self.state = State::Subneg,
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
                }
                // otherwise ignore subnegotiation bytes
            }
            State::SubnegGotIac => {
                if b == constants::SE {
                    self.state = State::Normal;
                } else if b == constants::IAC {
                    // Escaped IAC inside subnegotiation, stay in subneg
                    self.state = State::Subneg;
                } else {
                    // Wasn't SE, go back to subneg
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

#[derive(Debug, Clone, Copy)]
pub enum NegotiationAction {
    Will(u8),
    Wont(u8),
    Do(u8),
    Dont(u8),
}

/// Represents a single telnet negotiation received from the client.
/// Extracted by the codec during processing.
#[derive(Debug, Clone)]
pub struct Negotiation {
    pub action: NegotiationAction,
}

/// Build raw IAC negotiation bytes to send to the client.
pub fn build_negotiation(action: NegotiationAction) -> [u8; 3] {
    let (cmd, option) = match action {
        NegotiationAction::Will(o) => (constants::WILL, o),
        NegotiationAction::Wont(o) => (constants::WONT, o),
        NegotiationAction::Do(o) => (constants::DO, o),
        NegotiationAction::Dont(o) => (constants::DONT, o),
    };
    [constants::IAC, cmd, option]
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
}
