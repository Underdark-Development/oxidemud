pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS components_room (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    spawn_key TEXT
);

CREATE TABLE IF NOT EXISTS components_exit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    direction TEXT NOT NULL,
    dest_entity_id INTEGER NOT NULL REFERENCES entities(id),
    flags INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS components_position (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    room_entity_id INTEGER NOT NULL REFERENCES entities(id)
);

CREATE TABLE IF NOT EXISTS components_player (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    account_id INTEGER NOT NULL,
    prompt TEXT,
    screen_width INTEGER NOT NULL DEFAULT 80
);

CREATE TABLE IF NOT EXISTS components_npc (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    template_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS components_health (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    current INTEGER NOT NULL,
    max INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS components_mana (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    current INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS components_stamina (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    current INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS components_attributes (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    strength INTEGER NOT NULL DEFAULT 10,
    dexterity INTEGER NOT NULL DEFAULT 10,
    intelligence INTEGER NOT NULL DEFAULT 10,
    wisdom INTEGER NOT NULL DEFAULT 10,
    constitution INTEGER NOT NULL DEFAULT 10,
    charisma INTEGER NOT NULL DEFAULT 10
);

CREATE TABLE IF NOT EXISTS components_level (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    level INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS components_experience (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    xp INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS components_practice_points (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    points INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS components_appearance (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    height INTEGER NOT NULL,
    weight INTEGER NOT NULL,
    build TEXT NOT NULL,
    hair_color TEXT NOT NULL,
    hair_style TEXT NOT NULL,
    eye_color TEXT NOT NULL,
    skin_tone TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS components_age (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    age INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS components_deity (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    deity TEXT NOT NULL
);

-- Phase 3 tables
CREATE TABLE IF NOT EXISTS components_item (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    template_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS components_durability (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    current INTEGER NOT NULL DEFAULT 100,
    max INTEGER NOT NULL DEFAULT 100,
    decay_rate REAL NOT NULL DEFAULT 1.0
);

CREATE TABLE IF NOT EXISTS components_weapon (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    damage_dice TEXT NOT NULL,
    damage_type TEXT NOT NULL,
    speed REAL NOT NULL DEFAULT 1.0,
    weapon_range TEXT NOT NULL DEFAULT 'melee'
);

CREATE TABLE IF NOT EXISTS components_armor (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    base INTEGER NOT NULL DEFAULT 0,
    bonus INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS components_combat_stats (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    base_attack_bonus INTEGER NOT NULL DEFAULT 0,
    fort_save INTEGER NOT NULL DEFAULT 0,
    ref_save INTEGER NOT NULL DEFAULT 0,
    will_save INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS components_golds (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    copper INTEGER NOT NULL DEFAULT 0,
    silver INTEGER NOT NULL DEFAULT 0,
    gold INTEGER NOT NULL DEFAULT 0,
    platinum INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS components_skills (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    skill_id TEXT NOT NULL,
    rank INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (entity_id, skill_id)
);

CREATE TABLE IF NOT EXISTS components_alignment (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    alignment TEXT NOT NULL DEFAULT 'true_neutral'
);

CREATE TABLE IF NOT EXISTS components_description (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS components_equipment (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    slot TEXT NOT NULL,
    item_entity_id INTEGER NOT NULL REFERENCES entities(id),
    PRIMARY KEY (entity_id, slot)
);

CREATE TABLE IF NOT EXISTS components_inventory_items (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    item_entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    slot INTEGER NOT NULL,
    PRIMARY KEY (entity_id, item_entity_id)
);

CREATE TABLE IF NOT EXISTS components_stance (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    stance_id TEXT
);

CREATE TABLE IF NOT EXISTS attributes (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (entity_id, key)
);

-- indexes
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);
CREATE INDEX IF NOT EXISTS idx_components_exit_dest ON components_exit(dest_entity_id);
CREATE INDEX IF NOT EXISTS idx_components_equipment_item ON components_equipment(item_entity_id);
CREATE INDEX IF NOT EXISTS idx_components_inventory_item ON components_inventory_items(item_entity_id);

-- existing tables
CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    access_level TEXT NOT NULL DEFAULT 'player',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_login TEXT,
    show_motd INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS characters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL UNIQUE,
    race TEXT NOT NULL,
    class TEXT NOT NULL,
    gender TEXT NOT NULL DEFAULT 'neutral',
    pronoun_subject TEXT NOT NULL DEFAULT 'they',
    pronoun_object TEXT NOT NULL DEFAULT 'them',
    pronoun_possessive TEXT NOT NULL DEFAULT 'their',
    level INTEGER NOT NULL DEFAULT 1,
    experience INTEGER NOT NULL DEFAULT 0,
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    room_id INTEGER REFERENCES entities(id),
    spawn_key TEXT,
    recall_room_id INTEGER REFERENCES entities(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen TEXT
);

CREATE INDEX IF NOT EXISTS idx_characters_account ON characters(account_id);
";

pub const VERSION: i64 = 15;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_runs() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
    }

    #[test]
    fn test_schema_creates_tables() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let expected = [
            "accounts",
            "attributes",
            "characters",
            "components_age",
            "components_alignment",
            "components_appearance",
            "components_armor",
            "components_attributes",
            "components_combat_stats",
            "components_deity",
            "components_description",
            "components_durability",
            "components_equipment",
            "components_exit",
            "components_experience",
            "components_golds",
            "components_health",
            "components_inventory_items",
            "components_item",
            "components_level",
            "components_mana",
            "components_npc",
            "components_player",
            "components_practice_points",
            "components_position",
            "components_room",
            "components_skills",
            "components_stamina",
            "components_stance",
            "components_weapon",
            "entities",
            "schema_version",
        ];
        for name in &expected {
            assert!(tables.iter().any(|t| t == name), "missing table: {name}");
        }
    }
}
