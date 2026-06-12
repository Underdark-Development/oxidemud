#[derive(Debug, Clone)]
pub struct WorldName(pub String);

impl Default for WorldName {
    fn default() -> Self {
        WorldName("Mud".to_string())
    }
}
