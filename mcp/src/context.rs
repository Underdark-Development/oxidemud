use std::collections::HashMap;
use std::path::Path;

use crate::content;
use crate::params::LoadedPlayer;

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
    pub(crate) fn api_url(&self) -> &Option<String> {
        self.api_url
    }
    pub(crate) fn api_key(&self) -> &Option<String> {
        self.api_key
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
    pub(crate) async fn fetch_player_state(&self, name: &str) -> Result<LoadedPlayer, String> {
        let (_url, _key) = match (self.api_url, self.api_key) { (Some(u), Some(k)) => (u,k), _ => return Err("Offline mode: cannot fetch real player data. Provide --url and --key to connect to the MUD server.".to_string()) };
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
        let (url,key)=match (self.api_url,self.api_key) { (Some(u),Some(k))=>(u,k), _=>return Err("Offline mode: cannot fetch real player data. Provide --url and --key to connect to the MUD server.".to_string()) };
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
        let (url,key)=match (self.api_url,self.api_key) { (Some(u),Some(k))=>(u,k), _=>return Err("Offline mode: cannot fetch real player data. Provide --url and --key to connect to the MUD server.".to_string()) };
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
