use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginState {
    Connected,
    Username,
    Password { username: Arc<str>, attempts: u8 },
    AccountCreateConfirm { username: Arc<str> },
    AccountCreatePassword,
    AccountCreateConfirmPassword,
    CharacterSelect,
    CharacterCreateName,
    CharacterCreateRace,
    CharacterCreateClass,
    CharacterCreateSpawn,
    CharacterCreateConfirm,
    Playing,
}

impl LoginState {
    pub fn is_playing(&self) -> bool {
        matches!(self, LoginState::Playing)
    }
}
