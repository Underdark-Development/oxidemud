//! End-to-end test: the real `oxide_mcp::ws_client::connect_ws` client
//! transport against a live WebSocket MCP server built on the shared
//! `oxide_ws_mcp` codec + rmcp's server machinery. This exercises the exact
//! same code paths as a remote `oxide-mcp` connecting to a running server's
//! `/ws/mcp`.

use oxide_ws_mcp::WsServerTransport;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio_tungstenite::accept_async;

#[derive(Deserialize, JsonSchema)]
struct EchoParams {
    text: String,
}

/// Minimal MCP server handler for the test.
#[derive(Clone)]
struct TestServer;

#[tool_router(server_handler)]
impl TestServer {
    #[tool(description = "Echo the given text back")]
    fn echo(&self, params: Parameters<EchoParams>) -> String {
        params.0.text
    }

    #[tool(description = "Return a fixed greeting")]
    fn hello(&self) -> String {
        "Hello from test server".to_string()
    }
}

async fn run_test_server(listener: tokio::net::TcpListener) -> anyhow::Result<()> {
    let (stream, _addr) = listener.accept().await?;
    let ws = accept_async(stream).await?;
    let transport = WsServerTransport::new(ws);
    let server = TestServer;
    // Keep the running service alive for the duration of the connection;
    // dropping it closes the socket.
    let _running = rmcp::ServiceExt::serve(server, transport).await?;
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    Ok(())
}

#[tokio::test]
async fn client_connects_and_lists_tools() {
    // Bind a loopback listener.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");

    // Spawn the server.
    let server_task = tokio::spawn(run_test_server(listener));

    // Connect the real client transport.
    let url = format!("ws://{addr}/mcp");
    let transport = oxide_mcp::ws_client::connect_ws(&url, None)
        .await
        .expect("connect_ws");

    let client = ().serve(transport).await.expect("client serve");
    let peer = client.peer();

    let tools = peer.list_all_tools().await.expect("list tools");
    let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    assert!(
        names.contains(&"echo".to_string()),
        "expected echo tool, got {names:?}"
    );
    assert!(
        names.contains(&"hello".to_string()),
        "expected hello tool, got {names:?}"
    );

    // Call a tool through the peer.
    let mut params = rmcp::model::CallToolRequestParams::default();
    params.name = "echo".to_string().into();
    params.arguments = Some(
        serde_json::json!({"text": "ping"})
            .as_object()
            .cloned()
            .unwrap_or_default(),
    );
    let result = peer.call_tool(params).await.expect("call tool");
    let text = result
        .content
        .into_iter()
        .filter_map(|c| match c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    assert_eq!(text, "ping");

    // Surface the server task's result so a server-side failure isn't hidden.
    server_task.abort();
    let _ = server_task.await;
}
