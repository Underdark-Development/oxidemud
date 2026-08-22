use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use oxide_ws_rpc::{RpcClient, RpcError};

use crate::content;
use crate::params::LoadedPlayer;

const OFFLINE_ERR: &str =
    "Offline mode: cannot fetch real player data. Provide --url and --key to connect to the MUD server.";

/// Render a sorted `label:` block of id → name entries (used by list tools).
pub(crate) fn entity_list(items: &HashMap<String, impl AsRef<str>>, label: &str) -> String {
    if items.is_empty() {
        return format!("No {} found.", label);
    }
    let mut ids: Vec<&String> = items.keys().collect();
    ids.sort();
    let mut out = format!("{}:\n", label);
    for id in ids {
        out.push_str(&format!("  {id}: {}\n", items[id].as_ref()));
    }
    out.trim().to_string()
}

#[derive(Clone, Copy)]
pub(crate) struct HandlerContext<'a> {
    content_path: &'a Path,
    api_url: &'a Option<String>,
    api_key: &'a Option<String>,
}

impl<'a> HandlerContext<'a> {
    pub(crate) fn new(
        content_path: &'a Path,
        api_url: &'a Option<String>,
        api_key: &'a Option<String>,
    ) -> Self {
        Self {
            content_path,
            api_url,
            api_key,
        }
    }
    pub(crate) fn content_path(&self) -> &Path {
        self.content_path
    }
    pub(crate) fn load(&self) -> (oxide_core::templates::TemplateRegistry, content::FileMap) {
        content::load_registry(self.content_path)
    }
    pub(crate) fn validate_id(&self, id: &str) -> Result<(), String> {
        content::validate_content_id(id)
    }
    pub(crate) fn validate_and_contain(&self, id: &str, path: &Path) -> Result<(), String> {
        content::validate_content_id(id)?;
        content::assert_within_content_dir(self.content_path, path)
    }

    /// Resolve the configured API credentials, or return an offline-mode error.
    fn creds(&self) -> Result<(&str, &str), String> {
        match (self.api_url.as_deref(), self.api_key.as_deref()) {
            (Some(u), Some(k)) => Ok((u, k)),
            _ => Err(OFFLINE_ERR.to_string()),
        }
    }

    /// Whether an online WS connection is configured (both URL and API key).
    pub(crate) fn has_creds(&self) -> bool {
        self.api_url.is_some() && self.api_key.is_some()
    }

    /// Open a fresh WS RPC client for the configured endpoint, or return the
    /// offline-mode / connection error as a String.
    pub(crate) async fn rpc(&self) -> Result<Arc<RpcClient>, String> {
        let (url, key) = self.creds()?;
        RpcClient::connect(url, Some(key))
            .await
            .map(Arc::new)
            .map_err(|e| format!("Failed to connect to MUD server: {e}"))
    }

    pub(crate) async fn fetch_player_state(&self, name: &str) -> Result<LoadedPlayer, String> {
        let client = self.rpc().await?;
        match client
            .call_typed::<LoadedPlayer>("player.state", serde_json::json!({ "name": name }))
            .await
        {
            Ok(player) => Ok(player),
            Err(e) => Err(rpc_typed_error(e)),
        }
    }

    /// Run an `imm.*` method and return its human `message` on success. On
    /// failure returns the offline/connection error or the server's raw error
    /// message, matching the old REST error body.
    pub(crate) async fn call_imm(&self, method: &str, params: serde_json::Value) -> String {
        self.run_imm(method, params, false).await
    }

    /// Like [`call_imm`](Self::call_imm), but server errors are prefixed with
    /// `Error from server: ` to match the former REST handlers that did so.
    pub(crate) async fn call_imm_prefixed(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> String {
        self.run_imm(method, params, true).await
    }

    async fn run_imm(&self, method: &str, params: serde_json::Value, prefix: bool) -> String {
        let client = match self.rpc().await {
            Ok(c) => c,
            Err(e) => return e,
        };
        match client.call(method, params).await {
            Ok(value) => imm_message(&value).to_string(),
            Err(e) => {
                let msg = rpc_error_message(e);
                if prefix {
                    format!("Error from server: {msg}")
                } else {
                    msg
                }
            }
        }
    }

    /// Read-only REST call kept for the `simulate_character_creation` online
    /// branch (`/api/character/simulate`), which has no WS equivalent.
    pub(crate) async fn authenticated_request_with_body(
        &self,
        method: reqwest::Method,
        path: String,
        body: Option<&serde_json::Value>,
    ) -> Result<reqwest::Response, String> {
        let (url, key) = self.creds()?;
        let mut req = reqwest::Client::new()
            .request(method, format!("{}{}", url.trim_end_matches('/'), path))
            .header("Authorization", format!("Bearer {key}"));
        if let Some(b) = body {
            req = req.json(b);
        }
        req.send()
            .await
            .map_err(|e| format!("Failed to connect to MUD server: {e}"))
    }
}

/// Format an `imm.*` RPC failure into the raw server message (or a transport
/// fallback), mirroring the old REST error-body text.
pub(crate) fn rpc_error_message(e: RpcError) -> String {
    match e {
        RpcError::Server(m) | RpcError::MethodNotFound(m) => m,
        other => format!("Failed to connect to MUD server: {other}"),
    }
}

/// Format a typed-RPC (e.g. `player.state`) failure, matching the old REST
/// error wording used by `fetch_player_state`.
pub(crate) fn rpc_typed_error(e: RpcError) -> String {
    match e {
        RpcError::Malformed(m) => format!("Failed to parse MUD Server response as JSON: {m}"),
        RpcError::Server(m) | RpcError::MethodNotFound(m) => {
            format!("Error from server: {m}")
        }
        other => format!("Failed to connect to MUD server: {other}"),
    }
}

/// Extract the human-readable `message` from an `imm.*` RPC response, falling
/// back to `"Success"` when the server omits or malforms it.
fn imm_message(value: &serde_json::Value) -> &str {
    value
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Success")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_error_message_returns_raw_msg_for_server_errors() {
        assert_eq!(rpc_error_message(RpcError::Server("boom".into())), "boom");
        assert_eq!(
            rpc_error_message(RpcError::MethodNotFound("no_such_method".into())),
            "no_such_method"
        );
    }

    #[test]
    fn rpc_error_message_falls_back_on_transport_errors() {
        assert_eq!(
            rpc_error_message(RpcError::Malformed("bad frame".into())),
            "Failed to connect to MUD server: malformed message: bad frame"
        );
        assert_eq!(
            rpc_error_message(RpcError::Timeout),
            "Failed to connect to MUD server: request timed out"
        );
        assert_eq!(
            rpc_error_message(RpcError::Closed),
            "Failed to connect to MUD server: connection closed"
        );
        assert_eq!(
            rpc_error_message(RpcError::Io(std::io::Error::other("disk"))),
            "Failed to connect to MUD server: io error: disk"
        );
    }

    #[test]
    fn rpc_typed_error_maps_malformed_to_parse_failure() {
        assert_eq!(
            rpc_typed_error(RpcError::Malformed("garbage".into())),
            "Failed to parse MUD Server response as JSON: garbage"
        );
    }

    #[test]
    fn rpc_typed_error_passes_server_errors_through() {
        assert_eq!(
            rpc_typed_error(RpcError::Server("denied".into())),
            "Error from server: denied"
        );
        assert_eq!(
            rpc_typed_error(RpcError::MethodNotFound("player.state".into())),
            "Error from server: player.state"
        );
    }

    #[test]
    fn rpc_typed_error_falls_back_on_transport_errors() {
        assert_eq!(
            rpc_typed_error(RpcError::Timeout),
            "Failed to connect to MUD server: request timed out"
        );
        assert_eq!(
            rpc_typed_error(RpcError::Closed),
            "Failed to connect to MUD server: connection closed"
        );
    }

    #[test]
    fn imm_message_extracts_message_or_defaults_to_success() {
        assert_eq!(imm_message(&serde_json::json!({"message": "done"})), "done");
        assert_eq!(
            imm_message(&serde_json::json!({"success": true})),
            "Success"
        );
        // Non-object and non-string message -> default, no panic.
        assert_eq!(imm_message(&serde_json::json!(42)), "Success");
        assert_eq!(imm_message(&serde_json::json!("x")), "Success");
        assert_eq!(imm_message(&serde_json::json!({"message": 42})), "Success");
    }
}
