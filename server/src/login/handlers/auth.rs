use std::sync::Arc;

use tokio::sync::Mutex;

use oxide_core::templates::TemplateRegistry;

use super::super::state::{
    ChangePasswordSubstate, CharacterSelectSubstate, LoginState, LoginSubstate,
};
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
    flow.state = LoginState::Login(LoginSubstate::Username);
    Vec::new()
}

pub async fn handle_username_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<oxide_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    // API key login: @apikey <key>
    if input == "@apikey" || input.starts_with("@apikey ") {
        let key = input.strip_prefix("@apikey").unwrap().trim();
        if key.is_empty() {
            lines.push("Usage: @apikey <your-api-key>".to_string());
            return lines;
        }
        let db = match db {
            Some(d) => d,
            None => return lines,
        };
        let db_guard = db.lock().await;
        let (account_id, username, _access_level) =
            match oxide_data::validate_api_key(db_guard.conn(), key, Some("spade")) {
                Ok(Some(t)) => t,
                Ok(None) => {
                    lines.push("Invalid or expired API key.".to_string());
                    return lines;
                }
                Err(e) => {
                    lines.push(format!("DB error: {e}"));
                    return lines;
                }
            };
        drop(db_guard);

        {
            let db_guard = db.lock().await;
            let _ = oxide_data::update_last_login(db_guard.conn(), account_id);
        }

        flow.account_id = Some(account_id);
        lines.push(String::new());
        lines.push(format!("Welcome, {username}! (authenticated via API key)"));
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
        return lines;
    }

    let username = input;
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
        flow.state = LoginState::Login(LoginSubstate::Password {
            username: Arc::from(username.to_string()),
            attempts: 0,
        });
    } else {
        flow.state = LoginState::Login(LoginSubstate::AccountCreateConfirm {
            username: Arc::from(username.to_string()),
        });
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
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
    } else {
        let new_attempts = attempts + 1;
        if new_attempts >= 3 {
            lines.push("Too many failed attempts. Disconnecting.".to_string());
            flow.disconnect_requested = true;
        } else {
            lines.push(String::new());
            lines.push(format!("Invalid password. ({new_attempts}/3 attempts)"));
            flow.echo_on = true;
            flow.strikes += 1;
            flow.state = LoginState::Login(LoginSubstate::Password {
                username,
                attempts: new_attempts,
            });
        }
    }
    lines
}

pub fn handle_account_create_confirm_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => {
            if let LoginState::Login(LoginSubstate::AccountCreateConfirm { username }) = &flow.state
            {
                flow.create_buffer.name = Some(username.to_string());
            }
            flow.echo_on = true;
            flow.state = LoginState::Login(LoginSubstate::AccountCreatePassword);
        }
        "n" | "no" => {
            flow.state = LoginState::Login(LoginSubstate::Username);
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
        flow.state = LoginState::Login(LoginSubstate::Username);
        return lines;
    }

    flow.create_buffer.password = Some(password.to_string());
    flow.echo_on = true;
    flow.state = LoginState::Login(LoginSubstate::AccountCreateConfirmPassword);
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
        flow.state = LoginState::Login(LoginSubstate::Username);
        return lines;
    }

    if confirm != stored_password.as_deref().unwrap() {
        lines.push(String::new());
        lines.push("Passwords do not match. Try again.".to_string());
        flow.state = LoginState::Login(LoginSubstate::AccountCreatePassword);
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
        flow.state = LoginState::Login(LoginSubstate::Username);
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
    flow.state = LoginState::Login(LoginSubstate::Username);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::LoginFlow;
    use crate::login::LoginState;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_handle_change_password_states() {
        let db = Mutex::new(oxide_data::Database::open_in_memory().unwrap());

        // 1. Create account
        let hash = oxide_data::hash_password("oldpassword").unwrap();
        let account_id = {
            let db_guard = db.lock().await;
            oxide_data::create_account(db_guard.conn(), "pwchange_user", &hash).unwrap()
        };

        let mut flow = LoginFlow::new();
        flow.account_id = Some(account_id);
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
            ChangePasswordSubstate::Old,
        ));

        // 2. Submit wrong password
        let lines = handle_change_password_old_state(&mut flow, "wrongpass", Some(&db)).await;
        assert!(lines.iter().any(|l| l.contains("Invalid password")));
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::List)
        );
        assert!(!flow.echo_on);

        // 3. Reset state & submit correct old password
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
            ChangePasswordSubstate::Old,
        ));
        let lines = handle_change_password_old_state(&mut flow, "oldpassword", Some(&db)).await;
        assert!(lines.is_empty());
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
                ChangePasswordSubstate::New
            ))
        );
        assert!(flow.echo_on);

        // 4. Submit new password that is too short
        let lines = handle_change_password_new_state(&mut flow, "short");
        assert!(lines
            .iter()
            .any(|l| l.contains("must be at least 8 characters")));
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::List)
        );
        assert!(!flow.echo_on);

        // 5. Submit valid new password
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
            ChangePasswordSubstate::New,
        ));
        let lines = handle_change_password_new_state(&mut flow, "newsecretpass");
        assert!(lines.is_empty());
        match &flow.state {
            LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
                ChangePasswordSubstate::Confirm { new_password },
            )) => {
                assert_eq!(&**new_password, "newsecretpass");
            }
            _ => panic!("Expected ChangePasswordConfirm state"),
        }
        assert!(flow.echo_on);

        // 6. Confirm password with mismatch
        let new_pw = match &flow.state {
            LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
                ChangePasswordSubstate::Confirm { new_password },
            )) => new_password.clone(),
            _ => unreachable!(),
        };
        let lines =
            handle_change_password_confirm_state(&mut flow, "mismatch", new_pw.clone(), Some(&db))
                .await;
        assert!(lines.iter().any(|l| l.contains("do not match")));
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::List)
        );
        assert!(!flow.echo_on);

        // 7. Confirm password successfully
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
            ChangePasswordSubstate::Confirm {
                new_password: new_pw.clone(),
            },
        ));
        let lines =
            handle_change_password_confirm_state(&mut flow, "newsecretpass", new_pw, Some(&db))
                .await;
        assert!(lines.iter().any(|l| l.contains("changed successfully")));
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::List)
        );
        assert!(!flow.echo_on);

        // 8. Verify password hash updated in DB
        let db_guard = db.lock().await;
        let acc = oxide_data::get_account_by_id(db_guard.conn(), account_id)
            .unwrap()
            .unwrap();
        assert!(oxide_data::verify_password("newsecretpass", &acc.password_hash).unwrap());
    }

    #[tokio::test]
    async fn test_handle_character_delete_confirm() {
        let db = Mutex::new(oxide_data::Database::open_in_memory().unwrap());
        let hash = oxide_data::hash_password("pass").unwrap();
        let (account_id, char_id) = {
            let db_guard = db.lock().await;
            let aid = oxide_data::create_account(db_guard.conn(), "deluser", &hash).unwrap();
            let eid = oxide_data::insert_entity(db_guard.conn(), "player").unwrap();
            let cid = oxide_data::create_character(
                db_guard.conn(),
                aid,
                "DelCharName",
                "human",
                "warrior",
                eid,
                Some("test:room"),
                None,
            )
            .unwrap();
            (aid, cid)
        };

        let mut flow = LoginFlow::new();
        flow.account_id = Some(account_id);
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::CharacterDeleteConfirm {
            character_id: char_id,
            name: Arc::from("DelCharName"),
        });

        // 1. Cancel deletion
        let lines = handle_character_delete_confirm_state(
            &mut flow,
            "no",
            char_id,
            Arc::from("DelCharName"),
            Some(&db),
        )
        .await;
        assert!(lines.iter().any(|l| l.contains("cancelled")));
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::List)
        );

        // Verify character still exists
        {
            let db_guard = db.lock().await;
            let ch = oxide_data::get_character_by_name(db_guard.conn(), "DelCharName").unwrap();
            assert!(ch.is_some());
        }

        // 2. Perform deletion
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::CharacterDeleteConfirm {
            character_id: char_id,
            name: Arc::from("DelCharName"),
        });
        let lines = handle_character_delete_confirm_state(
            &mut flow,
            "yes",
            char_id,
            Arc::from("DelCharName"),
            Some(&db),
        )
        .await;
        assert!(lines.iter().any(|l| l.contains("permanently deleted")));
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::List)
        );

        // Verify character is gone
        {
            let db_guard = db.lock().await;
            let ch = oxide_data::get_character_by_name(db_guard.conn(), "DelCharName").unwrap();
            assert!(ch.is_none());
        }
    }

    #[tokio::test]
    async fn test_apikey_login() {
        let db = Mutex::new(oxide_data::Database::open_in_memory().unwrap());

        // Create account + API key with spade scope
        let hash = oxide_data::hash_password("password123").unwrap();
        let account_id = {
            let db_guard = db.lock().await;
            oxide_data::create_account(db_guard.conn(), "apikey_user", &hash).unwrap()
        };
        let api_key = {
            let db_guard = db.lock().await;
            oxide_data::insert_api_key(
                db_guard.conn(),
                "test-key-spade",
                account_id,
                None,
                None,
                &["spade"],
            )
            .unwrap();
            "test-key-spade"
        };

        // 1. Valid @apikey login
        let mut flow = LoginFlow::new();
        flow.state = LoginState::Login(LoginSubstate::Username);
        let lines = handle_username_state(&mut flow, &format!("@apikey {api_key}"), Some(&db)).await;
        assert!(lines.iter().any(|l| l.contains("Welcome")));
        assert!(lines.iter().any(|l| l.contains("API key")));
        assert_eq!(
            flow.state,
            LoginState::CharacterSelect(CharacterSelectSubstate::List)
        );
        assert_eq!(flow.account_id, Some(account_id));

        // 2. Invalid @apikey
        let mut flow = LoginFlow::new();
        flow.state = LoginState::Login(LoginSubstate::Username);
        let lines =
            handle_username_state(&mut flow, "@apikey invalid-key", Some(&db)).await;
        assert!(lines.iter().any(|l| l.contains("Invalid")));
        assert!(matches!(flow.state, LoginState::Login(LoginSubstate::Username)));

        // 3. Empty @apikey
        let mut flow = LoginFlow::new();
        flow.state = LoginState::Login(LoginSubstate::Username);
        let lines = handle_username_state(&mut flow, "@apikey ", Some(&db)).await;
        assert!(lines.iter().any(|l| l.contains("Usage")));
    }
}

pub async fn handle_change_password_old_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<oxide_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let db = match db {
        Some(d) => d,
        None => {
            flow.echo_on = false;
            lines
                .push("Server error: database unavailable. Password change cancelled.".to_string());
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
            return lines;
        }
    };

    let account_id = match flow.account_id {
        Some(id) => id,
        None => {
            flow.echo_on = false;
            lines.push("Session error. Please log in again.".to_string());
            flow.state = LoginState::Login(LoginSubstate::Username);
            return lines;
        }
    };

    let db_guard = db.lock().await;
    let account = match oxide_data::get_account_by_id(db_guard.conn(), account_id) {
        Ok(Some(acc)) => acc,
        _ => {
            flow.echo_on = false;
            lines.push("Account not found. Password change cancelled.".to_string());
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
            return lines;
        }
    };

    let valid =
        oxide_data::verify_password(input.trim(), &account.password_hash).unwrap_or_default();

    if valid {
        flow.echo_on = true;
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
            ChangePasswordSubstate::New,
        ));
    } else {
        flow.echo_on = false;
        lines.push("Invalid password. Password change cancelled.".to_string());
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
    }
    lines
}

pub fn handle_change_password_new_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let new_pw = input.trim();
    if new_pw.len() < 8 {
        flow.echo_on = false;
        lines
            .push("Password must be at least 8 characters. Password change cancelled.".to_string());
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
        return lines;
    }

    flow.echo_on = true;
    flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::ChangePassword(
        ChangePasswordSubstate::Confirm {
            new_password: Arc::from(new_pw),
        },
    ));
    lines
}

pub async fn handle_change_password_confirm_state(
    flow: &mut LoginFlow,
    input: &str,
    new_password: Arc<str>,
    db: Option<&Mutex<oxide_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let confirm = input.trim();
    if confirm != &*new_password {
        flow.echo_on = false;
        lines.push("Passwords do not match. Password change cancelled.".to_string());
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
        return lines;
    }

    let db = match db {
        Some(d) => d,
        None => {
            flow.echo_on = false;
            lines
                .push("Server error: database unavailable. Password change cancelled.".to_string());
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
            return lines;
        }
    };

    let account_id = match flow.account_id {
        Some(id) => id,
        None => {
            flow.echo_on = false;
            lines.push("Session error. Please log in again.".to_string());
            flow.state = LoginState::Login(LoginSubstate::Username);
            return lines;
        }
    };

    let hash = match oxide_data::hash_password(&new_password) {
        Ok(h) => h,
        Err(e) => {
            flow.echo_on = false;
            lines.push(format!(
                "Error hashing password: {e}. Password change cancelled."
            ));
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
            return lines;
        }
    };

    let db_guard = db.lock().await;
    match oxide_data::update_account_password(db_guard.conn(), account_id, &hash) {
        Ok(_) => {
            flow.echo_on = false;
            lines.push("Password changed successfully.".to_string());
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
        }
        Err(e) => {
            flow.echo_on = false;
            lines.push(format!(
                "DB error updating password: {e}. Password change cancelled."
            ));
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
        }
    }
    lines
}

pub async fn handle_character_delete_confirm_state(
    flow: &mut LoginFlow,
    input: &str,
    character_id: i64,
    name: Arc<str>,
    db: Option<&Mutex<oxide_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let confirmation = input.trim().to_lowercase();
    if confirmation == "y" || confirmation == "yes" {
        let db = match db {
            Some(d) => d,
            None => {
                lines.push("Server error: database unavailable. Deletion cancelled.".to_string());
                flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
                return lines;
            }
        };

        let db_guard = db.lock().await;
        match oxide_data::delete_character(db_guard.conn(), character_id) {
            Ok(_) => {
                lines.push(format!(
                    "Character '{}' has been permanently deleted.",
                    name
                ));
            }
            Err(e) => {
                lines.push(format!("DB error deleting character: {e}"));
            }
        }
    } else {
        lines.push("Character deletion cancelled.".to_string());
    }

    flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
    lines
}

pub async fn handle_account_delete_confirm_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<oxide_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let db = match db {
        Some(d) => d,
        None => {
            flow.echo_on = false;
            lines.push("Server error: database unavailable. Deletion cancelled.".to_string());
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
            return lines;
        }
    };

    let account_id = match flow.account_id {
        Some(id) => id,
        None => {
            flow.echo_on = false;
            lines.push("Session error. Please log in again.".to_string());
            flow.state = LoginState::Login(LoginSubstate::Username);
            return lines;
        }
    };

    let db_guard = db.lock().await;
    let account = match oxide_data::get_account_by_id(db_guard.conn(), account_id) {
        Ok(Some(acc)) => acc,
        _ => {
            flow.echo_on = false;
            lines.push("Account not found. Deletion cancelled.".to_string());
            flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
            return lines;
        }
    };

    let valid =
        oxide_data::verify_password(input.trim(), &account.password_hash).unwrap_or_default();

    if valid {
        match oxide_data::delete_account(db_guard.conn(), account_id) {
            Ok(_) => {
                flow.echo_on = false;
                flow.account_id = None;
                flow.create_dismissed = false;
                flow.state = LoginState::Login(LoginSubstate::Username);
                lines.push(
                    "Your account and all associated characters have been permanently deleted."
                        .to_string(),
                );
            }
            Err(e) => {
                flow.echo_on = false;
                lines.push(format!(
                    "DB error deleting account: {e}. Deletion cancelled."
                ));
                flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
            }
        }
    } else {
        flow.echo_on = false;
        lines.push("Invalid password. Account deletion cancelled.".to_string());
        flow.state = LoginState::CharacterSelect(CharacterSelectSubstate::List);
    }
    lines
}
