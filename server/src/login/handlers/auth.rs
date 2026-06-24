use std::sync::Arc;

use tokio::sync::Mutex;

use oxide_core::templates::TemplateRegistry;

use super::super::state::LoginState;
use super::super::LoginFlow;

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

fn world_building_ready(templates: Option<&TemplateRegistry>) -> bool {
    match templates {
        Some(t) => !t.races.is_empty(),
        None => false,
    }
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
    db: Option<&Mutex<oxide_data::Database>>,
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
    let existing = match oxide_data::get_account_by_username(db_guard.conn(), username) {
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
    db: Option<&Mutex<oxide_data::Database>>,
    username: Arc<str>,
    attempts: u8,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();

    if input.is_empty() {
        flow.echo_on = true;
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
        let account = match oxide_data::get_account_by_username(db_guard.conn(), &username) {
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

    let valid = match oxide_data::verify_password(input.trim(), &password_hash) {
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
            let _ = oxide_data::update_last_login(db_guard.conn(), account_id);
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
            flow.echo_on = true;
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
    db: Option<&Mutex<oxide_data::Database>>,
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

    let hash = match oxide_data::hash_password(stored_password.as_deref().unwrap()) {
        Ok(h) => h,
        Err(e) => {
            lines.push(format!("Hashing error: {e}"));
            return lines;
        }
    };
    let username = username.as_deref().unwrap();

    let db_guard = db.lock().await;
    let existing = match oxide_data::get_account_by_username(db_guard.conn(), username) {
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

    if let Err(e) = oxide_data::create_account(db_guard.conn(), username, &hash) {
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
