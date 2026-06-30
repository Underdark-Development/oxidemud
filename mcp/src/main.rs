use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (content_path, url, key) = resolve_connect_config(&args);

    eprintln!(
        "MCP server starting, content path: {}, url: {:?}, key: {}",
        content_path.display(),
        url,
        if key.is_some() { "[PRESENT]" } else { "[NONE]" }
    );

    let server = oxide_mcp::OxideMcpServer::new(content_path, url, key);
    server.run().await
}

fn resolve_connect_config(args: &[String]) -> (PathBuf, Option<String>, Option<String>) {
    let mut content_path = None;
    let mut url = None;
    let mut key = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" => {
                if i + 1 < args.len() {
                    url = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--key" => {
                if i + 1 < args.len() {
                    key = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            other => {
                content_path = Some(PathBuf::from(other));
                i += 1;
            }
        }
    }

    let resolved_path = content_path.unwrap_or_else(|| PathBuf::from("content"));

    // CLI Flags > Environment Variables > Config File (mcp_config.toml)
    let resolved_url = url
        .or_else(|| std::env::var("OXIDE_API_URL").ok())
        .or_else(|| {
            read_mcp_config_toml("mcp_config.toml")
                .or_else(|| read_mcp_config_toml("content/mcp_config.toml"))
                .map(|c| c.url)
        });

    let resolved_key = key
        .or_else(|| std::env::var("OXIDE_API_KEY").ok())
        .or_else(|| {
            read_mcp_config_toml("mcp_config.toml")
                .or_else(|| read_mcp_config_toml("content/mcp_config.toml"))
                .map(|c| c.key)
        });

    (resolved_path, resolved_url, resolved_key)
}

#[derive(serde::Deserialize)]
struct McpConfigToml {
    url: String,
    key: String,
}

fn read_mcp_config_toml(path: &str) -> Option<McpConfigToml> {
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}
