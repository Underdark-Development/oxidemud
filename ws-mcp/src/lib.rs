//! Shared WebSocket ↔ rmcp JSON-RPC codec used by both the `oxide-server`
//! (MCP *server* role over `/ws/mcp`) and the `oxide-mcp` crate (MCP *client*
//! role). It adapts any WebSocket that yields text frames into rmcp's
//! `Sink`/`Stream` item types so the protocol machinery is provided entirely by
//! rmcp (`SinkStreamTransport`) and tokio-tungstenite/axum — no framing or
//! handshake logic lives here.

use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Sink, Stream};
use rmcp::service::{RoleClient, RoleServer, RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};
use thiserror::Error;

/// A WebSocket frame abstraction implemented by both axum's and tungstenite's
/// `Message` types, so the codec is transport-agnostic.
pub trait WsFrame: Send + Unpin {
    /// Wrap a JSON text payload into a frame.
    fn from_text(text: String) -> Self;

    /// Borrow the text payload if this is a text frame.
    fn as_text(&self) -> Option<&str>;

    /// Whether this frame signals connection close.
    fn is_close(&self) -> bool;
}

impl WsFrame for tokio_tungstenite::tungstenite::Message {
    fn from_text(text: String) -> Self {
        tokio_tungstenite::tungstenite::Message::Text(text.into())
    }

    fn as_text(&self) -> Option<&str> {
        match self {
            tokio_tungstenite::tungstenite::Message::Text(t) => Some(t.as_str()),
            _ => None,
        }
    }

    fn is_close(&self) -> bool {
        matches!(self, tokio_tungstenite::tungstenite::Message::Close(_))
    }
}

impl WsFrame for axum::extract::ws::Message {
    fn from_text(text: String) -> Self {
        axum::extract::ws::Message::Text(text)
    }

    fn as_text(&self) -> Option<&str> {
        match self {
            axum::extract::ws::Message::Text(t) => Some(t.as_str()),
            _ => None,
        }
    }

    fn is_close(&self) -> bool {
        matches!(self, axum::extract::ws::Message::Close(_))
    }
}

/// Errors produced while adapting a WebSocket frame to/from an rmcp message.
#[derive(Debug, Error)]
pub enum WsCodecError {
    #[error("websocket closed")]
    Closed,
    #[error("websocket error: {0}")]
    Transport(String),
    #[error("json-rpc serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Adapts an underlying WebSocket (a `Stream<Item = Result<F, E>>` +
/// `Sink<F, Error = E>`) into rmcp's `Sink<TxJsonRpcMessage<Role>>` +
/// `Stream<Item = RxJsonRpcMessage<Role>>`, so it can be handed to
/// `rmcp::transport::IntoTransport` / `SinkStreamTransport`.
pub struct WsJsonRpcTransport<Role, F, S> {
    inner: S,
    _marker: PhantomData<fn(Role, F)>,
}

impl<Role, F, S> WsJsonRpcTransport<Role, F, S>
where
    Role: ServiceRole,
    F: WsFrame,
    S: Send + Unpin + 'static,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<Role, F, S> Sink<TxJsonRpcMessage<Role>> for WsJsonRpcTransport<Role, F, S>
where
    Role: ServiceRole,
    F: WsFrame,
    S: Sink<F> + Unpin,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = WsCodecError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner)
            .poll_ready(cx)
            .map_err(|e| WsCodecError::Transport(e.to_string()))
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: TxJsonRpcMessage<Role>,
    ) -> Result<(), Self::Error> {
        let text = serde_json::to_string(&item)?;
        Pin::new(&mut self.inner)
            .start_send(F::from_text(text))
            .map_err(|e| WsCodecError::Transport(e.to_string()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner)
            .poll_flush(cx)
            .map_err(|e| WsCodecError::Transport(e.to_string()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner)
            .poll_close(cx)
            .map_err(|e| WsCodecError::Transport(e.to_string()))
    }
}

impl<Role, F, S> Stream for WsJsonRpcTransport<Role, F, S>
where
    Role: ServiceRole,
    F: WsFrame,
    S: Sink<F> + Stream<Item = Result<F, S::Error>> + Unpin,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Item = RxJsonRpcMessage<Role>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(Err(_))) => return Poll::Ready(None),
                Poll::Ready(Some(Ok(frame))) => {
                    if frame.is_close() {
                        return Poll::Ready(None);
                    }
                    let Some(text) = frame.as_text() else {
                        continue; // skip ping/pong/binary
                    };
                    match serde_json::from_str::<RxJsonRpcMessage<Role>>(text) {
                        Ok(msg) => return Poll::Ready(Some(msg)),
                        Err(_) => continue, // skip non-JSON-RPC frames (e.g. greeting)
                    }
                }
            }
        }
    }
}

/// Convenience alias for the client role.
pub type WsClientTransport<F, S> = WsJsonRpcTransport<RoleClient, F, S>;
/// Convenience alias for the server role.
pub type WsServerTransport<F, S> = WsJsonRpcTransport<RoleServer, F, S>;

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::service::RoleClient;

    #[test]
    fn round_trip_client_message() {
        use rmcp::model::{ClientRequest, PingRequest, PingRequestMethod};
        let msg = TxJsonRpcMessage::<RoleClient>::request(
            ClientRequest::PingRequest(PingRequest {
                method: PingRequestMethod,
                extensions: Default::default(),
            }),
            rmcp::model::RequestId::Number(1),
        );
        let text = serde_json::to_string(&msg).unwrap();
        let parsed: RxJsonRpcMessage<RoleClient> = serde_json::from_str(&text).unwrap();
        // Round-trips through JSON without panicking; the untagged enum is
        // structurally identical on both sides.
        assert_eq!(serde_json::to_string(&parsed).unwrap(), text);
    }

    #[test]
    fn tungstenite_frame_adapter() {
        let frame = tokio_tungstenite::tungstenite::Message::from_text("{}".to_string());
        assert_eq!(frame.as_text(), Some("{}"));
        assert!(!frame.is_close());

        let close = tokio_tungstenite::tungstenite::Message::Close(None);
        assert!(close.is_close());
        assert_eq!(close.as_text(), None);
    }
}
