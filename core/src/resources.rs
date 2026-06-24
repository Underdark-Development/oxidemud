mod energy;
mod mana;
mod psi;
mod stamina;

pub use energy::*;
pub use mana::*;
pub use psi::*;
pub use stamina::*;

#[derive(Debug, Clone)]
pub struct WorldName(pub String);

impl Default for WorldName {
    fn default() -> Self {
        WorldName("Oxide MUD".to_string())
    }
}
