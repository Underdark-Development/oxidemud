use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginState {
    Connected,
    Username,
    Password {
        username: Arc<str>,
        attempts: u8,
    },
    AccountCreateConfirm {
        username: Arc<str>,
    },
    AccountCreatePassword,
    AccountCreateConfirmPassword,
    CharacterSelect,
    CharacterCreateName,
    CharacterCreateRace(Vec<String>),
    CharacterCreateClass(Vec<String>),
    CharacterCreateAttributesPickMethod,
    CharacterCreateAttributesPointBuy {
        remaining: u8,
        attrs: [u8; 6],
    },
    CharacterCreateAttributesArray {
        values: [u8; 6],
        assign_idx: usize,
        attrs: [u8; 6],
    },
    CharacterCreateAttributesRoll {
        rolls: [u8; 6],
        assign_idx: usize,
        attrs: [u8; 6],
        rerolls: u8,
    },
    CharacterCreateAlignment,
    CharacterCreateSkillSelection {
        pool: Vec<String>,
        selected: Vec<String>,
        slots: u8,
    },
    CharacterCreateDescription {
        lines: Vec<String>,
    },
    CharacterCreateSpawn,
    CharacterCreateConfirm,
    Playing,
}

impl LoginState {
    pub fn is_playing(&self) -> bool {
        matches!(self, LoginState::Playing)
    }

    pub fn is_pre_auth(&self) -> bool {
        matches!(
            self,
            LoginState::Connected
                | LoginState::Username
                | LoginState::Password { .. }
                | LoginState::AccountCreateConfirm { .. }
                | LoginState::AccountCreatePassword
                | LoginState::AccountCreateConfirmPassword
        )
    }
}
