use std::collections::HashMap;
use std::path::Path;

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

    pub(crate) async fn fetch_player_state(&self, name: &str) -> Result<LoadedPlayer, String> {
        let resp = self
            .authenticated_request(reqwest::Method::GET, format!("/api/character/{name}"))
            .await?;
        if resp.status().is_success() {
            resp.json::<LoadedPlayer>()
                .await
                .map_err(|e| format!("Failed to parse MUD Server response as JSON: {e}"))
        } else {
            let status = resp.status();
            match resp.text().await {
                Ok(t) => Err(format!("Error from server: {t}")),
                Err(_) => Err(format!("Server returned error status: {}", status)),
            }
        }
    }
    pub(crate) async fn authenticated_request(
        &self,
        method: reqwest::Method,
        path: String,
    ) -> Result<reqwest::Response, String> {
        let (url, key) = self.creds()?;
        reqwest::Client::new()
            .request(method, format!("{}{}", url.trim_end_matches('/'), path))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .map_err(|e| format!("Failed to connect to MUD server: {e}"))
    }
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
