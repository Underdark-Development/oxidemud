use super::registry::TemplateRegistry;
use std::sync::{Arc, OnceLock, RwLock};

static GLOBAL_TEMPLATES: OnceLock<RwLock<Option<Arc<TemplateRegistry>>>> = OnceLock::new();

/// Register or replace the global template registry singleton.
pub fn register_global_templates(templates: Arc<TemplateRegistry>) {
    let cell = GLOBAL_TEMPLATES.get_or_init(|| RwLock::new(None));
    let mut writer = cell.write().unwrap_or_else(|e| e.into_inner());
    *writer = Some(templates);
}

/// Retrieve a clone of the global template registry Arc, if set.
pub fn get_global_templates() -> Option<Arc<TemplateRegistry>> {
    GLOBAL_TEMPLATES.get().and_then(|cell| {
        let reader = cell.read().unwrap_or_else(|e| e.into_inner());
        reader.clone()
    })
}
