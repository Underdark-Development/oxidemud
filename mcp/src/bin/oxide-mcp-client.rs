//! Standalone WebSocket MCP *client* driver.
//!
//! Connects to a running OxideMUD server's `/ws/mcp` endpoint, runs rmcp's
//! client machinery, and exercises `tools/list` + a `ping`. Useful for
//! verifying the WebSocket MCP transport and for scripting remote checks.

use rmcp::ServiceExt;

fn print_usage() {
    eprintln!("Usage: oxide-mcp-client --ws <ws://host:port/ws/mcp> [--key <API_KEY>]");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut url = None;
    let mut key = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--ws" | "--url" => {
                if i + 1 < args.len() {
                    url = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    print_usage();
                    std::process::exit(1);
                }
            }
            "--key" => {
                if i + 1 < args.len() {
                    key = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    print_usage();
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                print_usage();
                std::process::exit(1);
            }
        }
    }

    let url = url.unwrap_or_else(|| "ws://127.0.0.1:8080/ws/mcp".to_string());

    eprintln!(
        "Connecting to {url} (key: {})",
        if key.is_some() { "[PRESENT]" } else { "[NONE]" }
    );

    let transport = oxide_mcp::ws_client::connect_ws(&url, key.as_deref()).await?;
    let client = ().serve(transport).await?;
    let peer = client.peer();

    eprintln!("Connected. Listing tools...");
    let tools = peer.list_all_tools().await?;
    println!("{} tool(s):", tools.len());
    for tool in &tools {
        println!("  - {}", tool.name);
    }

    Ok(())
}
