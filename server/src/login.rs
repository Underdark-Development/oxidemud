mod handlers;
mod prompt;
mod state;

pub use state::LoginState;

pub use prompt::list_who;

use tokio::sync::Mutex;

use mud_core::templates::TemplateRegistry;
use mud_core::{Entity, World};

use crate::registry::ConnectionRegistry;

/// Temporary buffer for character creation wizard data.
#[derive(Debug, Clone, Default)]
pub struct CharacterCreateBuffer {
    pub name: Option<String>,
    pub race: Option<String>,
    pub class: Option<String>,
    pub password: Option<String>,
    pub spawn_key: Option<String>,
    pub attributes: Option<mud_core::Attributes>,
    pub alignment: Option<String>,
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
            state: LoginState::Connected,
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
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_input(
        &mut self,
        input: &str,
        db: Option<&Mutex<mud_data::Database>>,
        templates: Option<&TemplateRegistry>,
        world: &mut World,
        registry: &mut ConnectionRegistry,
        void_room: Entity,
        spawn_room: Entity,
    ) -> Vec<String> {
        let mut lines = match &self.state {
            LoginState::Connected => handlers::handle_connected_state(self),
            LoginState::Username => handlers::handle_username_state(self, input, db).await,
            LoginState::Password { username, attempts } => {
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
            LoginState::AccountCreateConfirm { .. } => {
                handlers::handle_account_create_confirm_state(self, input)
            }
            LoginState::AccountCreatePassword => {
                handlers::handle_account_create_password_state(self, input)
            }
            LoginState::AccountCreateConfirmPassword => {
                handlers::handle_account_create_confirm_password_state(self, input, db).await
            }
            LoginState::CharacterSelect => {
                handlers::handle_character_select_state(
                    self, input, db, world, registry, void_room, spawn_room,
                )
                .await
            }
            LoginState::CharacterCreateName => {
                handlers::handle_character_create_name_state(self, input, db).await
            }
            LoginState::CharacterCreateRace(..) => {
                handlers::handle_character_create_race_state(self, input, templates)
            }
            LoginState::CharacterCreateClass(..) => {
                handlers::handle_character_create_class_state(self, input, templates)
            }
            LoginState::CharacterCreateAttributesPickMethod => {
                handlers::handle_attributes_pick_method_state(self, input)
            }
            LoginState::CharacterCreateAttributesPointBuy { .. } => {
                handlers::handle_point_buy_state(self, input)
            }
            LoginState::CharacterCreateAttributesArray { .. } => {
                handlers::handle_standard_array_state(self, input)
            }
            LoginState::CharacterCreateAttributesRoll { .. } => {
                handlers::handle_roll_state(self, input)
            }
            LoginState::CharacterCreateAlignment => {
                handlers::handle_alignment_state(self, input, templates)
            }
            LoginState::CharacterCreateSkillSelection { .. } => {
                handlers::handle_skill_selection_state(self, input, templates)
            }
            LoginState::CharacterCreateDescription { .. } => {
                handlers::handle_description_state(self, input)
            }
            LoginState::CharacterCreateSpawn => {
                handlers::handle_spawn_select_state(self, input, templates)
            }
            LoginState::CharacterCreateConfirm => {
                handlers::handle_character_create_confirm_state(
                    self, input, db, world, void_room, spawn_room, templates,
                )
                .await
            }
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
        db: Option<&Mutex<mud_data::Database>>,
        templates: Option<&TemplateRegistry>,
    ) -> Vec<String> {
        match &self.state {
            LoginState::Connected => {
                self.state = LoginState::Username;
                Vec::new()
            }
            LoginState::Username
            | LoginState::Password { .. }
            | LoginState::AccountCreateConfirm { .. }
            | LoginState::AccountCreatePassword
            | LoginState::AccountCreateConfirmPassword => {
                // Prompt already sent by handler during transition; no re-display needed.
                Vec::new()
            }
            LoginState::CharacterSelect => {
                prompt::go_to_character_select(self, db, templates).await
            }
            LoginState::CharacterCreateName => {
                vec![
                    String::new(),
                    "Enter your character's name (3-16 letters, hyphens, apostrophes):".to_string(),
                ]
            }
            LoginState::CharacterCreateRace(..) => {
                prompt::show_character_race_prompt(self, templates)
            }
            LoginState::CharacterCreateClass(..) => {
                if let Some(t) = templates {
                    prompt::show_character_class_prompt(self, t)
                } else {
                    Vec::new()
                }
            }
            LoginState::CharacterCreateAttributesPickMethod => {
                prompt::show_attribute_method_prompt()
            }
            LoginState::CharacterCreateAttributesPointBuy { .. } => {
                prompt::show_point_buy_prompt(self)
            }
            LoginState::CharacterCreateAttributesArray { .. } => {
                prompt::show_standard_array_prompt(self)
            }
            LoginState::CharacterCreateAttributesRoll { .. } => prompt::show_roll_prompt(self),
            LoginState::CharacterCreateAlignment => prompt::show_alignment_prompt(self, templates),
            LoginState::CharacterCreateSkillSelection { .. } => {
                prompt::show_skill_selection_prompt(self, templates)
            }
            LoginState::CharacterCreateDescription { .. } => {
                vec![
                    String::new(),
                    "Enter your character's description (multi-line). Type '.' on a blank line to finish:".to_string(),
                ]
            }
            LoginState::CharacterCreateSpawn => {
                if let Some(t) = templates {
                    prompt::show_spawn_prompt(self, t)
                } else {
                    Vec::new()
                }
            }
            LoginState::CharacterCreateConfirm => {
                if let Some(t) = templates {
                    prompt::show_character_confirm(self, t)
                } else {
                    Vec::new()
                }
            }
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
