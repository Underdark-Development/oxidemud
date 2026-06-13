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

// ---------------------------------------------------------------------------
// Character queries
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CharacterRow {
    pub id: i64,
    pub account_id: i64,
    pub name: String,
    pub race: String,
    pub class: String,
    pub level: i64,
    pub experience: i64,
    pub entity_id: i64,
    pub room_id: i64,
    pub created_at: String,
    pub last_seen: Option<String>,
}

pub fn get_characters_by_account(
    conn: &Connection,
    account_id: i64,
) -> Result<Vec<CharacterRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, race, class, level, experience, entity_id, room_id, created_at, last_seen \
         FROM characters WHERE account_id = ?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map(params![account_id], |row| {
        Ok(CharacterRow {
            id: row.get(0)?,
            account_id: row.get(1)?,
            name: row.get(2)?,
            race: row.get(3)?,
            class: row.get(4)?,
            level: row.get(5)?,
            experience: row.get(6)?,
            entity_id: row.get(7)?,
            room_id: row.get(8)?,
            created_at: row.get(9)?,
            last_seen: row.get(10)?,
        })
    })?;
    rows.collect()
}

pub fn get_character_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<CharacterRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, race, class, level, experience, entity_id, room_id, created_at, last_seen \
         FROM characters WHERE name = ?1",
    )?;
    let mut rows = stmt.query(params![name])?;
    match rows.next()? {
        Some(row) => Ok(Some(CharacterRow {
            id: row.get(0)?,
            account_id: row.get(1)?,
            name: row.get(2)?,
            race: row.get(3)?,
            class: row.get(4)?,
            level: row.get(5)?,
            experience: row.get(6)?,
            entity_id: row.get(7)?,
            room_id: row.get(8)?,
            created_at: row.get(9)?,
            last_seen: row.get(10)?,
        })),
        None => Ok(None),
    }
}

pub fn create_character(
    conn: &Connection,
    account_id: i64,
    name: &str,
    race: &str,
    class: &str,
    entity_id: i64,
    room_id: i64,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO characters (account_id, name, race, class, entity_id, room_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![account_id, name, race, class, entity_id, room_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_character(conn: &Connection, character_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM characters WHERE id = ?1",
        params![character_id],
    )?;
    Ok(())
}

pub fn update_character_level(
    conn: &Connection,
    character_id: i64,
    level: i64,
    xp: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET level = ?1, experience = ?2 WHERE id = ?3",
        params![level, xp, character_id],
    )?;
    Ok(())
}

pub fn update_character_position(
    conn: &Connection,
    character_id: i64,
    room_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET room_id = ?1 WHERE id = ?2",
        params![room_id, character_id],
    )?;
    Ok(())
}

pub fn update_character_last_seen(
    conn: &Connection,
    character_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET last_seen = datetime('now') WHERE id = ?1",
        params![character_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Entity persistence helpers — individual component save/load
// ---------------------------------------------------------------------------

pub fn save_room_component(
    conn: &Connection,
    entity_id: i64,
    name: &str,
    description: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_room (entity_id, name, description) VALUES (?1, ?2, ?3)",
        params![entity_id, name, description],
    )?;
    Ok(())
}

pub fn load_room_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(String, String)>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT name, description FROM components_room WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?))),
        None => Ok(None),
    }
}

pub fn save_position_component(
    conn: &Connection,
    entity_id: i64,
    room_entity_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_position (entity_id, room_entity_id) VALUES (?1, ?2)",
        params![entity_id, room_entity_id],
    )?;
    Ok(())
}

pub fn load_position_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<i64>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT room_entity_id FROM components_position WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_player_component(
    conn: &Connection,
    entity_id: i64,
    account_id: i64,
    prompt: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_player (entity_id, account_id, prompt) VALUES (?1, ?2, ?3)",
        params![entity_id, account_id, prompt],
    )?;
    Ok(())
}

pub fn load_player_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i64, String)>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT account_id, prompt FROM components_player WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?))),
        None => Ok(None),
    }
}

pub fn save_npc_component(
    conn: &Connection,
    entity_id: i64,
    template_id: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_npc (entity_id, template_id) VALUES (?1, ?2)",
        params![entity_id, template_id],
    )?;
    Ok(())
}

pub fn load_npc_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT template_id FROM components_npc WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_health_component(
    conn: &Connection,
    entity_id: i64,
    current: i32,
    max: i32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_health (entity_id, current, max) VALUES (?1, ?2, ?3)",
        params![entity_id, current, max],
    )?;
    Ok(())
}

pub fn load_health_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i32, i32)>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT current, max FROM components_health WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?))),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AttributesRow {
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub constitution: u8,
    pub charisma: u8,
}

pub fn save_attributes_component(
    conn: &Connection,
    entity_id: i64,
    attrs: &AttributesRow,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_attributes (entity_id, strength, dexterity, intelligence, wisdom, constitution, charisma) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![entity_id, attrs.strength, attrs.dexterity, attrs.intelligence, attrs.wisdom, attrs.constitution, attrs.charisma],
    )?;
    Ok(())
}

pub fn load_attributes_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<AttributesRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT strength, dexterity, intelligence, wisdom, constitution, charisma \
         FROM components_attributes WHERE entity_id = ?1",
    )?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(AttributesRow {
            strength: row.get::<_, i64>(0)? as u8,
            dexterity: row.get::<_, i64>(1)? as u8,
            intelligence: row.get::<_, i64>(2)? as u8,
            wisdom: row.get::<_, i64>(3)? as u8,
            constitution: row.get::<_, i64>(4)? as u8,
            charisma: row.get::<_, i64>(5)? as u8,
        })),
        None => Ok(None),
    }
}

pub fn save_level_component(
    conn: &Connection,
    entity_id: i64,
    level: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_level (entity_id, level) VALUES (?1, ?2)",
        params![entity_id, level],
    )?;
    Ok(())
}

pub fn load_level_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<i64>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT level FROM components_level WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_experience_component(
    conn: &Connection,
    entity_id: i64,
    xp: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_experience (entity_id, xp) VALUES (?1, ?2)",
        params![entity_id, xp],
    )?;
    Ok(())
}

pub fn load_experience_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<i64>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT xp FROM components_experience WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_entity_attribute(
    conn: &Connection,
    entity_id: i64,
    key: &str,
    value: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO attributes (entity_id, key, value) VALUES (?1, ?2, ?3)",
        params![entity_id, key, value],
    )?;
    Ok(())
}

pub fn delete_entity_attribute(
    conn: &Connection,
    entity_id: i64,
    key: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM attributes WHERE entity_id = ?1 AND key = ?2",
        params![entity_id, key],
    )?;
    Ok(())
}

pub fn load_entity_attributes(
    conn: &Connection,
    entity_id: i64,
) -> Result<Vec<(String, String)>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT key, value FROM attributes WHERE entity_id = ?1")?;
    let rows = stmt.query_map(params![entity_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn load_all_entity_ids(conn: &Connection) -> Result<Vec<(i64, String)>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, type FROM entities ORDER BY id")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn update_entity_type(
    conn: &Connection,
    entity_id: i64,
    entity_type: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE entities SET type = ?1 WHERE id = ?2",
        params![entity_type, entity_id],
    )?;
    Ok(())
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

    // ── Character queries ───────────────────────────────────

    #[test]
    fn test_create_character() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "charowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn, account_id, "Aragorn", "human", "warrior", eid, room_id,
        )
        .unwrap();
        assert!(char_id > 0);
    }

    #[test]
    fn test_get_character_by_name() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "owner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        create_character(&conn, account_id, "Legolas", "elf", "ranger", eid, room_id).unwrap();

        let char_row = get_character_by_name(&conn, "Legolas")
            .unwrap()
            .expect("character should exist");
        assert_eq!(char_row.name, "Legolas");
        assert_eq!(char_row.race, "elf");
        assert_eq!(char_row.class, "ranger");
        assert_eq!(char_row.account_id, account_id);
    }

    #[test]
    fn test_get_character_by_name_nonexistent() {
        let conn = setup();
        let result = get_character_by_name(&conn, "Nobody").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_characters_by_account() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "multiowner", &hash).unwrap();
        let eid1 = insert_entity(&conn, "player").unwrap();
        let eid2 = insert_entity(&conn, "player").unwrap();
        create_character(
            &conn, account_id, "Char1", "human", "warrior", eid1, room_id,
        )
        .unwrap();
        create_character(&conn, account_id, "Char2", "elf", "mage", eid2, room_id).unwrap();

        let chars = get_characters_by_account(&conn, account_id).unwrap();
        assert_eq!(chars.len(), 2);
    }

    #[test]
    fn test_get_characters_by_account_empty() {
        let conn = setup();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "lonely", &hash).unwrap();
        let chars = get_characters_by_account(&conn, account_id).unwrap();
        assert!(chars.is_empty());
    }

    #[test]
    fn test_delete_character() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "delowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn, account_id, "DeleteMe", "human", "warrior", eid, room_id,
        )
        .unwrap();
        delete_character(&conn, char_id).unwrap();

        let result = get_character_by_name(&conn, "DeleteMe").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_character_name_unique() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash1 = hash_password("pass1");
        let hash2 = hash_password("pass2");
        let account1 = create_account(&conn, "unique1", &hash1).unwrap();
        let account2 = create_account(&conn, "unique2", &hash2).unwrap();
        let eid1 = insert_entity(&conn, "player").unwrap();
        let eid2 = insert_entity(&conn, "player").unwrap();
        create_character(
            &conn, account1, "SameName", "human", "warrior", eid1, room_id,
        )
        .unwrap();
        let result = create_character(&conn, account2, "SameName", "elf", "mage", eid2, room_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_character_level() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "levelowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn, account_id, "Leveler", "human", "warrior", eid, room_id,
        )
        .unwrap();
        update_character_level(&conn, char_id, 5, 5000).unwrap();

        let char_row = get_character_by_name(&conn, "Leveler").unwrap().unwrap();
        assert_eq!(char_row.level, 5);
        assert_eq!(char_row.experience, 5000);
    }

    #[test]
    fn test_update_character_position() {
        let conn = setup();
        let room1 = insert_entity(&conn, "room").unwrap();
        let room2 = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "posowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn, account_id, "Wanderer", "human", "warrior", eid, room1,
        )
        .unwrap();
        update_character_position(&conn, char_id, room2).unwrap();

        let char_row = get_character_by_name(&conn, "Wanderer").unwrap().unwrap();
        assert_eq!(char_row.room_id, room2);
    }

    #[test]
    fn test_update_character_last_seen() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "seenowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn, account_id, "SeenMe", "human", "warrior", eid, room_id,
        )
        .unwrap();
        assert!(get_character_by_name(&conn, "SeenMe")
            .unwrap()
            .unwrap()
            .last_seen
            .is_none());
        update_character_last_seen(&conn, char_id).unwrap();
        assert!(get_character_by_name(&conn, "SeenMe")
            .unwrap()
            .unwrap()
            .last_seen
            .is_some());
    }

    // ── Entity persistence tests ─────────────────────────────

    #[test]
    fn test_save_and_load_room_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "room").unwrap();
        save_room_component(&conn, eid, "Test Room", "A test room.").unwrap();
        let (name, desc) = load_room_component(&conn, eid).unwrap().unwrap();
        assert_eq!(name, "Test Room");
        assert_eq!(desc, "A test room.");
    }

    #[test]
    fn test_save_and_load_position_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        let room_id = insert_entity(&conn, "room").unwrap();
        save_position_component(&conn, eid, room_id).unwrap();
        let loaded = load_position_component(&conn, eid).unwrap().unwrap();
        assert_eq!(loaded, room_id);
    }

    #[test]
    fn test_save_and_load_player_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        save_player_component(&conn, eid, 42, "<%hhp> ").unwrap();
        let (account_id, prompt) = load_player_component(&conn, eid).unwrap().unwrap();
        assert_eq!(account_id, 42);
        assert_eq!(prompt, "<%hhp> ");
    }

    #[test]
    fn test_save_and_load_npc_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "npc").unwrap();
        save_npc_component(&conn, eid, "goblin_01").unwrap();
        let template_id = load_npc_component(&conn, eid).unwrap().unwrap();
        assert_eq!(template_id, "goblin_01");
    }

    #[test]
    fn test_save_and_load_health_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        save_health_component(&conn, eid, 80, 100).unwrap();
        let (cur, max) = load_health_component(&conn, eid).unwrap().unwrap();
        assert_eq!(cur, 80);
        assert_eq!(max, 100);
    }

    #[test]
    fn test_save_and_load_attributes_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        save_attributes_component(
            &conn,
            eid,
            &AttributesRow {
                strength: 15,
                dexterity: 12,
                intelligence: 14,
                wisdom: 10,
                constitution: 16,
                charisma: 8,
            },
        )
        .unwrap();
        let attrs = load_attributes_component(&conn, eid).unwrap().unwrap();
        assert_eq!(attrs.strength, 15);
        assert_eq!(attrs.dexterity, 12);
        assert_eq!(attrs.intelligence, 14);
        assert_eq!(attrs.wisdom, 10);
        assert_eq!(attrs.constitution, 16);
        assert_eq!(attrs.charisma, 8);
    }

    #[test]
    fn test_save_and_load_level_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        save_level_component(&conn, eid, 5).unwrap();
        let level = load_level_component(&conn, eid).unwrap().unwrap();
        assert_eq!(level, 5);
    }

    #[test]
    fn test_save_and_load_experience_component() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        save_experience_component(&conn, eid, 12500).unwrap();
        let xp = load_experience_component(&conn, eid).unwrap().unwrap();
        assert_eq!(xp, 12500);
    }

    #[test]
    fn test_save_load_delete_entity_attributes() {
        let conn = setup();
        let eid = insert_entity(&conn, "room").unwrap();
        save_entity_attribute(&conn, eid, "key1", "val1").unwrap();
        save_entity_attribute(&conn, eid, "key2", "val2").unwrap();
        let attrs = load_entity_attributes(&conn, eid).unwrap();
        assert_eq!(attrs.len(), 2);

        delete_entity_attribute(&conn, eid, "key1").unwrap();
        let attrs = load_entity_attributes(&conn, eid).unwrap();
        assert_eq!(attrs.len(), 1);
    }

    #[test]
    fn test_load_all_entity_ids() {
        let conn = setup();
        insert_entity(&conn, "room").unwrap();
        insert_entity(&conn, "player").unwrap();
        insert_entity(&conn, "room").unwrap();
        let ids = load_all_entity_ids(&conn).unwrap();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_update_entity_type() {
        let conn = setup();
        let eid = insert_entity(&conn, "generic").unwrap();
        update_entity_type(&conn, eid, "room").unwrap();
        let ids = load_all_entity_ids(&conn).unwrap();
        assert_eq!(ids[0].1, "room");
    }

    #[test]
    fn test_component_not_found_returns_none() {
        let conn = setup();
        assert!(load_room_component(&conn, 999).unwrap().is_none());
        assert!(load_position_component(&conn, 999).unwrap().is_none());
        assert!(load_player_component(&conn, 999).unwrap().is_none());
        assert!(load_npc_component(&conn, 999).unwrap().is_none());
        assert!(load_health_component(&conn, 999).unwrap().is_none());
        assert!(load_attributes_component(&conn, 999).unwrap().is_none());
        assert!(load_level_component(&conn, 999).unwrap().is_none());
        assert!(load_experience_component(&conn, 999).unwrap().is_none());
    }
}
