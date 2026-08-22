//! WebSocket MCP *client* transport for the `oxide-mcp` crate.
//!
//! Connects to a running OxideMUD server's `/ws/mcp` endpoint over WebSocket
//! (WS/WSS) and adapts the socket into rmcp's client transport via the shared
//! [`oxide_ws_mcp::WsClientTransport`] codec. The JSON-RPC protocol machinery is
//! provided entirely by rmcp (`serve_client` / `SinkStreamTransport`); this
//! module only handles connection establishment and bearer-token auth.

use oxide_ws_mcp::WsClientTransport;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

/// A connected WebSocket MCP client transport.
pub type WsMcpClient = WsClientTransport<WsMessage, WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Establish a WebSocket connection to an MCP endpoint, attaching an optional
/// bearer token to the upgrade request.
pub async fn connect_ws(url: &str, api_key: Option<&str>) -> anyhow::Result<WsMcpClient> {
    let mut req = url.into_client_request()?;
    if let Some(key) = api_key {
        let value = http::HeaderValue::from_str(&format!("Bearer {key}"))?;
        req.headers_mut().insert(http::header::AUTHORIZATION, value);
    }
    let (socket, _resp) = connect_async(req).await?;
    Ok(WsClientTransport::new(socket))
}
