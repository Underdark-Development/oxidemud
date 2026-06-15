use std::sync::Arc;

use tokio::sync::Mutex;

use mud_core::templates::TemplateRegistry;
use mud_core::{
    Class, DbId, Entity, Experience, Health, Level, Name, Player, Position, Race, World,
};

use crate::registry::ConnectionRegistry;

use super::state::LoginState;
use super::LoginFlow;

// ---------------------------------------------------------------------------
// Handler helpers
// ---------------------------------------------------------------------------

/// Validates a username: 3-20 chars, alphanumeric plus hyphens and underscores.
fn is_valid_username(s: &str) -> bool {
    if s.len() < 3 || s.len() > 20 {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Validates a character name: 3-16 chars, letters, hyphens, apostrophes.
fn is_valid_character_name(s: &str) -> bool {
    if !(3..=16).contains(&s.len()) {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    let last = chars.last().unwrap_or(first);
    if !first.is_ascii_alphabetic() || !last.is_ascii_alphabetic() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'')
}

fn world_building_ready(templates: Option<&TemplateRegistry>) -> bool {
    match templates {
        Some(t) => !t.races.is_empty(),
        None => false,
    }
}

fn compute_character_stats(
    templates: Option<&TemplateRegistry>,
    race_id: &str,
    class_id: &str,
) -> (mud_core::Attributes, i32, mud_core::LearnedSkills) {
    let mut skills = mud_core::LearnedSkills::new();

    let (base_str, base_dex, base_int, base_wis, base_con, base_cha) = templates
        .and_then(|t| t.get_race(race_id))
        .map(|r| {
            for ability in &r.racial_abilities {
                skills.grant(ability);
            }
            (
                r.attributes.strength as i16,
                r.attributes.dexterity as i16,
                r.attributes.intelligence as i16,
                r.attributes.wisdom as i16,
                r.attributes.constitution as i16,
                r.attributes.charisma as i16,
            )
        })
        .unwrap_or((10, 10, 10, 10, 10, 10));

    let (mod_str, mod_dex, mod_int, mod_wis, mod_con, mod_cha, hit_die) = templates
        .and_then(|t| t.get_class(class_id))
        .map(|c| {
            for skill_id in &c.auto_skills {
                skills.grant(skill_id);
            }
            (
                c.attribute_mods.strength,
                c.attribute_mods.dexterity,
                c.attribute_mods.intelligence,
                c.attribute_mods.wisdom,
                c.attribute_mods.constitution,
                c.attribute_mods.charisma,
                c.hit_die,
            )
        })
        .unwrap_or((0, 0, 0, 0, 0, 0, 8));

    let attrs = mud_core::Attributes::new(
        (base_str + mod_str as i16).clamp(3, 50) as u8,
        (base_dex + mod_dex as i16).clamp(3, 50) as u8,
        (base_int + mod_int as i16).clamp(3, 50) as u8,
        (base_wis + mod_wis as i16).clamp(3, 50) as u8,
        (base_con + mod_con as i16).clamp(3, 50) as u8,
        (base_cha + mod_cha as i16).clamp(3, 50) as u8,
    );

    let hp = hit_die as i32 + (attrs.constitution as i32 - 10) / 2;

    (attrs, hp.max(1), skills)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub fn handle_connected_state(flow: &mut LoginFlow) -> Vec<String> {
    flow.state = LoginState::Username;
    Vec::new()
}

pub async fn handle_username_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<mud_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let username = input.trim();
    if !is_valid_username(username) {
        lines.push(String::new());
        lines.push(
            "Invalid username. Use 3-20 letters, numbers, hyphens, or underscores.".to_string(),
        );
        return lines;
    }

    let db = match db {
        Some(d) => d,
        None => return lines,
    };

    let db_guard = db.lock().await;
    let existing = match mud_data::get_account_by_username(db_guard.conn(), username) {
        Ok(e) => e,
        Err(e) => {
            lines.push(format!("DB error: {e}"));
            return lines;
        }
    };
    drop(db_guard);

    if existing.is_some() {
        flow.echo_on = true;
        lines.push(String::new());
        lines.push("Password:".to_string());
        flow.state = LoginState::Password {
            username: Arc::from(username.to_string()),
            attempts: 0,
        };
    } else {
        lines.push(String::new());
        lines.push(format!(
            "No account found for '{username}'. Create a new account? (y/n)"
        ));
        flow.state = LoginState::AccountCreateConfirm {
            username: Arc::from(username.to_string()),
        };
    }
    lines
}

pub async fn handle_password_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<mud_data::Database>>,
    username: Arc<str>,
    attempts: u8,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();

    if input.is_empty() {
        lines.push(String::new());
        lines.push("Password cannot be empty.".to_string());
        flow.strikes += 1;
        if flow.strikes >= 3 {
            lines.push("Too many failed attempts. Disconnecting.".to_string());
            flow.disconnect_requested = true;
        }
        return lines;
    }

    let db = match db {
        Some(d) => d,
        None => return lines,
    };

    let (account_id, password_hash) = {
        let db_guard = db.lock().await;
        let account = match mud_data::get_account_by_username(db_guard.conn(), &username) {
            Ok(Some(a)) => a,
            Ok(None) => {
                lines.push("Account vanished.".to_string());
                return lines;
            }
            Err(e) => {
                lines.push(format!("DB error: {e}"));
                return lines;
            }
        };
        (account.id, account.password_hash.clone())
    };

    let valid = match mud_data::verify_password(input.trim(), &password_hash) {
        Ok(v) => v,
        Err(e) => {
            lines.push(format!("Password verify error: {e}"));
            return lines;
        }
    };

    if valid {
        flow.echo_on = false;
        {
            let db_guard = db.lock().await;
            let _ = mud_data::update_last_login(db_guard.conn(), account_id);
        }

        flow.account_id = Some(account_id);
        lines.push(String::new());
        lines.push(format!("Welcome back, {username}!"));
        if !world_building_ready(templates) {
            lines
                .push("World building is still in progress — please check back later.".to_string());
            flow.disconnect_requested = true;
            return lines;
        }
        flow.state = LoginState::CharacterSelect;
    } else {
        let new_attempts = attempts + 1;
        if new_attempts >= 3 {
            lines.push("Too many failed attempts. Disconnecting.".to_string());
            flow.disconnect_requested = true;
        } else {
            lines.push(String::new());
            lines.push(format!("Invalid password. ({new_attempts}/3 attempts)"));
            lines.push("Password:".to_string());
            flow.strikes += 1;
            flow.state = LoginState::Password {
                username,
                attempts: new_attempts,
            };
        }
    }
    lines
}

pub fn handle_account_create_confirm_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => {
            if let LoginState::AccountCreateConfirm { username } = &flow.state {
                flow.create_buffer.name = Some(username.to_string());
            }
            flow.echo_on = true;
            lines.push(String::new());
            lines.push("Enter a password (8+ characters):".to_string());
            flow.state = LoginState::AccountCreatePassword;
        }
        "n" | "no" => {
            flow.state = LoginState::Username;
        }
        _ => {
            lines.push("Please answer y or n.".to_string());
        }
    }
    lines
}

pub fn handle_account_create_password_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let password = input.trim();
    if password.len() < 8 {
        lines.push(String::new());
        lines.push("Password must be at least 8 characters.".to_string());
        return lines;
    }

    let username = flow.create_buffer.name.as_deref().map(|s| s.to_string());
    if username.is_none() {
        flow.echo_on = false;
        lines.push(String::new());
        lines.push("Session error. Starting over.".to_string());
        flow.state = LoginState::Username;
        return lines;
    }

    flow.create_buffer.password = Some(password.to_string());
    flow.echo_on = true;
    lines.push(String::new());
    lines.push("Confirm password:".to_string());
    flow.state = LoginState::AccountCreateConfirmPassword;
    lines
}

pub async fn handle_account_create_confirm_password_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<mud_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let confirm = input.trim();
    let stored_password = flow
        .create_buffer
        .password
        .as_deref()
        .map(|s| s.to_string());
    let username = flow.create_buffer.name.as_deref().map(|s| s.to_string());

    if stored_password.is_none() || username.is_none() {
        flow.echo_on = false;
        lines.push(String::new());
        lines.push("Session error. Starting over.".to_string());
        flow.state = LoginState::Username;
        return lines;
    }

    if confirm != stored_password.as_deref().unwrap() {
        lines.push(String::new());
        lines.push("Passwords do not match. Try again.".to_string());
        flow.state = LoginState::AccountCreatePassword;
        return lines;
    }

    let db = match db {
        Some(d) => d,
        None => return lines,
    };

    let hash = match mud_data::hash_password(stored_password.as_deref().unwrap()) {
        Ok(h) => h,
        Err(e) => {
            lines.push(format!("Hashing error: {e}"));
            return lines;
        }
    };
    let username = username.as_deref().unwrap();

    let db_guard = db.lock().await;
    let existing = match mud_data::get_account_by_username(db_guard.conn(), username) {
        Ok(e) => e,
        Err(e) => {
            lines.push(format!("DB error: {e}"));
            return lines;
        }
    };

    if existing.is_some() {
        flow.echo_on = false;
        lines.push(String::new());
        lines.push(
            "That username was taken while you were choosing a password. Starting over."
                .to_string(),
        );
        flow.state = LoginState::Username;
        flow.create_buffer.name = None;
        flow.create_buffer.password = None;
        return lines;
    }

    if let Err(e) = mud_data::create_account(db_guard.conn(), username, &hash) {
        lines.push(format!("Account creation error: {e}"));
        return lines;
    }
    drop(db_guard);

    flow.create_buffer.name = None;
    flow.create_buffer.password = None;

    flow.echo_on = false;
    lines.push(String::new());
    lines.push("Account created! Please log in.".to_string());
    flow.state = LoginState::Username;
    lines
}

pub async fn handle_character_select_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<mud_data::Database>>,
    world: &mut World,
    registry: &ConnectionRegistry,
    _void_room: Entity,
    spawn_room: Entity,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    let db = match db {
        Some(d) => d,
        None => return lines,
    };

    let account_id = match flow.account_id {
        Some(id) => id,
        None => {
            lines.push("Session error. Please log in again.".to_string());
            flow.state = LoginState::Username;
            return lines;
        }
    };

    let db_guard = db.lock().await;
    let chars = match mud_data::get_characters_by_account(db_guard.conn(), account_id) {
        Ok(c) => c,
        Err(e) => {
            lines.push(format!("DB error: {e}"));
            return lines;
        }
    };

    if chars.is_empty() {
        if flow.create_dismissed {
            drop(db_guard);
            match input.to_lowercase().as_str() {
                "c" => {
                    flow.create_buffer.name = None;
                    flow.create_buffer.race = None;
                    flow.create_buffer.class = None;
                    lines.push(String::new());
                    lines.push("--- Create a New Character ---".to_string());
                    flow.state = LoginState::CharacterCreateName;
                }
                "who" | "w" => {
                    lines.extend(super::prompt::list_who(world, registry));
                }
                _ => {
                    lines.push(
                        "Type 'c' to create a character, or 'who' to see who's online.".to_string(),
                    );
                }
            }
            return lines;
        }

        drop(db_guard);
        match input.to_lowercase().as_str() {
            "y" | "yes" | "c" => {
                flow.create_buffer.name = None;
                flow.create_buffer.race = None;
                flow.create_buffer.class = None;
                lines.push(String::new());
                lines.push("--- Create a New Character ---".to_string());
                flow.state = LoginState::CharacterCreateName;
            }
            "n" | "no" => {
                flow.create_dismissed = true;
            }
            "who" | "w" => {
                lines.extend(super::prompt::list_who(world, registry));
            }
            _ => {
                lines.push("You have no characters yet. Create one now? (y/n)".to_string());
            }
        }
        return lines;
    }

    match input.to_lowercase().as_str() {
        "c" => {
            drop(db_guard);
            flow.create_buffer.name = None;
            flow.create_buffer.race = None;
            flow.create_buffer.class = None;
            lines.push(String::new());
            lines.push("--- Create a New Character ---".to_string());
            flow.state = LoginState::CharacterCreateName;
        }
        "who" | "w" => {
            drop(db_guard);
            lines.extend(super::prompt::list_who(world, registry));
        }
        _ => {
            if let Ok(idx) = input.parse::<usize>() {
                if idx == 0 || idx > chars.len() {
                    drop(db_guard);
                    lines.push("Invalid selection. Pick a number from the list, or type 'c' to create a new character.".to_string());
                } else {
                    let char_row = &chars[idx - 1];
                    drop(db_guard); // must drop before load_character (needs mutable world)
                    lines.extend(load_character(flow, world, spawn_room, char_row, db).await);
                }
            } else {
                drop(db_guard);
                lines.push(
                    "Type a number to pick a character, 'c' to create one, or 'who' to see who's online.".to_string(),
                );
            }
        }
    }
    lines
}

pub async fn handle_character_create_name_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<mud_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let name = input.trim();
    if !is_valid_character_name(name) {
        lines.push("Invalid name. Use 3-16 letters, hyphens, or apostrophes.".to_string());
        return lines;
    }

    let db = match db {
        Some(d) => d,
        None => return lines,
    };

    let db_guard = db.lock().await;
    let existing = match mud_data::get_character_by_name(db_guard.conn(), name) {
        Ok(e) => e,
        Err(e) => {
            lines.push(format!("DB error: {e}"));
            return lines;
        }
    };
    drop(db_guard);

    if existing.is_some() {
        lines.push("That name is already taken. Please choose another.".to_string());
        return lines;
    }

    flow.create_buffer.name = Some(name.to_string());
    flow.state = LoginState::CharacterCreateRace;
    lines
}

pub fn handle_character_create_race_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let templates = match templates {
        Some(t) => t,
        None => {
            lines.push("No race templates available. Cannot create character.".to_string());
            return lines;
        }
    };

    let input = input.trim();
    let races: Vec<&str> = templates.races.keys().map(|s| s.as_str()).collect();

    match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= races.len() => {
            let race_id = races[idx - 1].to_string();
            flow.create_buffer.race = Some(race_id);
            flow.state = LoginState::CharacterCreateClass;
        }
        _ => {
            lines.push("Invalid selection.".to_string());
        }
    }
    lines
}

pub fn handle_character_create_class_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let templates = match templates {
        Some(t) => t,
        None => {
            lines.push("No class templates available. Cannot create character.".to_string());
            return lines;
        }
    };

    let race_id = match flow.create_buffer.race.as_deref() {
        Some(r) => r.to_string(),
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let available = templates.available_classes_for_race(&race_id);
    let input = input.trim();

    match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= available.len() => {
            let class_id = available[idx - 1].id.clone();
            flow.create_buffer.class = Some(class_id);
            flow.state = LoginState::CharacterCreateSpawn;
        }
        _ => {
            lines.push("Invalid selection.".to_string());
        }
    }
    lines
}

pub async fn handle_character_create_confirm_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<mud_data::Database>>,
    world: &mut World,
    _void_room: Entity, // used during character creation flows
    spawn_room: Entity,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => {
            lines.extend(finalize_character(flow, db, world, spawn_room, templates).await);
        }
        "n" | "no" => {
            flow.create_buffer.name = None;
            flow.create_buffer.race = None;
            flow.create_buffer.class = None;
            lines.push("Character creation cancelled.".to_string());
            flow.state = LoginState::CharacterSelect;
        }
        _ => {
            lines.push("Type 'y' or 'n'.".to_string());
        }
    }
    lines
}

pub fn handle_spawn_select_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let templates = match templates {
        Some(t) => t,
        None => {
            lines.push("No spawn points available.".to_string());
            return lines;
        }
    };

    let race_id = match flow.create_buffer.race.as_deref() {
        Some(r) => r,
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let class_id = match flow.create_buffer.class.as_deref() {
        Some(c) => c,
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let available = templates.available_spawns(race_id, class_id, "");
    let input = input.trim();

    match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= available.len() => {
            let (area_id, spawn) = available[idx - 1];
            let spawn_key = format!("{}:{}", area_id, spawn.room);
            flow.create_buffer.spawn_key = Some(spawn_key);
            flow.state = LoginState::CharacterCreateConfirm;
        }
        _ => {
            lines.push("Invalid selection.".to_string());
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Character creation finalisation
// ---------------------------------------------------------------------------

async fn finalize_character(
    flow: &mut LoginFlow,
    db: Option<&Mutex<mud_data::Database>>,
    world: &mut World,
    fallback_room: Entity,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();

    let name = match flow.create_buffer.name.as_deref() {
        Some(n) => n.to_string(),
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    let race_id = match flow.create_buffer.race.as_deref() {
        Some(r) => r.to_string(),
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    let class_id = match flow.create_buffer.class.as_deref() {
        Some(c) => c.to_string(),
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    let spawn_key = match flow.create_buffer.spawn_key.as_deref() {
        Some(s) => s.to_string(),
        None => {
            lines.push("Session error: no spawn selected.".to_string());
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    let account_id = match flow.account_id {
        Some(id) => id,
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    let db_con = match db {
        Some(d) => d,
        None => {
            lines.push("Server error: database unavailable.".to_string());
            return lines;
        }
    };

    let (attrs, hp, skills) = compute_character_stats(templates, &race_id, &class_id);

    // Resolve spawn room from spawn key, falling back to area's spawn_room
    let room_entity = resolve_room(templates, world, &spawn_key).unwrap_or(fallback_room);

    let db_guard = db_con.lock().await;
    let conn_db = db_guard.conn();

    let entity_id = match mud_data::insert_entity(conn_db, "player") {
        Ok(id) => id,
        Err(e) => {
            lines.push(format!("Error creating character: {e}"));
            return lines;
        }
    };

    if let Err(e) = mud_data::save_player_component(conn_db, entity_id, account_id, "<%hhp %hmhp> ")
    {
        lines.push(format!("Error saving character: {e}"));
        return lines;
    }

    if let Err(e) = mud_data::save_attributes_component(
        conn_db,
        entity_id,
        &mud_data::AttributesRow {
            strength: attrs.strength,
            dexterity: attrs.dexterity,
            intelligence: attrs.intelligence,
            wisdom: attrs.wisdom,
            constitution: attrs.constitution,
            charisma: attrs.charisma,
        },
    ) {
        lines.push(format!("Error saving attributes: {e}"));
        return lines;
    }

    if let Err(e) = mud_data::save_health_component(conn_db, entity_id, hp, hp) {
        lines.push(format!("Error saving health: {e}"));
        return lines;
    }

    if let Err(e) = mud_data::save_level_component(conn_db, entity_id, 1) {
        lines.push(format!("Error saving level: {e}"));
        return lines;
    }

    if let Err(e) = mud_data::save_experience_component(conn_db, entity_id, 0) {
        lines.push(format!("Error saving experience: {e}"));
        return lines;
    }

    if let Err(e) = mud_data::create_character(
        conn_db,
        account_id,
        &name,
        &race_id,
        &class_id,
        entity_id,
        0,
        Some(&spawn_key),
    ) {
        lines.push(format!("Error saving character: {e}"));
        return lines;
    }

    drop(db_guard);

    let player = world.spawn((
        Position::new(room_entity),
        Name::new(name.clone()),
        Player::new(account_id),
        Race(race_id.clone()),
        Class(class_id),
        attrs,
        Health::new(hp),
        Level::default(),
        Experience::default(),
        skills,
        DbId::new(entity_id),
    ));

    // Apply racial and class passives
    if let Some(templates) = templates {
        mud_core::systems::passive::apply_all_passives(world, player, templates);
    }

    flow.entity = Some(player);
    flow.entity_just_spawned = true;

    lines.push(String::new());
    lines.push("--- Character Score ---".to_string());
    lines.push(format!("  Name:       {name}"));
    lines.push("  Level:      1".to_string());
    lines.push(format!("  HP:         {hp} / {hp}"));
    lines.push(String::new());
    lines.push(format!("Welcome, {name}! Your adventure begins."));
    flow.state = LoginState::Playing;
    lines
}

/// Resolve a spawn key to a room entity, falling back if not found.
fn resolve_room(
    templates: Option<&TemplateRegistry>,
    world: &World,
    spawn_key: &str,
) -> Option<Entity> {
    templates.and_then(|t| t.find_room_by_key(world, spawn_key))
}

// ---------------------------------------------------------------------------
// Load an existing character
// ---------------------------------------------------------------------------

async fn load_character(
    flow: &mut LoginFlow,
    world: &mut World,
    spawn_room: Entity,
    char_row: &mud_data::CharacterRow,
    db: &Mutex<mud_data::Database>,
) -> Vec<String> {
    let mut lines = Vec::new();

    let db_guard = db.lock().await;
    let conn_db = db_guard.conn();

    let entity_id = char_row.entity_id;

    let attrs = mud_data::load_attributes_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|a| {
            mud_core::Attributes::new(
                a.strength,
                a.dexterity,
                a.intelligence,
                a.wisdom,
                a.constitution,
                a.charisma,
            )
        })
        .unwrap_or_default();

    let hp = mud_data::load_health_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|(current, max)| Health { current, max })
        .unwrap_or_else(|| Health::new(20));

    let level = mud_data::load_level_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|l| Level(l as u8))
        .unwrap_or_default();

    let xp = mud_data::load_experience_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|x| Experience(x as u64))
        .unwrap_or_default();

    let hp_current = hp.current;
    let hp_max = hp.max;
    let level_val = level.0;

    drop(db_guard);

    // Resolve room from spawn_key, falling back to the server's spawn_room
    let room = char_row
        .spawn_key
        .as_deref()
        .and_then(|key| crate::get_templates().and_then(|t| t.find_room_by_key(world, key)))
        .unwrap_or(spawn_room);

    let player = world.spawn((
        Position::new(room),
        Name::new(char_row.name.clone()),
        Player::new(char_row.account_id),
        Race(char_row.race.clone()),
        Class(char_row.class.clone()),
        attrs,
        hp,
        level,
        xp,
        DbId::new(entity_id),
    ));

    // Apply racial and class passives
    if let Some(templates) = crate::get_templates() {
        mud_core::systems::passive::apply_all_passives(world, player, &templates);
    }

    flow.entity = Some(player);
    flow.entity_just_spawned = true;

    lines.push(String::new());
    lines.push("--- Character Score ---".to_string());
    lines.push(format!("  Name:       {}", char_row.name));
    lines.push(format!("  Level:      {level_val}"));
    lines.push(format!("  HP:         {hp_current} / {hp_max}"));
    lines.push(String::new());
    lines.push(format!("Welcome back, {}!", char_row.name));
    flow.state = LoginState::Playing;
    lines
}
