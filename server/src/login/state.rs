use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginSubstate {
    Connected,
    Username,
    Password { username: Arc<str>, attempts: u8 },
    AccountCreateConfirm { username: Arc<str> },
    AccountCreatePassword,
    AccountCreateConfirmPassword,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangePasswordSubstate {
    Old,
    New,
    Confirm { new_password: Arc<str> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterCreateSubstate {
    Name,
    Race(Vec<String>),
    Class(Vec<String>),
    Gender,
    AttributesPickMethod,
    AttributesPointBuy {
        remaining: u8,
        attrs: [u8; 6],
    },
    AttributesArray {
        values: [u8; 6],
        assign_idx: usize,
        attrs: [u8; 6],
    },
    AttributesRoll {
        rolls: [u8; 6],
        assign_idx: usize,
        attrs: [u8; 6],
        rerolls: u8,
    },
    Alignment,
    Deity(Vec<String>),
    SkillSelection {
        pool: Vec<String>,
        selected: Vec<String>,
        slots: u8,
    },
    AppearanceHeight,
    AppearanceWeight,
    AppearanceBuild(Vec<String>),
    AppearanceHairStyle,
    AppearanceHairColor(Vec<String>),
    AppearanceEyeColor(Vec<String>),
    AppearanceSkinTone(Vec<String>),
    Age,
    Description {
        lines: Vec<String>,
    },
    Spawn,
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterSelectSubstate {
    List,
    CharacterCreate(CharacterCreateSubstate),
    ChangePassword(ChangePasswordSubstate),
    CharacterDeleteConfirm { character_id: i64, name: Arc<str> },
    AccountDeleteConfirm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginState {
    Login(LoginSubstate),
    CharacterSelect(CharacterSelectSubstate),
    Playing,
}

impl LoginState {
    pub fn is_playing(&self) -> bool {
        matches!(self, LoginState::Playing)
    }

    pub fn is_pre_auth(&self) -> bool {
        matches!(self, LoginState::Login(_))
    }
}
