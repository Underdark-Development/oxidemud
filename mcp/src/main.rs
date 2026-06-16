use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let content_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("content"));

    eprintln!(
        "MCP server starting, content path: {}",
        content_path.display()
    );

    let server = mud_mcp::MudMcpServer::new(content_path);
    server.run().await
}
