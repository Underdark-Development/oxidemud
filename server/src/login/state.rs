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
    CharacterCreateGender,
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
    CharacterCreateDeity(Vec<String>),
    CharacterCreateSkillSelection {
        pool: Vec<String>,
        selected: Vec<String>,
        slots: u8,
    },
    CharacterCreateAppearanceHeight,
    CharacterCreateAppearanceWeight,
    CharacterCreateAppearanceBuild(Vec<String>),
    CharacterCreateAppearanceHairStyle,
    CharacterCreateAppearanceHairColor(Vec<String>),
    CharacterCreateAppearanceEyeColor(Vec<String>),
    CharacterCreateAppearanceSkinTone(Vec<String>),
    CharacterCreateAge,
    CharacterCreateDescription {
        lines: Vec<String>,
    },
    CharacterCreateSpawn,
    CharacterCreateConfirm,
    ChangePasswordOld,
    ChangePasswordNew,
    ChangePasswordConfirm {
        new_password: Arc<str>,
    },
    CharacterDeleteConfirm {
        character_id: i64,
        name: Arc<str>,
    },
    AccountDeleteConfirm,
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
