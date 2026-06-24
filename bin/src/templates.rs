use std::path::Path;

use oxide_core::templates::TemplateRegistry;

pub fn load_templates(content_path: &Path) -> TemplateRegistry {
    let (registry, _) = oxide_core::content::load_registry(content_path);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn project_root() -> PathBuf {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.parent().unwrap().to_path_buf()
    }

    #[test]
    fn test_actual_content_class_filtering() {
        let content_path = project_root().join("content");
        let registry = load_templates(&content_path);

        let orc_classes = registry.available_classes_for_race("orc");
        assert_eq!(orc_classes.len(), 1);
        assert_eq!(orc_classes[0].id, "warrior");

        let mage_races = registry.available_races_for_class("mage");
        assert_eq!(mage_races.len(), 1);
        assert_eq!(mage_races[0].id, "human");
    }
}
