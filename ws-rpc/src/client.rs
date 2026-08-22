use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::MaybeTlsStream;

use crate::error::RpcError;
use crate::types::{Request, Response};

/// Timeout applied to a single in-flight RPC call.
const CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// JSON-RPC 2.0 error code for an unknown method.
const METHOD_NOT_FOUND_CODE: i64 = -32601;

type WsStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;
type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Response, RpcError>>>>>;

/// An async JSON-RPC 2.0 client speaking over a WebSocket transport.
///
/// `call` methods are safe to drive concurrently from many tasks; request ids are
/// allocated atomically and responses are correlated by id via the shared pending map.
pub struct RpcClient {
    writer: mpsc::UnboundedSender<Vec<u8>>,
    pending: PendingMap,
    next_id: AtomicU64,
}

impl RpcClient {
    /// Open a WebSocket connection and spawn the read/write background tasks.
    ///
    /// When `api_key` is provided it is attached to the upgrade handshake as an
    /// `Authorization: Bearer <api_key>` header.
    #[allow(clippy::result_large_err)] // RpcError carries a full tungstenite error by design.
    pub async fn connect(url: &str, api_key: Option<&str>) -> Result<Self, RpcError> {
        let mut request = url
            .to_string()
            .into_client_request()
            .map_err(RpcError::Transport)?;
        if let Some(key) = api_key {
            let value = http::HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|e| RpcError::Malformed(format!("invalid api key: {e}")))?;
            request
                .headers_mut()
                .insert(http::header::AUTHORIZATION, value);
        }
        let (ws_stream, _response) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(RpcError::Transport)?;
        let (mut write, read) = ws_stream.split();

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

        // Reader task: owns the read half + a clone of the pending map.
        {
            let pending = pending.clone();
            tokio::spawn(async move {
                reader(read, pending).await;
            });
        }

        // Writer task: owns the write half, draining an unbounded mpsc of serialized frames.
        let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            while let Some(bytes) = writer_rx.recv().await {
                if write.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
            let _ = write.close().await;
        });

        Ok(RpcClient {
            writer: writer_tx,
            pending,
            next_id: AtomicU64::new(0),
        })
    }

    /// Send a JSON-RPC request and await the deserialized `result`, or the server error.
    #[allow(clippy::result_large_err)] // RpcError carries a full tungstenite error by design.
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = Request {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params: Some(params),
        };
        let bytes = serde_json::to_vec(&request).map_err(|e| RpcError::Malformed(e.to_string()))?;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        if self.writer.send(bytes).is_err() {
            // Sending failed; no response can ever arrive for this id. Drop the
            // pending entry so the map does not accumulate stale senders.
            let _ = self.pending.lock().await.remove(&id);
            return Err(RpcError::Closed);
        }

        match tokio::time::timeout(CALL_TIMEOUT, rx).await {
            Ok(Ok(Ok(response))) => {
                if let Some(err) = response.error {
                    let msg = err.message;
                    return if err.code == METHOD_NOT_FOUND_CODE {
                        Err(RpcError::MethodNotFound(msg))
                    } else {
                        Err(RpcError::Server(format!("{msg} (code {})", err.code)))
                    };
                }
                Ok(response.result.unwrap_or(serde_json::Value::Null))
            }
            Ok(Ok(Err(rpc_err))) => Err(rpc_err),
            Ok(Err(_recv_err)) => Err(RpcError::Closed),
            Err(_elapsed) => {
                // Request timed out; drop the pending entry so the map does not
                // accumulate stale senders. Any late response is ignored safely.
                let _ = self.pending.lock().await.remove(&id);
                Err(RpcError::Timeout)
            }
        }
    }

    /// Send a JSON-RPC request and deserialize the `result` into `T`.
    #[allow(clippy::result_large_err)] // RpcError carries a full tungstenite error by design.
    pub async fn call_typed<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, RpcError> {
        let value = self.call(method, params).await?;
        serde_json::from_value(value).map_err(|e| RpcError::Malformed(e.to_string()))
    }
}

/// Drain the pending map, failing every outstanding request with `Closed`.
async fn fail_all(pending: &PendingMap) {
    let mut guard = pending.lock().await;
    for (_, sender) in guard.drain() {
        let _ = sender.send(Err(RpcError::Closed));
    }
}

/// Read loop that correlates inbound text frames with pending requests by id.
async fn reader(mut read: futures_util::stream::SplitStream<WsStream>, pending: PendingMap) {
    loop {
        match read.next().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<Response>(&text) {
                Ok(response) => {
                    if let Some(sender) = pending.lock().await.remove(&response.id) {
                        let _ = sender.send(Ok(response));
                    }
                }
                Err(_) => {
                    // Unexpected or malformed frame; release all waiters rather than hang.
                    fail_all(&pending).await;
                    break;
                }
            },
            Some(Ok(Message::Close(_))) => {
                fail_all(&pending).await;
                break;
            }
            Some(Err(_)) => {
                fail_all(&pending).await;
                break;
            }
            None => {
                // Stream ended: remote closed the socket.
                fail_all(&pending).await;
                break;
            }
            Some(Ok(_)) => {
                // Ignore ping/pong/binary/frame traffic that is not a text response.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let request = Request {
            jsonrpc: "2.0".into(),
            id: 42,
            method: "echo".into(),
            params: Some(serde_json::json!({"msg": "hi"})),
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 42);
        assert_eq!(json["method"], "echo");
        assert_eq!(json["params"]["msg"], "hi");

        let back: Request = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, request.id);
        assert_eq!(back.method, request.method);
    }

    #[test]
    fn response_error_roundtrip() {
        let response = Response {
            jsonrpc: "2.0".into(),
            id: 7,
            result: None,
            error: Some(crate::types::RpcErrorBody {
                code: -32601,
                message: "method not found".into(),
                data: None,
            }),
        };
        let json = serde_json::to_value(&response).unwrap();
        let back: Response = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, 7);
        assert!(back.error.is_some());
        assert_eq!(back.error.unwrap().code, -32601);
    }
}
