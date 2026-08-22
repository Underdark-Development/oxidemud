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
    writer: mpsc::UnboundedSender<Message>,
    pending: PendingMap,
    next_id: AtomicU64,
}

impl RpcClient {
    /// Open a WebSocket connection and spawn the read/write background tasks.
    ///
    /// When `api_key` is provided it is attached to the upgrade handshake as an
    /// `Authorization: Bearer *** header.
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

        // The writer channel carries raw `Message` frames so the reader can
        // route a keepalive `Pong` back through the writer task (which owns the
        // write half) — requests ride the same channel as `Message::Text`.
        let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<Message>();

        // Reader task: owns the read half + a clone of the pending map and an
        // outbound channel so it can answer server pings through the writer.
        {
            let pending = pending.clone();
            let writer = writer_tx.clone();
            tokio::spawn(async move {
                reader(read, pending, writer).await;
            });
        }

        // Writer task: owns the write half, draining an unbounded mpsc and
        // forwarding whatever frame arrives (a request Text or a keepalive Pong).
        tokio::spawn(async move {
            while let Some(data) = writer_rx.recv().await {
                if write.send(data).await.is_err() {
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
        let json =
            serde_json::to_string(&request).map_err(|e| RpcError::Malformed(e.to_string()))?;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        if self.writer.send(Message::Text(json.into())).is_err() {
            // Sending failed; no response can ever arrive for this id. Drop the
            // pending entry so the map does not accumulate stale senders.
            let _ = self.pending.lock().await.remove(&id);
            return Err(RpcError::Closed);
        }

        match tokio::time::timeout(CALL_TIMEOUT, rx).await {
            Ok(Ok(Ok(response))) => response_to_result(response),
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

/// Map a correlated JSON-RPC response to its `result`, projecting server
/// errors onto `RpcError`. A response with NEITHER `result` NOR `error` set is
/// malformed JSON-RPC 2.0 (it must carry exactly one), so it yields
/// `RpcError::Malformed` rather than a silent `null`.
#[allow(clippy::result_large_err)] // RpcError carries a full tungstenite error by design.
fn response_to_result(response: Response) -> Result<serde_json::Value, RpcError> {
    if let Some(err) = response.error {
        let msg = err.message;
        return if err.code == METHOD_NOT_FOUND_CODE {
            Err(RpcError::MethodNotFound(msg))
        } else {
            Err(RpcError::Server(format!("{msg} (code {})", err.code)))
        };
    }
    match response.result {
        Some(result) => Ok(result),
        None => Err(RpcError::Malformed(format!(
            "response (id {}) has neither result nor error",
            response.id
        ))),
    }
}

/// Drain the pending map, failing every outstanding request with `Closed`.
async fn fail_all(pending: &PendingMap) {
    let mut guard = pending.lock().await;
    for (_, sender) in guard.drain() {
        let _ = sender.send(Err(RpcError::Closed));
    }
}

/// Read loop that correlates inbound text frames with pending requests by id,
/// and answers keepalive pings through the writer channel.
async fn reader(
    mut read: futures_util::stream::SplitStream<WsStream>,
    pending: PendingMap,
    writer: mpsc::UnboundedSender<Message>,
) {
    loop {
        match read.next().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<Response>(&text) {
                Ok(response) => {
                    if let Some(sender) = pending.lock().await.remove(&response.id) {
                        let _ = sender.send(Ok(response));
                    }
                }
                Err(_) => {
                    // Unexpected or malformed frame: skip and keep the
                    // connection alive. A single bad/junk frame from a hostile
                    // peer must not tear down the whole bridge (see hostile-input
                    // posture) — only genuine EOF/error/close is fatal.
                    continue;
                }
            },
            Some(Ok(Message::Ping(data))) => {
                // A proxy/LB pings otherwise-idle connections and drops any
                // that silently go silent; tokio-tungstenite does not auto-pong,
                // so echo the payload back through the writer task.
                let _ = writer.send(Message::Pong(data));
            }
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
                // Ignore pong/binary/other frame traffic (pings already handled).
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

    #[test]
    fn empty_response_rejected_as_malformed() {
        // A response with neither `result` nor `error` is not JSON-RPC 2.0; a
        // strict client must surface it as malformed rather than a silent null.
        let response = Response {
            jsonrpc: "2.0".into(),
            id: 3,
            result: None,
            error: None,
        };
        let err = response_to_result(response).unwrap_err();
        assert!(matches!(err, RpcError::Malformed(_)));
    }

    #[test]
    fn response_with_result_returns_it() {
        let response = Response {
            jsonrpc: "2.0".into(),
            id: 4,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        };
        let value = response_to_result(response).unwrap();
        assert_eq!(value, serde_json::json!({"ok": true}));
    }

    #[test]
    fn response_with_error_maps_code() {
        let response = Response {
            jsonrpc: "2.0".into(),
            id: 5,
            result: None,
            error: Some(crate::types::RpcErrorBody {
                code: -32004,
                message: "confirm required".into(),
                data: None,
            }),
        };
        let err = response_to_result(response).unwrap_err();
        assert!(matches!(err, RpcError::Server(_)));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ping_is_answered_with_pong() {
        // A peer pings an otherwise-idle connection; the client must echo a
        // Pong back (with the same payload) so a proxy/LB that times out
        // silent connections never drops the bridge.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let payload = b"keepalive-check".to_vec();
            ws.send(Message::Ping(payload.clone().into()))
                .await
                .unwrap();

            let got_pong = tokio::time::timeout(Duration::from_secs(5), async {
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Pong(echoed) = msg {
                        assert_eq!(
                            echoed.as_ref(),
                            payload.as_slice(),
                            "pong must echo the ping's payload"
                        );
                        return true;
                    }
                }
                false
            })
            .await
            .unwrap_or(false);
            assert!(got_pong, "client must answer a ping with a pong");
        });

        let _client = RpcClient::connect(&format!("ws://{addr}"), None)
            .await
            .expect("connect should succeed");
        server.await.unwrap();
    }
}
