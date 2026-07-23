use super::registry::TemplateRegistry;
use std::sync::Arc;
use std::sync::OnceLock;

static GLOBAL_TEMPLATES: OnceLock<std::sync::RwLock<Arc<TemplateRegistry>>> = OnceLock::new();

pub fn register_global_templates(templates: Arc<TemplateRegistry>) {
    if let Some(lock) = GLOBAL_TEMPLATES.get() {
        if let Ok(mut writer) = lock.write() {
            *writer = templates;
        }
    } else {
        let _ = GLOBAL_TEMPLATES.set(std::sync::RwLock::new(templates));
    }
}

pub fn get_global_templates() -> Option<Arc<TemplateRegistry>> {
    GLOBAL_TEMPLATES
        .get()
        .and_then(|lock| lock.read().ok().map(|r| r.clone()))
}
