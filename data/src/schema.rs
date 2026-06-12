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
    description TEXT NOT NULL DEFAULT ''
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
    prompt TEXT NOT NULL DEFAULT '<%hhp %hmhp> '
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

CREATE TABLE IF NOT EXISTS attributes (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (entity_id, key)
);

CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);
CREATE INDEX IF NOT EXISTS idx_components_exit_dest ON components_exit(dest_entity_id);

CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    access_level TEXT NOT NULL DEFAULT 'player',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_login TEXT
);
";

pub const VERSION: i64 = 2;

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
            "entities",
            "components_room",
            "components_exit",
            "components_position",
            "components_player",
            "components_npc",
            "components_health",
            "components_attributes",
            "components_level",
            "components_experience",
            "accounts",
            "attributes",
            "schema_version",
        ];
        for name in &expected {
            assert!(tables.iter().any(|t| t == name), "missing table: {name}");
        }
    }
}
