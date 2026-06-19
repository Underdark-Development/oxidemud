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
    pub gender: String,
    pub pronoun_subject: String,
    pub pronoun_object: String,
    pub pronoun_possessive: String,
    pub level: i64,
    pub experience: i64,
    pub entity_id: i64,
    pub room_id: Option<i64>,
    pub spawn_key: Option<String>,
    pub created_at: String,
    pub last_seen: Option<String>,
}

pub fn get_characters_by_account(
    conn: &Connection,
    account_id: i64,
) -> Result<Vec<CharacterRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, race, class, gender, pronoun_subject, pronoun_object, pronoun_possessive, level, experience, entity_id, room_id, spawn_key, created_at, last_seen \
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
            room_id: row.get(12)?,
            spawn_key: row.get(13)?,
            created_at: row.get(14)?,
            last_seen: row.get(15)?,
        })
    })?;
    rows.collect()
}

pub fn get_character_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<CharacterRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, race, class, gender, pronoun_subject, pronoun_object, pronoun_possessive, level, experience, entity_id, room_id, spawn_key, created_at, last_seen \
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
            room_id: row.get(12)?,
            spawn_key: row.get(13)?,
            created_at: row.get(14)?,
            last_seen: row.get(15)?,
        })),
        None => Ok(None),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_character(
    conn: &Connection,
    account_id: i64,
    name: &str,
    race: &str,
    class: &str,
    entity_id: i64,
    room_id: Option<i64>,
    spawn_key: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO characters (account_id, name, race, class, entity_id, room_id, spawn_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![account_id, name, race, class, entity_id, room_id, spawn_key],
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

pub fn update_character_position(
    conn: &Connection,
    entity_id: i64,
    room_entity_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE characters SET room_id = ?1 WHERE entity_id = ?2",
        params![room_entity_id, entity_id],
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
    conn.execute(
        "INSERT OR REPLACE INTO components_room (entity_id, name, description, spawn_key) VALUES (?1, ?2, ?3, ?4)",
        params![entity_id, name, description, spawn_key],
    )?;
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

pub fn save_player_component(
    conn: &Connection,
    entity_id: i64,
    account_id: i64,
    prompt: &str,
    screen_width: u16,
    unspent_skill_points: u32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_player (entity_id, account_id, prompt, screen_width, unspent_skill_points) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![entity_id, account_id, prompt, screen_width, unspent_skill_points],
    )?;
    Ok(())
}

pub fn load_player_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i64, String, u16, u32)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT account_id, prompt, screen_width, unspent_skill_points FROM components_player WHERE entity_id = ?1",
    )?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((
            row.get(0)?,
            row.get(1)?,
            row.get::<_, i64>(2)? as u16,
            row.get::<_, i64>(3)? as u32,
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

pub fn save_combat_stats_component(
    conn: &Connection,
    entity_id: i64,
    bab: i32,
    fort: i32,
    ref_: i32,
    will: i32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_combat_stats (entity_id, base_attack_bonus, fort_save, ref_save, will_save) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![entity_id, bab, fort, ref_, will],
    )?;
    Ok(())
}

pub fn load_combat_stats_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i32, i32, i32, i32)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT base_attack_bonus, fort_save, ref_save, will_save FROM components_combat_stats WHERE entity_id = ?1",
    )?;
    let mut rows = stmt.query(params![entity_id])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))),
        None => Ok(None),
    }
}

pub fn save_golds_component(
    conn: &Connection,
    entity_id: i64,
    copper: i64,
    silver: i64,
    gold: i64,
    platinum: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO components_golds (entity_id, copper, silver, gold, platinum) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![entity_id, copper, silver, gold, platinum],
    )?;
    Ok(())
}

pub fn load_golds_component(
    conn: &Connection,
    entity_id: i64,
) -> Result<Option<(i64, i64, i64, i64)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT copper, silver, gold, platinum FROM components_golds WHERE entity_id = ?1",
    )?;
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
            &conn,
            account_id,
            "Aragorn",
            "human",
            "warrior",
            eid,
            Some(room_id),
            None,
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
        create_character(
            &conn,
            account_id,
            "Legolas",
            "elf",
            "ranger",
            eid,
            Some(room_id),
            None,
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
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "multiowner", &hash).unwrap();
        let eid1 = insert_entity(&conn, "player").unwrap();
        let eid2 = insert_entity(&conn, "player").unwrap();
        create_character(
            &conn,
            account_id,
            "Char1",
            "human",
            "warrior",
            eid1,
            Some(room_id),
            None,
        )
        .unwrap();
        create_character(
            &conn,
            account_id,
            "Char2",
            "elf",
            "mage",
            eid2,
            Some(room_id),
            None,
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
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "delowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let char_id = create_character(
            &conn,
            account_id,
            "DeleteMe",
            "human",
            "warrior",
            eid,
            Some(room_id),
            None,
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
            &conn,
            account1,
            "SameName",
            "human",
            "warrior",
            eid1,
            Some(room_id),
            None,
        )
        .unwrap();
        let result = create_character(
            &conn,
            account2,
            "SameName",
            "elf",
            "mage",
            eid2,
            Some(room_id),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_character_level() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "levelowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let _char_id = create_character(
            &conn,
            account_id,
            "Leveler",
            "human",
            "warrior",
            eid,
            Some(room_id),
            None,
        )
        .unwrap();
        update_character_level(&conn, eid, 5, 5000).unwrap();

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
        let _char_id = create_character(
            &conn,
            account_id,
            "Wanderer",
            "human",
            "warrior",
            eid,
            Some(room1),
            None,
        )
        .unwrap();
        update_character_position(&conn, eid, room2).unwrap();

        let char_row = get_character_by_name(&conn, "Wanderer").unwrap().unwrap();
        assert_eq!(char_row.room_id, Some(room2));
    }

    #[test]
    fn test_update_character_last_seen() {
        let conn = setup();
        let room_id = insert_entity(&conn, "room").unwrap();
        let hash = hash_password("pass");
        let account_id = create_account(&conn, "seenowner", &hash).unwrap();
        let eid = insert_entity(&conn, "player").unwrap();
        let _char_id = create_character(
            &conn,
            account_id,
            "SeenMe",
            "human",
            "warrior",
            eid,
            Some(room_id),
            None,
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
        save_player_component(&conn, eid, 42, "<%hhp> ", 80, 5).unwrap();
        let (account_id, prompt, width, unspent) =
            load_player_component(&conn, eid).unwrap().unwrap();
        assert_eq!(account_id, 42);
        assert_eq!(prompt, "<%hhp> ");
        assert_eq!(width, 80);
        assert_eq!(unspent, 5);
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
