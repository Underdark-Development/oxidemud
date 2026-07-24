pub mod energy;
pub mod mana;
pub mod psi;
pub mod stamina;

pub use energy::Energy;
pub use mana::Mana;
pub use psi::Psi;
pub use stamina::Stamina;

#[derive(Debug, Clone)]
pub struct WorldName(pub String);

impl Default for WorldName {
    fn default() -> Self {
        WorldName("Oxide MUD".to_string())
    }
}
