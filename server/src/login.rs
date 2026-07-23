mod handlers;
mod prompt;
mod state;

pub(crate) use handlers::{class_starting_gold, compute_final_attributes};
pub use state::{
    ChangePasswordSubstate, CharacterCreateSubstate, CharacterSelectSubstate, LoginState,
    LoginSubstate,
};

pub use prompt::list_who;

use tokio::sync::Mutex;

use oxide_core::templates::TemplateRegistry;
use oxide_core::{Entity, World};

use crate::registry::ConnectionRegistry;

/// Temporary buffer for character creation wizard data.
#[derive(Debug, Clone, Default)]
pub struct CharacterCreateBuffer {
    pub name: Option<String>,
    pub race: Option<String>,
    pub class: Option<String>,
    pub gender: Option<String>,
    pub pronoun_subject: Option<String>,
    pub pronoun_object: Option<String>,
    pub pronoun_possessive: Option<String>,
    pub password: Option<String>,
    pub spawn_key: Option<String>,
    pub attributes: Option<oxide_core::Attributes>,
    pub alignment: Option<String>,
    pub deity: Option<String>,
    pub appearance_height: Option<u8>,
    pub appearance_weight: Option<u16>,
    pub appearance_build: Option<String>,
    pub appearance_hair_style: Option<String>,
    pub appearance_hair_color: Option<String>,
    pub appearance_eye_color: Option<String>,
    pub appearance_skin_tone: Option<String>,
    pub age: Option<u16>,
    pub description: Option<String>,
    pub selected_skills: Vec<String>,
}

/// Owns all login / character-creation state. Kept independent of the
/// [`Connection`](crate::Connection) trait so it can be unit-tested without
/// transport infrastructure.
///
/// # Output model
///
/// Handlers return `Vec<String>` for display. Side‑effects (echo, disconnect,
/// entity registration) are tracked as flags that the caller MUST apply:
///
/// 1. Call [`handle_input`](LoginFlow::handle_input) → get messages.
/// 2. Read `echo_enabled()` → apply `set_echo()` on the connection.
/// 3. Send messages via `conn.send_line()`.
/// 4. Read `entity_just_spawned()` → register entity with registry.
/// 5. Read `disconnect_requested()` → call `conn.disconnect()`.
pub struct LoginFlow {
    pub(crate) state: LoginState,
    create_buffer: CharacterCreateBuffer,
    /// Number of failed login attempts.
    pub strikes: u8,
    /// The authenticated account ID (set after password verification).
    pub account_id: Option<i64>,
    /// Whether the user has dismissed the "no characters" prompt.
    pub create_dismissed: bool,

    /// The player entity after a successful login or character creation.
    entity: Option<Entity>,
    /// Set to `true` when `entity` is freshly spawned (one-shot flag).
    entity_just_spawned: bool,

    // --- Output flags (set by handlers, consumed by caller) ---
    /// Whether server-side echo should be enabled.
    echo_on: bool,
    /// Whether the caller should disconnect the connection.
    pub disconnect_requested: bool,
}

impl Default for LoginFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginFlow {
    pub fn new() -> Self {
        LoginFlow {
            state: LoginState::Login(LoginSubstate::Connected),
            create_buffer: CharacterCreateBuffer::default(),
            strikes: 0,
            account_id: None,
            create_dismissed: false,
            entity: None,
            entity_just_spawned: false,
            echo_on: false,
            disconnect_requested: false,
        }
    }

    // -- accessors -----------------------------------------------------------

    pub fn state(&self) -> &LoginState {
        &self.state
    }

    /// Returns the current echo flag and resets it to `false`.
    /// Callers should apply this to the connection before sending prompt lines.
    pub fn take_echo(&mut self) -> bool {
        std::mem::take(&mut self.echo_on)
    }

    /// Returns the disconnect flag and resets it to `false`.
    pub fn take_disconnect(&mut self) -> bool {
        std::mem::take(&mut self.disconnect_requested)
    }

    pub fn entity(&self) -> Option<Entity> {
        self.entity
    }

    /// Returns `true` if a new entity was spawned during the last
    /// [`handle_input`](LoginFlow::handle_input) call, and resets the flag.
    /// The caller should read [`entity()`](LoginFlow::entity),
    /// set it on the connection, and register with the registry.
    pub fn take_entity_just_spawned(&mut self) -> bool {
        std::mem::take(&mut self.entity_just_spawned)
    }

    // -- dispatch ------------------------------------------------------------

    /// Dispatch the user's input to the handler corresponding to the current
    /// [`LoginState`]. Returns lines to display.
    ///
    /// After calling this, the caller MUST check the output flags
    /// ([`take_echo`](LoginFlow::take_echo),
    /// [`take_disconnect`](LoginFlow::take_disconnect)) and apply them to
    /// the connection.
    pub async fn handle_input(
        &mut self,
        input: &str,
        db: Option<&Mutex<oxide_data::Database>>,
        templates: Option<&TemplateRegistry>,
        world: &mut World,
        registry: &mut ConnectionRegistry,
    ) -> Vec<String> {
        let mut lines = match &self.state {
            LoginState::Login(sub) => match sub {
                LoginSubstate::Connected => handlers::handle_connected_state(self),
                LoginSubstate::Username => handlers::handle_username_state(self, input, db).await,
                LoginSubstate::Password { username, attempts } => {
                    handlers::handle_password_state(
                        self,
                        input,
                        db,
                        username.clone(),
                        *attempts,
                        templates,
                    )
                    .await
                }
                LoginSubstate::AccountCreateConfirm { .. } => {
                    handlers::handle_account_create_confirm_state(self, input)
                }
                LoginSubstate::AccountCreatePassword => {
                    handlers::handle_account_create_password_state(self, input)
                }
                LoginSubstate::AccountCreateConfirmPassword => {
                    handlers::handle_account_create_confirm_password_state(self, input, db).await
                }
            },
            LoginState::CharacterSelect(sub) => match sub {
                CharacterSelectSubstate::List => {
                    handlers::handle_character_select_state(self, input, db, world, registry).await
                }
                CharacterSelectSubstate::ChangePassword(cp_sub) => match cp_sub {
                    ChangePasswordSubstate::Old => {
                        handlers::handle_change_password_old_state(self, input, db).await
                    }
                    ChangePasswordSubstate::New => {
                        handlers::handle_change_password_new_state(self, input)
                    }
                    ChangePasswordSubstate::Confirm { new_password } => {
                        let new_pw = new_password.clone();
                        handlers::handle_change_password_confirm_state(self, input, new_pw, db)
                            .await
                    }
                },
                CharacterSelectSubstate::CharacterDeleteConfirm { character_id, name } => {
                    let cid = *character_id;
                    let cname = name.clone();
                    handlers::handle_character_delete_confirm_state(self, input, cid, cname, db)
                        .await
                }
                CharacterSelectSubstate::AccountDeleteConfirm => {
                    handlers::handle_account_delete_confirm_state(self, input, db).await
                }
                CharacterSelectSubstate::CharacterCreate(cc_sub) => match cc_sub {
                    CharacterCreateSubstate::Name => {
                        handlers::handle_character_create_name_state(self, input, db).await
                    }
                    CharacterCreateSubstate::Race(..) => {
                        handlers::handle_character_create_race_state(self, input, templates)
                    }
                    CharacterCreateSubstate::Class(..) => {
                        handlers::handle_character_create_class_state(self, input, templates)
                    }
                    CharacterCreateSubstate::Gender => {
                        handlers::handle_character_create_gender_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AttributesPickMethod => {
                        handlers::handle_attributes_pick_method_state(self, input)
                    }
                    CharacterCreateSubstate::AttributesPointBuy { .. } => {
                        handlers::handle_point_buy_state(self, input)
                    }
                    CharacterCreateSubstate::AttributesArray { .. } => {
                        handlers::handle_standard_array_state(self, input)
                    }
                    CharacterCreateSubstate::AttributesRoll { .. } => {
                        handlers::handle_roll_state(self, input)
                    }
                    CharacterCreateSubstate::Alignment => {
                        handlers::handle_alignment_state(self, input, templates)
                    }
                    CharacterCreateSubstate::Deity(..) => {
                        handlers::handle_character_create_deity_state(self, input, templates)
                    }
                    CharacterCreateSubstate::SkillSelection { .. } => {
                        handlers::handle_skill_selection_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AppearanceHeight => {
                        handlers::handle_appearance_height_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AppearanceWeight => {
                        handlers::handle_appearance_weight_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AppearanceBuild(..) => {
                        handlers::handle_appearance_build_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AppearanceHairStyle => {
                        handlers::handle_appearance_hair_style_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AppearanceHairColor(..) => {
                        handlers::handle_appearance_hair_color_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AppearanceEyeColor(..) => {
                        handlers::handle_appearance_eye_color_state(self, input, templates)
                    }
                    CharacterCreateSubstate::AppearanceSkinTone(..) => {
                        handlers::handle_appearance_skin_tone_state(self, input, templates)
                    }
                    CharacterCreateSubstate::Age => {
                        handlers::handle_age_state(self, input, templates)
                    }
                    CharacterCreateSubstate::Description { .. } => {
                        handlers::handle_description_state(self, input)
                    }
                    CharacterCreateSubstate::Spawn => {
                        handlers::handle_spawn_select_state(self, input, templates)
                    }
                    CharacterCreateSubstate::Confirm => {
                        handlers::handle_character_create_confirm_state(
                            self, input, db, world, templates,
                        )
                        .await
                    }
                },
            },
            LoginState::Playing => Vec::new(),
        };

        if lines.first().is_none_or(|l| !l.is_empty()) {
            lines.insert(0, String::new());
        }
        lines
    }

    /// Return the lines that form the prompt for the current state.
    /// Callers should send these after handling input and before the next read.
    pub async fn show_state_prompt(
        &mut self,
        db: Option<&Mutex<oxide_data::Database>>,
        templates: Option<&TemplateRegistry>,
        screen_width: u16,
    ) -> Vec<String> {
        match &self.state {
            LoginState::Login(sub) => match sub {
                LoginSubstate::Connected => {
                    self.state = LoginState::Login(LoginSubstate::Username);
                    vec!["Enter your username:".to_string()]
                }
                LoginSubstate::Username => {
                    vec!["Enter your username:".to_string()]
                }
                LoginSubstate::Password { .. } => {
                    vec!["Password:".to_string()]
                }
                LoginSubstate::AccountCreateConfirm { username } => {
                    vec![format!(
                        "No account found for '{username}'. Create a new account? (y/n)"
                    )]
                }
                LoginSubstate::AccountCreatePassword => {
                    vec!["Enter a password (8+ characters):".to_string()]
                }
                LoginSubstate::AccountCreateConfirmPassword => {
                    vec!["Confirm password:".to_string()]
                }
            },
            LoginState::CharacterSelect(sub) => match sub {
                CharacterSelectSubstate::List => {
                    prompt::go_to_character_select(self, db, templates, screen_width).await
                }
                CharacterSelectSubstate::ChangePassword(cp_sub) => match cp_sub {
                    ChangePasswordSubstate::Old => {
                        vec![String::new(), "Enter current password:".to_string()]
                    }
                    ChangePasswordSubstate::New => {
                        vec![
                            String::new(),
                            "Enter new password (8+ characters):".to_string(),
                        ]
                    }
                    ChangePasswordSubstate::Confirm { .. } => {
                        vec![String::new(), "Confirm new password:".to_string()]
                    }
                },
                CharacterSelectSubstate::CharacterDeleteConfirm { name, .. } => {
                    vec![
                        String::new(),
                        format!(
                            "Are you sure you want to permanently delete character '{}'? (y/n)",
                            name
                        ),
                    ]
                }
                CharacterSelectSubstate::AccountDeleteConfirm => {
                    vec![
                        String::new(),
                        "Are you sure you want to delete your account?".to_string(),
                        "This will permanently delete your account and all characters.".to_string(),
                        "Enter your password to confirm account deletion:".to_string(),
                    ]
                }
                CharacterSelectSubstate::CharacterCreate(cc_sub) => match cc_sub {
                    CharacterCreateSubstate::Name => {
                        vec![
                            String::new(),
                            "Enter your character's name (3-16 letters, hyphens, apostrophes):"
                                .to_string(),
                        ]
                    }
                    CharacterCreateSubstate::Race(..) => {
                        prompt::show_character_race_prompt(self, templates)
                    }
                    CharacterCreateSubstate::Class(..) => {
                        if let Some(t) = templates {
                            prompt::show_character_class_prompt(self, t)
                        } else {
                            Vec::new()
                        }
                    }
                    CharacterCreateSubstate::Gender => {
                        if let Some(t) = templates {
                            prompt::show_character_gender_prompt(self, t)
                        } else {
                            Vec::new()
                        }
                    }
                    CharacterCreateSubstate::AttributesPickMethod => {
                        prompt::show_attribute_method_prompt()
                    }
                    CharacterCreateSubstate::AttributesPointBuy { .. } => {
                        prompt::show_point_buy_prompt(self)
                    }
                    CharacterCreateSubstate::AttributesArray { .. } => {
                        prompt::show_standard_array_prompt(self)
                    }
                    CharacterCreateSubstate::AttributesRoll { .. } => {
                        prompt::show_roll_prompt(self)
                    }
                    CharacterCreateSubstate::Alignment => {
                        prompt::show_alignment_prompt(self, templates)
                    }
                    CharacterCreateSubstate::Deity(options) => {
                        prompt::show_character_deity_prompt(self, templates, options)
                    }
                    CharacterCreateSubstate::SkillSelection { .. } => {
                        prompt::show_skill_selection_prompt(self, templates)
                    }
                    CharacterCreateSubstate::AppearanceHeight => {
                        prompt::show_appearance_height_prompt(self, templates)
                    }
                    CharacterCreateSubstate::AppearanceWeight => {
                        prompt::show_appearance_weight_prompt(self, templates)
                    }
                    CharacterCreateSubstate::AppearanceBuild(options) => {
                        prompt::show_appearance_build_prompt(self, options)
                    }
                    CharacterCreateSubstate::AppearanceHairStyle => {
                        prompt::show_appearance_hair_style_prompt()
                    }
                    CharacterCreateSubstate::AppearanceHairColor(options) => {
                        prompt::show_appearance_hair_color_prompt(self, options)
                    }
                    CharacterCreateSubstate::AppearanceEyeColor(options) => {
                        prompt::show_appearance_eye_color_prompt(self, options)
                    }
                    CharacterCreateSubstate::AppearanceSkinTone(options) => {
                        prompt::show_appearance_skin_tone_prompt(self, options)
                    }
                    CharacterCreateSubstate::Age => prompt::show_age_prompt(self, templates),
                    CharacterCreateSubstate::Description { .. } => {
                        vec![
                            String::new(),
                            "Enter your character's description (multi-line). Type '.' on a blank line to finish:".to_string(),
                        ]
                    }
                    CharacterCreateSubstate::Spawn => {
                        if let Some(t) = templates {
                            prompt::show_spawn_prompt(self, t)
                        } else {
                            Vec::new()
                        }
                    }
                    CharacterCreateSubstate::Confirm => {
                        if let Some(t) = templates {
                            prompt::show_character_confirm(self, t)
                        } else {
                            Vec::new()
                        }
                    }
                },
            },
            LoginState::Playing => {
                // Don't show a prompt here; the game command loop handles its own prompt.
                Vec::new()
            }
        }
    }

    #[cfg(test)]
    pub fn in_state(&self, state: LoginState) -> bool {
        self.state == state
    }
}
