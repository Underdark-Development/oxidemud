use rusqlite::{params, Connection};

use oxide_core::Appearance;

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

pub fn insert_api_key(
    conn: &Connection,
    key: &str,
    account_id: i64,
    description: Option<&str>,
    expires_at: Option<&str>,
    scopes: &[&str],
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO api_keys (key, account_id, description, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![key, account_id, description, expires_at],
    )?;
    for scope in scopes {
        conn.execute(
            "INSERT INTO api_key_scopes (key, scope) VALUES (?1, ?2)",
            params![key, scope],
        )?;
    }
    Ok(())
}

/// Validate an API key, optionally requiring a specific scope.
///
/// If `required_scope` is `Some`, the key must have that scope. If `None`, any
/// valid (non-expired) key matches.
pub fn validate_api_key(
    conn: &Connection,
    key: &str,
    required_scope: Option<&str>,
) -> Result<Option<(i64, String, String)>, rusqlite::Error> {
    let (sql, param): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match required_scope {
        Some(scope) => (
            "SELECT a.id, a.username, a.access_level
             FROM api_keys k
             JOIN accounts a ON k.account_id = a.id
             JOIN api_key_scopes s ON s.key = k.key
             WHERE k.key = ?1
               AND s.scope = ?2
               AND (k.expires_at IS NULL OR k.expires_at > datetime('now'))",
            vec![Box::new(key.to_string()), Box::new(scope.to_string())],
        ),
        None => (
            "SELECT a.id, a.username, a.access_level
             FROM api_keys k
             JOIN accounts a ON k.account_id = a.id
             WHERE k.key = ?1
               AND (k.expires_at IS NULL OR k.expires_at > datetime('now'))",
            vec![Box::new(key.to_string())],
        ),
    };
    let mut stmt = conn.prepare(sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = param.iter().map(|p| p.as_ref()).collect();
    let mut rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    if let Some(res) = rows.next() {
        Ok(Some(res?))
    } else {
        Ok(None)
    }
}

pub fn add_api_key_scope(conn: &Connection, key: &str, scope: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR IGNORE INTO api_key_scopes (key, scope) VALUES (?1, ?2)",
        params![key, scope],
    )?;
    Ok(())
}

pub fn remove_api_key_scope(
    conn: &Connection,
    key: &str,
    scope: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM api_key_scopes WHERE key = ?1 AND scope = ?2",
        params![key, scope],
    )?;
    Ok(())
}

pub fn save_world_time(
    conn: &Connection,
    hour: u8,
    minute: u8,
    day: u32,
    season: &str,
    year: u32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO world_time (id, hour, minute, day, season, year)
         VALUES (1, ?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
           hour = excluded.hour,
           minute = excluded.minute,
           day = excluded.day,
           season = excluded.season,
           year = excluded.year",
        params![hour, minute, day, season, year],
    )?;
    Ok(())
}

pub type WorldTimeRecord = (u8, u8, u32, String, u32);

pub fn load_world_time(conn: &Connection) -> Result<Option<WorldTimeRecord>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT hour, minute, day, season, year FROM world_time WHERE id = 1")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let hour: u8 = row.get(0)?;
        let minute: u8 = row.get(1)?;
        let day: u32 = row.get(2)?;
        let season: String = row.get(3)?;
        let year: u32 = row.get(4)?;
        Ok(Some((hour, minute, day, season, year)))
    } else {
        Ok(None)
    }
}

pub type WeatherStateRecord = (String, Option<String>, Option<String>);

pub fn save_weather_state(
    conn: &Connection,
    zone_id: &str,
    base: Option<&str>,
    modifier: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO weather_states (zone_id, base, modifier)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(zone_id) DO UPDATE SET
           base = excluded.base,
           modifier = excluded.modifier",
        params![zone_id, base, modifier],
    )?;
    Ok(())
}

pub fn load_weather_states(conn: &Connection) -> Result<Vec<WeatherStateRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT zone_id, base, modifier FROM weather_states")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

pub fn revoke_api_key(conn: &Connection, key: &str) -> Result<u32, rusqlite::Error> {
    let rows = conn.execute("DELETE FROM api_keys WHERE key = ?1", params![key])?;
    Ok(rows as u32)
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

pub fn get_account_by_id(
    conn: &Connection,
    id: i64,
) -> Result<Option<AccountRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, access_level, created_at, last_login \
         FROM accounts WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
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
    pub gender: String,
    pub pronoun_subject: String,
    pub pronoun_object: String,
    pub pronoun_possessive: String,
    pub level: i64,
    pub experience: i64,
    pub entity_id: i64,
    pub spawn_key: Option<String>,
    pub current_room_key: Option<String>,
    pub recall_room_key: Option<String>,
    pub created_at: String,
    pub last_seen: Option<String>,
}

pub fn get_characters_by_account(
    conn: &Connection,
    account_id: i64,
) -> Result<Vec<CharacterRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, race, class, gender, pronoun_subject, pronoun_object, pronoun_possessive, level, experience, entity_id, spawn_key, current_room_key, recall_room_key, created_at, last_seen \
         FROM characters WHERE account_id = ?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map(params![account_id], |row| {
        Ok(CharacterRow {
            id: row.get(0)?,
            account_id: row.get(1)?,
            name: row.get(2)?,
            race: row.get(3)?,
            class: row.get(4)?,
            gender: row.get(5)?,
            pronoun_subject: row.get(6)?,
            pronoun_object: row.get(7)?,
            pronoun_possessive: row.get(8)?,
            level: row.get(9)?,
            experience: row.get(10)?,
            entity_id: row.get(11)?,
            spawn_key: row.get(12)?,
            current_room_key: row.get(13)?,
            recall_room_key: row.get(14)?,
            created_at: row.get(15)?,
            last_seen: row.get(16)?,
        })
    })?;
    rows.collect()
}

pub fn get_character_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<CharacterRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, race, class, gender, pronoun_subject, pronoun_object, pronoun_possessive, level, experience, entity_id, spawn_key, current_room_key, recall_room_key, created_at, last_seen \
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
            gender: row.get(5)?,
            pronoun_subject: row.get(6)?,
            pronoun_object: row.get(7)?,
            pronoun_possessive: row.get(8)?,
            level: row.get(9)?,
            experience: row.get(10)?,
            entity_id: row.get(11)?,
            spawn_key: row.get(12)?,
            current_room_key: row.get(13)?,
            recall_room_key: row.get(14)?,
            created_at: row.get(15)?,
            last_seen: row.get(16)?,
        })),
        None => Ok(None),
    }
}

pub struct CreateCharacterParams {
    pub account_id: i64,
    pub name: String,
    pub race: String,
    pub class: String,
    pub entity_id: i64,
    pub spawn_key: Option<String>,
    pub current_room_key: Option<String>,
}

pub fn create_character(
    conn: &Connection,
    params: &CreateCharacterParams,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO characters (account_id, name, race, class, entity_id, spawn_key, current_room_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            params.account_id,
            params.name,
            params.race,
            params.class,
            params.entity_id,
            params.spawn_key,
            params.current_room_key,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_character(conn: &Connection, character_id: i64) -> Result<(), rusqlite::Error> {
    // 1. Get the entity_id of the character
    let mut stmt = conn.prepare("SELECT entity_id FROM characters WHERE id = ?1")?;
    let mut rows = stmt.query(params![character_id])?;
    let entity_id = match rows.next()? {
        Some(row) => Some(row.get::<_, i64>(0)?),
        None => None,
    };

    // 2. Delete the character row
    conn.execute(
        "DELETE FROM characters WHERE id = ?1",
        params![character_id],
    )?;

    // 3. Delete the entity row (which cascades and deletes components)
    if let Some(eid) = entity_id {
        conn.execute("DELETE FROM entities WHERE id = ?1", params![eid])?;
    }

    Ok(())
}

pub fn update_account_password(
    conn: &Connection,
    account_id: i64,
    new_password_hash: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE accounts SET password_hash = ?1 WHERE id = ?2",
        params![new_password_hash, account_id],
    )?;
    Ok(())
}

pub fn delete_account(conn: &Connection, account_id: i64) -> Result<(), rusqlite::Error> {
    // 1. Get all character entity IDs for this account
    let mut stmt = conn.prepare("SELECT entity_id FROM characters WHERE account_id = ?1")?;
    let mut rows = stmt.query(params![account_id])?;
    let mut entity_ids = Vec::new();
    while let Some(row) = rows.next()? {
        entity_ids.push(row.get::<_, i64>(0)?);
    }

    // 2. Delete the account (this will cascade delete api_keys and characters)
    conn.execute("DELETE FROM accounts WHERE id = ?1", params![account_id])?;

    // 3. Delete the entities (this will cascade delete all component tables data)
    for entity_id in entity_ids {
        conn.execute("DELETE FROM entities WHERE id = ?1", params![entity_id])?;
    }

    Ok(())
}

pub fn update_character_level(
    conn: &Connection,
    entity_id: i64,
    level: i64,
    xp: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET level = ?1, experience = ?2 WHERE entity_id = ?3",
        params![level, xp, entity_id],
    )?;
    Ok(())
}

pub fn update_character_spawn_key(
    conn: &Connection,
    entity_id: i64,
    spawn_key: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET spawn_key = ?1 WHERE entity_id = ?2",
        params![spawn_key, entity_id],
    )?;
    Ok(())
}

pub fn update_character_current_room_key(
    conn: &Connection,
    entity_id: i64,
    current_room_key: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET current_room_key = ?1 WHERE entity_id = ?2",
        params![current_room_key, entity_id],
    )?;
    Ok(())
}

pub fn update_character_recall_room_key(
    conn: &Connection,
    entity_id: i64,
    recall_room_key: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET recall_room_key = ?1 WHERE entity_id = ?2",
        params![recall_room_key, entity_id],
    )?;
    Ok(())
}

pub fn update_character_gender(
    conn: &Connection,
    character_id: i64,
    gender: &str,
    pronoun_subject: &str,
    pronoun_object: &str,
    pronoun_possessive: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET gender = ?1, pronoun_subject = ?2, pronoun_object = ?3, pronoun_possessive = ?4 WHERE id = ?5",
        params![gender, pronoun_subject, pronoun_object, pronoun_possessive, character_id],
    )?;
    Ok(())
}

pub fn update_character_last_seen(
    conn: &Connection,
    entity_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET last_seen = datetime('now') WHERE entity_id = ?1",
        params![entity_id],
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
    spawn_key: Option<&str>,
) -> Result<(), rusqlite::Error> {
    if let Some(spawn_key) = spawn_key {
        conn.execute(
            "INSERT OR REPLACE INTO components_room (entity_id, name, description, spawn_key) VALUES (?1, ?2, ?3, ?4)",
            params![entity_id, name, description, spawn_key],
        )?;
    } else {
        conn.execute(
            "INSERT OR REPLACE INTO components_room (entity_id, name, description, spawn_key) VALUES (?1, ?2, ?3, NULL)",
            params![entity_id, name, description],
        )?;
    }
    Ok(())
}

pub fn load_room_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(String, String, Option<String>)>, rusqlite::Error> {
    let mut stmt = conn
        .prepare("SELECT name, description, spawn_key FROM components_room WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?))),
        None => Ok(None),
    }
}

pub fn load_all_room_spawn_keys(conn: &Connection) -> Result<Vec<(i64, String)>, rusqlite::Error> {
    let mut stmt = conn
        .prepare("SELECT entity_id, spawn_key FROM components_room WHERE spawn_key IS NOT NULL")?;
    let mut rows = stmt.query([])?;
    let mut results = Vec::new();
    while let Some(row) = rows.next()? {
        results.push((row.get(0)?, row.get(1)?));
    }
    Ok(results)
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

pub const PLAYER_COLS: &[&str] = &["account_id", "prompt", "screen_width"];

pub fn save_player_component(
    conn: &Connection,
    entity_id: i64,
    account_id: i64,
    prompt: Option<&str>,
    screen_width: u16,
) -> Result<(), rusqlite::Error> {
    let sql = format!(
        "INSERT OR REPLACE INTO components_player (entity_id, {}) VALUES (?1, ?2, ?3, ?4)",
        PLAYER_COLS.join(", ")
    );
    if let Some(prompt) = prompt {
        conn.execute(&sql, params![entity_id, account_id, prompt, screen_width])?;
    } else {
        conn.execute(
            &sql,
            params![entity_id, account_id, Option::<&str>::None, screen_width],
        )?;
    }
    Ok(())
}

type PlayerComponent = (i64, Option<String>, u16);

pub fn load_player_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<PlayerComponent>, rusqlite::Error> {
    let sql = format!(
        "SELECT {} FROM components_player WHERE entity_id = ?1",
        PLAYER_COLS.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((
            row.get(0)?,
            row.get(1)?,
            row.get::<_, i64>(2)? as u16,
        ))),
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

pub const HEALTH_COLS: &[&str] = &["current", "max"];

pub fn save_health_component(
    conn: &Connection,
    entity_id: i64,
    current: i32,
    max: i32,
) -> Result<(), rusqlite::Error> {
    let sql = format!(
        "INSERT OR REPLACE INTO components_health (entity_id, {}) VALUES (?1, ?2, ?3)",
        HEALTH_COLS.join(", ")
    );
    conn.execute(&sql, params![entity_id, current, max])?;
    Ok(())
}

pub fn save_mana_component(
    conn: &Connection,
    entity_id: i64,
    current: i32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_mana (entity_id, current) VALUES (?1, ?2)",
        params![entity_id, current],
    )?;
    Ok(())
}

pub fn load_mana_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<i32>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT current FROM components_mana WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_stamina_component(
    conn: &Connection,
    entity_id: i64,
    current: i32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_stamina (entity_id, current) VALUES (?1, ?2)",
        params![entity_id, current],
    )?;
    Ok(())
}

pub fn load_stamina_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<i32>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT current FROM components_stamina WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn load_health_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i32, i32)>, rusqlite::Error> {
    let sql = format!(
        "SELECT {} FROM components_health WHERE entity_id = ?1",
        HEALTH_COLS.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?))),
        None => Ok(None),
    }
}

pub const APPEARANCE_COLS: &[&str] = &[
    "height",
    "weight",
    "build",
    "hair_color",
    "hair_style",
    "eye_color",
    "skin_tone",
];

pub fn save_appearance_component(
    conn: &Connection,
    entity_id: i64,
    appearance: &Appearance,
) -> Result<(), rusqlite::Error> {
    let sql = format!(
        "INSERT OR REPLACE INTO components_appearance (entity_id, {}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        APPEARANCE_COLS.join(", ")
    );
    conn.execute(
        &sql,
        params![
            entity_id,
            appearance.height as i32,
            appearance.weight as i32,
            appearance.build,
            appearance.hair_color,
            appearance.hair_style,
            appearance.eye_color,
            appearance.skin_tone,
        ],
    )?;
    Ok(())
}

#[allow(clippy::type_complexity)]
pub fn load_appearance_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i32, i32, String, String, String, String, String)>, rusqlite::Error> {
    let sql = format!(
        "SELECT {} FROM components_appearance WHERE entity_id = ?1",
        APPEARANCE_COLS.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))),
        None => Ok(None),
    }
}

pub fn save_age_component(
    conn: &Connection,
    entity_id: i64,
    age: i32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_age (entity_id, age) VALUES (?1, ?2)",
        params![entity_id, age],
    )?;
    Ok(())
}

pub fn load_age_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<i32>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT age FROM components_age WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_deity_component(
    conn: &Connection,
    entity_id: i64,
    deity: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_deity (entity_id, deity) VALUES (?1, ?2)",
        params![entity_id, deity],
    )?;
    Ok(())
}

pub fn load_deity_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT deity FROM components_deity WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub const ATTRIBUTES_COLS: &[&str] = &[
    "strength",
    "dexterity",
    "intelligence",
    "wisdom",
    "constitution",
    "charisma",
];

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
    let sql = format!(
        "INSERT OR REPLACE INTO components_attributes (entity_id, {}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        ATTRIBUTES_COLS.join(", ")
    );
    conn.execute(
        &sql,
        params![
            entity_id,
            attrs.strength,
            attrs.dexterity,
            attrs.intelligence,
            attrs.wisdom,
            attrs.constitution,
            attrs.charisma
        ],
    )?;
    Ok(())
}

pub fn load_attributes_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<AttributesRow>, rusqlite::Error> {
    let sql = format!(
        "SELECT {} FROM components_attributes WHERE entity_id = ?1",
        ATTRIBUTES_COLS.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
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

pub fn save_practice_points(
    conn: &Connection,
    entity_id: i64,
    points: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_practice_points (entity_id, points) VALUES (?1, ?2)",
        params![entity_id, points],
    )?;
    Ok(())
}

pub fn load_practice_points(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<i64>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT points FROM components_practice_points WHERE entity_id = ?1")?;
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

pub fn set_account_access_level(
    conn: &Connection,
    account_id: i64,
    access_level: &str,
) -> Result<(), rusqlite::Error> {
    let valid = ["player", "builder", "immortal", "god", "admin"];
    if !valid.contains(&access_level) {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Invalid access level '{access_level}'. Valid: {}",
            valid.join(", ")
        )));
    }
    conn.execute(
        "UPDATE accounts SET access_level = ?1 WHERE id = ?2",
        params![access_level, account_id],
    )?;
    Ok(())
}

pub fn set_account_password_hash(
    conn: &Connection,
    account_id: i64,
    password_hash: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE accounts SET password_hash = ?1 WHERE id = ?2",
        params![password_hash, account_id],
    )?;
    Ok(())
}

pub fn set_character_field(
    conn: &Connection,
    character_id: i64,
    field: &str,
    value: &str,
) -> Result<(), rusqlite::Error> {
    match field {
        "level" | "experience" | "xp" => {
            let val: i64 = value
                .parse()
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let sql = format!("UPDATE characters SET {field} = ?1 WHERE id = ?2");
            conn.execute(&sql, params![val, character_id])?;
        }
        "name" | "race" | "class" => {
            let sql = format!("UPDATE characters SET {field} = ?1 WHERE id = ?2");
            conn.execute(&sql, params![value, character_id])?;
        }
        _ => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Unknown character field '{field}'. Valid: level, xp, name, race, class"
            )));
        }
    }
    Ok(())
}

pub fn update_last_login(conn: &Connection, account_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE accounts SET last_login = datetime('now') WHERE id = ?1",
        params![account_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 3 — Item / Equipment / Inventory queries
// ---------------------------------------------------------------------------

pub fn save_item_component(
    conn: &Connection,
    entity_id: i64,
    template_id: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_item (entity_id, template_id) VALUES (?1, ?2)",
        params![entity_id, template_id],
    )?;
    Ok(())
}

pub fn load_item_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT template_id FROM components_item WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_durability_component(
    conn: &Connection,
    entity_id: i64,
    current: u16,
    max: u16,
    decay_rate: f64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_durability (entity_id, current, max, decay_rate) VALUES (?1, ?2, ?3, ?4)",
        params![entity_id, current, max, decay_rate],
    )?;
    Ok(())
}

pub fn load_durability_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(u16, u16, f64)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT current, max, decay_rate FROM components_durability WHERE entity_id = ?1",
    )?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?))),
        None => Ok(None),
    }
}

pub fn save_weapon_component(
    conn: &Connection,
    entity_id: i64,
    damage_dice: &str,
    damage_type: &str,
    speed: f64,
    weapon_range: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_weapon (entity_id, damage_dice, damage_type, speed, weapon_range) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![entity_id, damage_dice, damage_type, speed, weapon_range],
    )?;
    Ok(())
}

pub fn load_weapon_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(String, String, f64, String)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT damage_dice, damage_type, speed, weapon_range FROM components_weapon WHERE entity_id = ?1",
    )?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))),
        None => Ok(None),
    }
}

pub fn save_armor_component(
    conn: &Connection,
    entity_id: i64,
    base: i32,
    bonus: i32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_armor (entity_id, base, bonus) VALUES (?1, ?2, ?3)",
        params![entity_id, base, bonus],
    )?;
    Ok(())
}

pub fn load_armor_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i32, i32)>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT base, bonus FROM components_armor WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?))),
        None => Ok(None),
    }
}

pub const COMBAT_STATS_COLS: &[&str] = &["base_attack_bonus", "fort_save", "ref_save", "will_save"];

pub fn save_combat_stats_component(
    conn: &Connection,
    entity_id: i64,
    bab: i32,
    fort: i32,
    ref_: i32,
    will: i32,
) -> Result<(), rusqlite::Error> {
    let sql = format!(
        "INSERT OR REPLACE INTO components_combat_stats (entity_id, {}) VALUES (?1, ?2, ?3, ?4, ?5)",
        COMBAT_STATS_COLS.join(", ")
    );
    conn.execute(&sql, params![entity_id, bab, fort, ref_, will])?;
    Ok(())
}

pub fn load_combat_stats_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i32, i32, i32, i32)>, rusqlite::Error> {
    let sql = format!(
        "SELECT {} FROM components_combat_stats WHERE entity_id = ?1",
        COMBAT_STATS_COLS.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))),
        None => Ok(None),
    }
}

pub const GOLDS_COLS: &[&str] = &["copper", "silver", "gold", "platinum"];

pub fn save_golds_component(
    conn: &Connection,
    entity_id: i64,
    copper: i64,
    silver: i64,
    gold: i64,
    platinum: i64,
) -> Result<(), rusqlite::Error> {
    let sql = format!(
        "INSERT OR REPLACE INTO components_golds (entity_id, {}) VALUES (?1, ?2, ?3, ?4, ?5)",
        GOLDS_COLS.join(", ")
    );
    conn.execute(&sql, params![entity_id, copper, silver, gold, platinum])?;
    Ok(())
}

pub fn load_golds_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i64, i64, i64, i64)>, rusqlite::Error> {
    let sql = format!(
        "SELECT {} FROM components_golds WHERE entity_id = ?1",
        GOLDS_COLS.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))),
        None => Ok(None),
    }
}

// ── Alignment persistence ──

pub fn save_alignment_component(
    conn: &Connection,
    entity_id: i64,
    alignment: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_alignment (entity_id, alignment) VALUES (?1, ?2)",
        params![entity_id, alignment],
    )?;
    Ok(())
}

pub fn load_alignment_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT alignment FROM components_alignment WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

// ── Description persistence ──

pub fn save_description_component(
    conn: &Connection,
    entity_id: i64,
    description: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_description (entity_id, description) VALUES (?1, ?2)",
        params![entity_id, description],
    )?;
    Ok(())
}

pub fn save_skills(
    conn: &Connection,
    entity_id: i64,
    skills: &std::collections::HashMap<String, u16>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM components_skills WHERE entity_id = ?1",
        params![entity_id],
    )?;
    let mut stmt = conn
        .prepare("INSERT INTO components_skills (entity_id, skill_id, rank) VALUES (?1, ?2, ?3)")?;
    for (skill_id, rank) in skills {
        stmt.execute(params![entity_id, skill_id, rank])?;
    }
    Ok(())
}

pub fn load_skills(
    conn: &Connection,
    entity_id: i64,
) -> Result<std::collections::HashMap<String, u16>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT skill_id, rank FROM components_skills WHERE entity_id = ?1 ORDER BY skill_id",
    )?;
    let rows = stmt.query_map(params![entity_id], |row| {
        let skill_id: String = row.get(0)?;
        let rank: u16 = row.get(1)?;
        Ok((skill_id, rank))
    })?;
    let mut map = std::collections::HashMap::new();
    for row in rows {
        let (skill_id, rank) = row?;
        map.insert(skill_id, rank);
    }
    Ok(map)
}

pub fn load_description_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT description FROM components_description WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_equipment_slot(
    conn: &Connection,
    entity_id: i64,
    slot: &str,
    item_entity_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_equipment (entity_id, slot, item_entity_id) VALUES (?1, ?2, ?3)",
        params![entity_id, slot, item_entity_id],
    )?;
    Ok(())
}

pub fn delete_equipment_slot(
    conn: &Connection,
    entity_id: i64,
    slot: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM components_equipment WHERE entity_id = ?1 AND slot = ?2",
        params![entity_id, slot],
    )?;
    Ok(())
}

pub fn load_equipment(
    conn: &Connection,
    entity_id: i64,
) -> Result<Vec<(String, i64)>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT slot, item_entity_id FROM components_equipment WHERE entity_id = ?1")?;
    let rows = stmt.query_map(params![entity_id], |row| {
        let slot = row.get(0)?;
        let item_id: i64 = row.get(1)?;
        Ok((slot, item_id))
    })?;
    rows.collect()
}

pub fn add_inventory_item(
    conn: &Connection,
    entity_id: i64,
    item_entity_id: i64,
    slot: i32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_inventory_items (entity_id, item_entity_id, slot) VALUES (?1, ?2, ?3)",
        params![entity_id, item_entity_id, slot],
    )?;
    Ok(())
}

pub fn remove_inventory_item(
    conn: &Connection,
    entity_id: i64,
    item_entity_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM components_inventory_items WHERE entity_id = ?1 AND item_entity_id = ?2",
        params![entity_id, item_entity_id],
    )?;
    Ok(())
}

pub fn delete_all_inventory(conn: &Connection, entity_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM components_inventory_items WHERE entity_id = ?1",
        params![entity_id],
    )?;
    Ok(())
}

pub fn delete_all_equipment(conn: &Connection, entity_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM components_equipment WHERE entity_id = ?1",
        params![entity_id],
    )?;
    Ok(())
}

pub fn load_inventory(
    conn: &Connection,
    entity_id: i64,
) -> Result<Vec<(i64, i32)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT item_entity_id, slot FROM components_inventory_items WHERE entity_id = ?1 ORDER BY slot",
    )?;
    let rows = stmt.query_map(params![entity_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn save_stance_component(
    conn: &Connection,
    entity_id: i64,
    stance_id: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_stance (entity_id, stance_id) VALUES (?1, ?2)",
        params![entity_id, stance_id],
    )?;
    Ok(())
}

pub fn load_stance_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<Option<String>>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT stance_id FROM components_stance WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get::<_, Option<String>>(0)?)),
        None => Ok(None),
    }
}

pub fn save_quest_log_component(
    conn: &Connection,
    entity_id: i64,
    json: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_quest_log (entity_id, log_json) VALUES (?1, ?2)",
        params![entity_id, json],
    )?;
    Ok(())
}

pub fn load_quest_log_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT log_json FROM components_quest_log WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_faction_standing_component(
    conn: &Connection,
    entity_id: i64,
    json: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_faction_standing (entity_id, standing_json) VALUES (?1, ?2)",
        params![entity_id, json],
    )?;
    Ok(())
}

pub fn load_faction_standing_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT standing_json FROM components_faction_standing WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_learned_recipes_component(
    conn: &Connection,
    entity_id: i64,
    json: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_learned_recipes (entity_id, recipes_json) VALUES (?1, ?2)",
        params![entity_id, json],
    )?;
    Ok(())
}

pub fn load_learned_recipes_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT recipes_json FROM components_learned_recipes WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_channel_prefs(
    conn: &Connection,
    entity_id: i64,
    json: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_channel_prefs (entity_id, prefs_json) VALUES (?1, ?2)",
        params![entity_id, json],
    )?;
    Ok(())
}

pub fn load_channel_prefs(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT prefs_json FROM components_channel_prefs WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn save_multiclass_component(
    conn: &Connection,
    entity_id: i64,
    json: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_multiclass (entity_id, multiclass_json) VALUES (?1, ?2)",
        params![entity_id, json],
    )?;
    Ok(())
}

pub fn load_multiclass_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT multiclass_json FROM components_multiclass WHERE entity_id = ?1")?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

// ── Report queries ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReportRow {
    pub id: i64,
    pub reporter_name: String,
    pub report_type: String,
    pub message: String,
    pub room_key: Option<String>,
    pub status: String,
    pub staff_notes: String,
    pub closed_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ReportReplyRow {
    pub id: i64,
    pub report_id: i64,
    pub staff_name: String,
    pub message: String,
    pub created_at: String,
    pub seen_by_player: bool,
}

pub fn insert_report(
    conn: &Connection,
    reporter_name: &str,
    report_type: &str,
    message: &str,
    room_key: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO reports (reporter_name, report_type, message, room_key) VALUES (?1, ?2, ?3, ?4)",
        params![reporter_name, report_type, message, room_key],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn load_report(conn: &Connection, id: i64) -> Result<Option<ReportRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, reporter_name, report_type, message, room_key, status, staff_notes, closed_by, created_at, updated_at \
         FROM reports WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(ReportRow {
            id: row.get(0)?,
            reporter_name: row.get(1)?,
            report_type: row.get(2)?,
            message: row.get(3)?,
            room_key: row.get(4)?,
            status: row.get(5)?,
            staff_notes: row.get(6)?,
            closed_by: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })),
        None => Ok(None),
    }
}

pub fn load_reports(
    conn: &Connection,
    status_filter: Option<&str>,
) -> Result<Vec<ReportRow>, rusqlite::Error> {
    let (sql, param): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match status_filter {
        Some(s) if !s.is_empty() => (
            "SELECT id, reporter_name, report_type, message, room_key, status, staff_notes, closed_by, created_at, updated_at \
             FROM reports WHERE status = ?1 ORDER BY created_at DESC".to_string(),
            vec![Box::new(s.to_string())],
        ),
        _ => (
            "SELECT id, reporter_name, report_type, message, room_key, status, staff_notes, closed_by, created_at, updated_at \
             FROM reports ORDER BY created_at DESC".to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = param.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(ReportRow {
            id: row.get(0)?,
            reporter_name: row.get(1)?,
            report_type: row.get(2)?,
            message: row.get(3)?,
            room_key: row.get(4)?,
            status: row.get(5)?,
            staff_notes: row.get(6)?,
            closed_by: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    rows.collect()
}

pub fn update_report_status(
    conn: &Connection,
    id: i64,
    status: &str,
    staff_notes: &str,
    closed_by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE reports SET status = ?1, staff_notes = ?2, closed_by = ?3, updated_at = datetime('now') WHERE id = ?4",
        params![status, staff_notes, closed_by, id],
    )?;
    Ok(())
}

pub fn add_report_reply(
    conn: &Connection,
    report_id: i64,
    staff_name: &str,
    message: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO report_replies (report_id, staff_name, message) VALUES (?1, ?2, ?3)",
        params![report_id, staff_name, message],
    )?;
    Ok(())
}

pub fn load_replies_for_report(
    conn: &Connection,
    report_id: i64,
) -> Result<Vec<ReportReplyRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, report_id, staff_name, message, created_at, seen_by_player \
         FROM report_replies WHERE report_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![report_id], |row| {
        Ok(ReportReplyRow {
            id: row.get(0)?,
            report_id: row.get(1)?,
            staff_name: row.get(2)?,
            message: row.get(3)?,
            created_at: row.get(4)?,
            seen_by_player: row.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect()
}

pub fn count_unread_replies(
    conn: &Connection,
    reporter_name: &str,
) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*)
         FROM report_replies rr
         JOIN reports r ON r.id = rr.report_id
         WHERE r.reporter_name = ?1 AND rr.seen_by_player = 0",
        params![reporter_name],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SCHEMA;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
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
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "charowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "Aragorn".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();
        assert!(char_id > 0);
    }

    #[test]
    fn test_get_character_by_name() {
        let conn = setup();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "owner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "Legolas".into(),
                race: "elf".into(),
                class: "ranger".into(),
                entity_id: eid,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();

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
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "multiowner", &hash).unwrap();
        let eid1 = insert_entity(&conn, "player").unwrap();
        let eid2 = insert_entity(&conn, "player").unwrap();
        create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "Char1".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid1,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();
        create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "Char2".into(),
                race: "elf".into(),
                class: "mage".into(),
                entity_id: eid2,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();

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
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "delowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "DeleteMe".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();
        delete_character(&conn, char_id).unwrap();

        let result = get_character_by_name(&conn, "DeleteMe").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_account_password() {
        let conn = setup();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "pwowner", &hash).unwrap();

        let new_hash = hash_password("newpass");
        update_account_password(&conn, account_id, &new_hash).unwrap();

        let account = get_account_by_id(&conn, account_id).unwrap().unwrap();
        assert_eq!(account.password_hash, new_hash);
    }

    #[test]
    fn test_delete_account() {
        let conn = setup();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "accdelowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let _char_id = create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "CharToDel".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();

        delete_account(&conn, account_id).unwrap();

        let account = get_account_by_id(&conn, account_id).unwrap();
        assert!(account.is_none());

        let result = get_character_by_name(&conn, "CharToDel").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_character_name_unique() {
        let conn = setup();
        let hash1 = hash_password("pass1");
        let hash2 = hash_password("pass2");
        let account1 = create_account(&conn, "unique1", &hash1).unwrap();
        let account2 = create_account(&conn, "unique2", &hash2).unwrap();
        let eid1 = insert_entity(&conn, "player").unwrap();
        let eid2 = insert_entity(&conn, "player").unwrap();
        create_character(
            &conn,
            &CreateCharacterParams {
                account_id: account1,
                name: "SameName".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid1,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();
        let result = create_character(
            &conn,
            &CreateCharacterParams {
                account_id: account2,
                name: "SameName".into(),
                race: "elf".into(),
                class: "mage".into(),
                entity_id: eid2,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_character_level() {
        let conn = setup();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "levelowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let _char_id = create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "Leveler".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();
        update_character_level(&conn, eid, 5, 5000).unwrap();

        let char_row = get_character_by_name(&conn, "Leveler").unwrap().unwrap();
        assert_eq!(char_row.level, 5);
        assert_eq!(char_row.experience, 5000);
    }

    #[test]
    fn test_update_character_current_room_key() {
        let conn = setup();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "posowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let _char_id = create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "Wanderer".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid,
                spawn_key: Some("test:room1".into()),
                current_room_key: None,
            },
        )
        .unwrap();
        update_character_current_room_key(&conn, eid, "test:room2").unwrap();

        let char_row = get_character_by_name(&conn, "Wanderer").unwrap().unwrap();
        assert_eq!(char_row.current_room_key, Some("test:room2".to_string()));
    }

    #[test]
    fn test_update_character_last_seen() {
        let conn = setup();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "seenowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let _char_id = create_character(
            &conn,
            &CreateCharacterParams {
                account_id,
                name: "SeenMe".into(),
                race: "human".into(),
                class: "warrior".into(),
                entity_id: eid,
                spawn_key: Some("test:room".into()),
                current_room_key: None,
            },
        )
        .unwrap();
        assert!(get_character_by_name(&conn, "SeenMe")
            .unwrap()
            .unwrap()
            .last_seen
            .is_none());
        update_character_last_seen(&conn, eid).unwrap();
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
        save_room_component(
            &conn,
            eid,
            "Test Room",
            "A test room.",
            Some("Riverside:town_square"),
        )
        .unwrap();
        let (name, desc, spawn_key) = load_room_component(&conn, eid).unwrap().unwrap();
        assert_eq!(name, "Test Room");
        assert_eq!(desc, "A test room.");
        assert_eq!(spawn_key, Some("Riverside:town_square".to_string()));
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
        save_player_component(&conn, eid, 42, Some("<%hhp> "), 80).unwrap();
        let (account_id, prompt, width) = load_player_component(&conn, eid).unwrap().unwrap();
        assert_eq!(account_id, 42);
        assert_eq!(prompt, Some("<%hhp> ".to_string()));
        assert_eq!(width, 80);
    }

    #[test]
    fn test_player_component_overwrite_with_none_prompt() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        save_player_component(&conn, eid, 42, Some("<%hhp %hmhp> "), 80).unwrap();
        let (_, prompt, _) = load_player_component(&conn, eid).unwrap().unwrap();
        assert_eq!(prompt, Some("<%hhp %hmhp> ".to_string()));
        save_player_component(&conn, eid, 42, None, 80).unwrap();
        let (account_id, prompt, width) = load_player_component(&conn, eid).unwrap().unwrap();
        assert_eq!(account_id, 42);
        assert_eq!(prompt, None);
        assert_eq!(width, 80);
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
    fn test_save_and_load_practice_points() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        save_practice_points(&conn, eid, 42).unwrap();
        let points = load_practice_points(&conn, eid).unwrap().unwrap();
        assert_eq!(points, 42);
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

    #[test]
    fn test_player_component_overwrite_file_based() {
        let tmp =
            std::env::temp_dir().join(format!("test_prompt_overwrite_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        for ext in ["db-wal", "db-shm"] {
            let _ = std::fs::remove_file(tmp.with_extension(ext));
        }

        let conn = Connection::open(&tmp).unwrap();
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .unwrap();
        conn.execute_batch(SCHEMA).unwrap();

        let eid = insert_entity(&conn, "player").unwrap();
        save_player_component(&conn, eid, 42, Some("<%hhp %hmhp> "), 80).unwrap();
        let (_, prompt, _) = load_player_component(&conn, eid).unwrap().unwrap();
        assert_eq!(prompt, Some("<%hhp %hmhp> ".to_string()));

        save_player_component(&conn, eid, 42, None, 80).unwrap();
        let (account_id, prompt, width) = load_player_component(&conn, eid).unwrap().unwrap();
        assert_eq!(account_id, 42);
        assert_eq!(prompt, None);
        assert_eq!(width, 80);

        // Close and reopen to verify cross-connection persistence
        drop(conn);
        let conn2 = Connection::open(&tmp).unwrap();
        conn2
            .execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .unwrap();
        let (account_id, prompt, width) = load_player_component(&conn2, eid).unwrap().unwrap();
        assert_eq!(account_id, 42);
        assert_eq!(prompt, None);
        assert_eq!(width, 80);

        // Cleanup temp files
        std::fs::remove_file(&tmp).ok();
        for ext in ["db-wal", "db-shm"] {
            std::fs::remove_file(tmp.with_extension(ext)).ok();
        }
    }

    #[test]
    fn test_save_and_load_appearance_age_deity() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();

        // 1. Appearance
        save_appearance_component(
            &conn,
            eid,
            &Appearance {
                height: 70,
                weight: 180,
                build: "athletic".into(),
                hair_color: "blonde".into(),
                hair_style: "spiky".into(),
                eye_color: "blue".into(),
                skin_tone: "tan".into(),
            },
        )
        .unwrap();
        let (height, weight, build, hair_color, hair_style, eye_color, skin_tone) =
            load_appearance_component(&conn, eid).unwrap().unwrap();
        assert_eq!(height, 70);
        assert_eq!(weight, 180);
        assert_eq!(build, "athletic");
        assert_eq!(hair_color, "blonde");
        assert_eq!(hair_style, "spiky");
        assert_eq!(eye_color, "blue");
        assert_eq!(skin_tone, "tan");

        // 2. Age
        save_age_component(&conn, eid, 25).unwrap();
        let age = load_age_component(&conn, eid).unwrap().unwrap();
        assert_eq!(age, 25);

        // 3. Deity
        save_deity_component(&conn, eid, "solaris").unwrap();
        let deity = load_deity_component(&conn, eid).unwrap().unwrap();
        assert_eq!(deity, "solaris");
    }

    #[test]
    fn test_save_and_load_multiclass() {
        let conn = setup();
        let eid = insert_entity(&conn, "player").unwrap();
        let test_json = r#"{"classes":[{"id":"warrior","level":5,"is_favored":true}]}"#;
        save_multiclass_component(&conn, eid, test_json).unwrap();
        let loaded = load_multiclass_component(&conn, eid).unwrap().unwrap();
        assert_eq!(loaded, test_json);
    }

    // ── Report queries ────────────────────────────────────

    #[test]
    fn test_insert_and_load_report() {
        let conn = setup();
        let id = insert_report(&conn, "PlayerA", "bug", "It broken", None).unwrap();
        assert!(id > 0);

        let report = load_report(&conn, id)
            .unwrap()
            .expect("report should exist");
        assert_eq!(report.reporter_name, "PlayerA");
        assert_eq!(report.report_type, "bug");
        assert_eq!(report.message, "It broken");
        assert_eq!(report.status, "open");
        assert!(report.room_key.is_none());
    }

    #[test]
    fn test_insert_report_with_room_key() {
        let conn = setup();
        let id = insert_report(
            &conn,
            "PlayerB",
            "idea",
            "Add more goblins",
            Some("starting_vale:forest_path"),
        )
        .unwrap();
        let report = load_report(&conn, id).unwrap().unwrap();
        assert_eq!(
            report.room_key,
            Some("starting_vale:forest_path".to_string())
        );
    }

    #[test]
    fn test_load_reports_filters_by_status() {
        let conn = setup();
        insert_report(&conn, "P1", "bug", "bug one", None).unwrap();
        insert_report(&conn, "P2", "idea", "idea one", None).unwrap();
        let closed_id = insert_report(&conn, "P3", "typo", "typo one", None).unwrap();

        update_report_status(&conn, closed_id, "closed", "fixed", Some("Staffer")).unwrap();

        let open = load_reports(&conn, Some("open")).unwrap();
        assert_eq!(open.len(), 2);

        let closed = load_reports(&conn, Some("closed")).unwrap();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].id, closed_id);
        assert_eq!(closed[0].status, "closed");
        assert_eq!(closed[0].closed_by, Some("Staffer".to_string()));
    }

    #[test]
    fn test_load_reports_no_filter_returns_all() {
        let conn = setup();
        insert_report(&conn, "P1", "bug", "msg1", None).unwrap();
        insert_report(&conn, "P2", "idea", "msg2", None).unwrap();
        let all = load_reports(&conn, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_update_report_status() {
        let conn = setup();
        let id = insert_report(&conn, "P1", "bug", "help", None).unwrap();
        update_report_status(&conn, id, "closed", "not a bug", Some("Wizard")).unwrap();

        let report = load_report(&conn, id).unwrap().unwrap();
        assert_eq!(report.status, "closed");
        assert_eq!(report.staff_notes, "not a bug");
        assert_eq!(report.closed_by, Some("Wizard".to_string()));
    }

    #[test]
    fn test_add_and_load_replies() {
        let conn = setup();
        let id = insert_report(&conn, "PlayerX", "bug", "save error", None).unwrap();
        add_report_reply(&conn, id, "StaffA", "Looking into it.").unwrap();
        add_report_reply(&conn, id, "StaffB", "Fixed in next patch.").unwrap();

        let replies = load_replies_for_report(&conn, id).unwrap();
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0].staff_name, "StaffA");
        assert_eq!(replies[0].message, "Looking into it.");
        assert_eq!(replies[1].staff_name, "StaffB");
    }

    #[test]
    fn test_count_unread_replies() {
        let conn = setup();
        let id = insert_report(&conn, "PlayerY", "bug", "crash", None).unwrap();
        add_report_reply(&conn, id, "StaffA", "Can you reproduce?").unwrap();
        // reply not seen yet

        let count = count_unread_replies(&conn, "PlayerY").unwrap();
        assert_eq!(count, 1);

        // Second reply
        add_report_reply(&conn, id, "StaffA", "Never mind, found it.").unwrap();
        let count = count_unread_replies(&conn, "PlayerY").unwrap();
        assert_eq!(count, 2);

        // Different player doesn't see these
        let count_other = count_unread_replies(&conn, "PlayerZ").unwrap();
        assert_eq!(count_other, 0);
    }

    #[test]
    fn test_report_not_found_returns_none() {
        let conn = setup();
        assert!(load_report(&conn, 999).unwrap().is_none());
    }
}
