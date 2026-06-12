use rusqlite::{params, Connection};

pub fn insert_entity(conn: &Connection, entity_type: &str) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO entities (type) VALUES (?1)",
        params![entity_type],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_entity(conn: &Connection, entity_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM entities WHERE id = ?1", params![entity_id])?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct AccountRow {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub access_level: String,
    pub created_at: String,
    pub last_login: Option<String>,
}

pub fn get_account_by_username(
    conn: &Connection,
    username: &str,
) -> Result<Option<AccountRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, access_level, created_at, last_login \
         FROM accounts WHERE username = ?1",
    )?;
    let mut rows = stmt.query(params![username])?;
    match rows.next()? {
        Some(row) => Ok(Some(AccountRow {
            id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            access_level: row.get(3)?,
            created_at: row.get(4)?,
            last_login: row.get(5)?,
        })),
        None => Ok(None),
    }
}

pub fn create_account(
    conn: &Connection,
    username: &str,
    password_hash: &str,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO accounts (username, password_hash) VALUES (?1, ?2)",
        params![username, password_hash],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_last_login(conn: &Connection, account_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE accounts SET last_login = datetime('now') WHERE id = ?1",
        params![account_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SCHEMA;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        conn
    }

    #[test]
    fn test_insert_entity() {
        let conn = setup();
        let id = insert_entity(&conn, "test_type").unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_delete_entity() {
        let conn = setup();
        let id = insert_entity(&conn, "test_type").unwrap();
        delete_entity(&conn, id).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_insert_multiple_entities() {
        let conn = setup();
        let id1 = insert_entity(&conn, "room").unwrap();
        let id2 = insert_entity(&conn, "player").unwrap();
        assert_ne!(id1, id2);
    }

    // ── Account queries ────────────────────────────────────

    fn hash_password(password: &str) -> String {
        use argon2::password_hash::SaltString;
        use argon2::{Argon2, PasswordHasher};
        use rand_core::OsRng;
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    #[test]
    fn test_create_and_get_account() {
        let conn = setup();
        let hash = hash_password("hunter2");
        let id = create_account(&conn, "testuser", &hash).unwrap();
        assert!(id > 0);

        let account = get_account_by_username(&conn, "testuser")
            .unwrap()
            .expect("account should exist");
        assert_eq!(account.username, "testuser");
        assert_eq!(account.access_level, "player");
        assert_eq!(account.password_hash, hash);
    }

    #[test]
    fn test_get_account_nonexistent() {
        let conn = setup();
        let result = get_account_by_username(&conn, "nobody").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_create_duplicate_username_fails() {
        let conn = setup();
        let hash = hash_password("pass1");
        create_account(&conn, "dupuser", &hash).unwrap();
        let hash2 = hash_password("pass2");
        let result = create_account(&conn, "dupuser", &hash2);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_last_login() {
        let conn = setup();
        let hash = hash_password("pass");
        let id = create_account(&conn, "logintest", &hash).unwrap();
        update_last_login(&conn, id).unwrap();

        let account = get_account_by_username(&conn, "logintest")
            .unwrap()
            .unwrap();
        assert!(account.last_login.is_some());
    }
}
