mod password;
mod queries;
mod schema;

pub use password::*;
pub use queries::*;
pub use schema::*;

use rusqlite::Connection;
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        let mut db = Database { conn };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        let mut db = Database { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&mut self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(schema::SCHEMA)?;

        // Check current schema version and run incremental migrations
        let current: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current < 6 {
            // Migration 6: add spawn_key column to characters
            // Guard against double-apply (column already exists in new DBs)
            let has_col: bool = self
                .conn
                .prepare("SELECT spawn_key FROM characters LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn
                    .execute_batch("ALTER TABLE characters ADD COLUMN spawn_key TEXT;")?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (6)",
                [],
            )?;
        }

        if current < 7 {
            // Migration 7: expand components_golds with silver, gold, platinum
            let has_silver: bool = self
                .conn
                .prepare("SELECT silver FROM components_golds LIMIT 0")
                .is_ok();
            if !has_silver {
                self.conn.execute_batch(
                    "ALTER TABLE components_golds ADD COLUMN silver INTEGER NOT NULL DEFAULT 0;
                     ALTER TABLE components_golds ADD COLUMN gold INTEGER NOT NULL DEFAULT 0;
                     ALTER TABLE components_golds ADD COLUMN platinum INTEGER NOT NULL DEFAULT 0;",
                )?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (7)",
                [],
            )?;
        }

        if current < 8 {
            // Migration 8: components_skills table
            // Table is created by CREATE TABLE IF NOT EXISTS in SCHEMA,
            // so running the full schema batch handles it.
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (8)",
                [],
            )?;
        }

        if current < 9 {
            // Migration 9: add screen_width column to components_player
            let has_col: bool = self
                .conn
                .prepare("SELECT screen_width FROM components_player LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn.execute_batch(
                    "ALTER TABLE components_player ADD COLUMN screen_width INTEGER NOT NULL DEFAULT 80;",
                )?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (9)",
                [],
            )?;
        }

        if current < 10 {
            // Migration 10: add gender columns to characters table
            let has_col: bool = self
                .conn
                .prepare("SELECT gender FROM characters LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn.execute_batch(
                    "ALTER TABLE characters ADD COLUMN gender TEXT NOT NULL DEFAULT 'neutral';
                     ALTER TABLE characters ADD COLUMN pronoun_subject TEXT NOT NULL DEFAULT 'they';
                     ALTER TABLE characters ADD COLUMN pronoun_object TEXT NOT NULL DEFAULT 'them';
                     ALTER TABLE characters ADD COLUMN pronoun_possessive TEXT NOT NULL DEFAULT 'their';",
                )?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (10)",
                [],
            )?;
        }

        if current < 11 {
            // Migration 11: add unspent_skill_points to components_player
            let has_col: bool = self
                .conn
                .prepare("SELECT unspent_skill_points FROM components_player LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn.execute_batch(
                    "ALTER TABLE components_player ADD COLUMN unspent_skill_points INTEGER NOT NULL DEFAULT 0;",
                )?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (11)",
                [],
            )?;
        }

        if current < 12 {
            // Migration 12: add spawn_key to components_room
            let has_col: bool = self
                .conn
                .prepare("SELECT spawn_key FROM components_room LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn
                    .execute_batch("ALTER TABLE components_room ADD COLUMN spawn_key TEXT;")?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (12)",
                [],
            )?;
        }

        if current < 13 {
            // Migration 13: drop NOT NULL + DEFAULT from components_player.prompt
            // so that prompt can be NULL ("use server config default").
            // NOTE: guard uses column index (3 = notnull) to avoid keyword conflict
            // in SQLite 3.51+.
            let prompt_not_null: bool = self
                .conn
                .prepare(
                    "SELECT * FROM pragma_table_info('components_player') \
                     WHERE name = 'prompt'",
                )
                .and_then(|mut s| s.query_row([], |row| row.get::<_, i64>(3)))
                .map(|v| v != 0)
                .unwrap_or(false);
            if prompt_not_null {
                recreate_player_table(&self.conn)?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (13)",
                [],
            )?;
        }

        if current < 14 {
            // Migration 14: redo migration 13 for databases where the guard
            // was broken (notnull keyword not quoted).
            let prompt_not_null: bool = self
                .conn
                .prepare(
                    "SELECT * FROM pragma_table_info('components_player') \
                     WHERE name = 'prompt'",
                )
                .and_then(|mut s| s.query_row([], |row| row.get::<_, i64>(3)))
                .map(|v| v != 0)
                .unwrap_or(false);
            if prompt_not_null {
                recreate_player_table(&self.conn)?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (14)",
                [],
            )?;
        }

        if current < 15 {
            // Migration 15: components_practice_points table and retro calculation
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS components_practice_points (
                    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
                    points INTEGER NOT NULL DEFAULT 0
                );",
            )?;

            // Check if the legacy unspent_skill_points column exists in components_player
            let has_unspent: bool = self
                .conn
                .prepare("SELECT unspent_skill_points FROM components_player LIMIT 0")
                .is_ok();

            let query = if has_unspent {
                "SELECT 
                    p.entity_id, 
                    COALESCE(l.level, 1) as level, 
                    COALESCE(a.wisdom, 10) as wisdom, 
                    COALESCE(a.intelligence, 10) as intelligence, 
                    COALESCE(p.unspent_skill_points, 0) as unspent 
                 FROM components_player p
                 LEFT JOIN components_level l ON p.entity_id = l.entity_id
                 LEFT JOIN components_attributes a ON p.entity_id = a.entity_id;"
            } else {
                "SELECT 
                    p.entity_id, 
                    COALESCE(l.level, 1) as level, 
                    COALESCE(a.wisdom, 10) as wisdom, 
                    COALESCE(a.intelligence, 10) as intelligence, 
                    0 as unspent 
                 FROM components_player p
                 LEFT JOIN components_level l ON p.entity_id = l.entity_id
                 LEFT JOIN components_attributes a ON p.entity_id = a.entity_id;"
            };

            let mut stmt = self.conn.prepare(query)?;
            let mut rows = stmt.query([])?;

            let mut updates = Vec::new();
            while let Some(row) = rows.next()? {
                let entity_id: i64 = row.get(0)?;
                let level: i64 = row.get(1)?;
                let wisdom: i64 = row.get(2)?;
                let intelligence: i64 = row.get(3)?;
                let unspent: i64 = row.get(4)?;

                let wis_mod = (wisdom - 10) / 2;
                let int_mod = (intelligence - 10) / 2;
                let gain_per_level = (2 + wis_mod + int_mod).max(1);
                let retro_points = level * gain_per_level + unspent;
                updates.push((entity_id, retro_points));
            }
            drop(rows);
            drop(stmt);

            let tx = self.conn.transaction()?;
            {
                let mut insert_stmt = tx.prepare(
                    "INSERT INTO components_practice_points (entity_id, points) VALUES (?1, ?2)",
                )?;
                for (entity_id, points) in updates {
                    insert_stmt.execute(rusqlite::params![entity_id, points])?;
                }
            }
            tx.commit()?;

            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (15)",
                [],
            )?;
        }

        if current < 16 {
            // Migration 16: components_appearance, components_age, components_deity tables
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS components_appearance (
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
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (16)",
                [],
            )?;
        }

        if current < 17 {
            // Migration 17: add recall_room_id column to characters
            let has_col: bool = self
                .conn
                .prepare("SELECT recall_room_id FROM characters LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn.execute_batch(
                    "ALTER TABLE characters ADD COLUMN recall_room_id INTEGER REFERENCES entities(id);",
                )?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (17)",
                [],
            )?;
        }

        if current < 18 {
            // Migration 18: add current_room_key column to characters
            let has_col: bool = self
                .conn
                .prepare("SELECT current_room_key FROM characters LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn
                    .execute_batch("ALTER TABLE characters ADD COLUMN current_room_key TEXT;")?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (18)",
                [],
            )?;
        }

        if current < 19 {
            // Migration 19: add recall_room_key column to characters
            let has_col: bool = self
                .conn
                .prepare("SELECT recall_room_key FROM characters LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn
                    .execute_batch("ALTER TABLE characters ADD COLUMN recall_room_key TEXT;")?;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (19)",
                [],
            )?;
        }

        if current < 20 {
            // Migration 20: add api_keys table
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS api_keys (
                    key TEXT PRIMARY KEY NOT NULL,
                    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                    description TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (20)",
                [],
            )?;
        }

        if current < 21 {
            // Migration 21: add components_quest_log table
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS components_quest_log (
                    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
                    log_json TEXT NOT NULL
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (21)",
                [],
            )?;
        }

        if current < 22 {
            // Migration 22: add components_faction_standing table
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS components_faction_standing (
                    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
                    standing_json TEXT NOT NULL
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (22)",
                [],
            )?;
        }

        if current < 23 {
            // Migration 23: add components_learned_recipes table
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS components_learned_recipes (
                    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
                    recipes_json TEXT NOT NULL
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (23)",
                [],
            )?;
        }

        if current < 24 {
            // Migration 24: add components_multiclass table
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS components_multiclass (
                    entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
                    multiclass_json TEXT NOT NULL
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (24)",
                [],
            )?;
        }

        if current < 25 {
            // Migration 25: add expires_at to api_keys + api_key_scopes table
            // Guard: only add column if it doesn't exist (fresh DBs using SCHEMA already have it)
            let has_col: bool = self
                .conn
                .prepare("SELECT expires_at FROM api_keys LIMIT 0")
                .is_ok();
            if !has_col {
                self.conn
                    .execute_batch("ALTER TABLE api_keys ADD COLUMN expires_at TEXT;")?;
            }
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS api_key_scopes (
                    key TEXT NOT NULL REFERENCES api_keys(key) ON DELETE CASCADE,
                    scope TEXT NOT NULL CHECK(scope IN ('mcp', 'spade')),
                    PRIMARY KEY (key, scope)
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (25)",
                [],
            )?;
        }

        if current < 26 {
            // Migration 26: add world_time table
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS world_time (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    hour INTEGER NOT NULL DEFAULT 6,
                    minute INTEGER NOT NULL DEFAULT 0,
                    day INTEGER NOT NULL DEFAULT 1,
                    season TEXT NOT NULL DEFAULT 'spring',
                    year INTEGER NOT NULL DEFAULT 1
                );",
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (26)",
                [],
            )?;
        }

        Ok(())
    }

    pub fn run_backup(&self, backup_dir: impl AsRef<Path>) -> Result<(), rusqlite::Error> {
        if let Err(_e) = std::fs::create_dir_all(&backup_dir) {
            return Err(rusqlite::Error::InvalidPath(
                backup_dir.as_ref().to_path_buf(),
            ));
        }

        let now = chrono::Local::now();
        let filename = format!("backup_{}.db", now.format("%Y%m%d_%H%M%S"));
        let backup_path = backup_dir.as_ref().join(filename);

        let mut dest_conn = Connection::open(&backup_path)?;
        dest_conn.execute_batch("PRAGMA journal_mode = WAL;")?;

        let backup = rusqlite::backup::Backup::new(&self.conn, &mut dest_conn)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(50), None)?;

        self.prune_backups(&backup_dir)?;

        Ok(())
    }

    pub fn prune_backups(&self, backup_dir: impl AsRef<Path>) -> Result<(), rusqlite::Error> {
        use chrono::Datelike;

        let entries = std::fs::read_dir(&backup_dir)
            .map_err(|_| rusqlite::Error::InvalidPath(backup_dir.as_ref().to_path_buf()))?;

        struct BackupFile {
            path: std::path::PathBuf,
            date: chrono::NaiveDate,
            filename: String,
        }

        let mut backups = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let filename = path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .map(|s| s.to_string());
                if let Some(filename) = filename {
                    if filename.starts_with("backup_")
                        && filename.ends_with(".db")
                        && filename.len() == 25
                    {
                        let date_part = &filename[7..15];
                        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y%m%d") {
                            backups.push(BackupFile {
                                path,
                                date,
                                filename,
                            });
                        }
                    }
                }
            }
        }

        backups.sort_by_key(|b| b.date);

        let mut daily_groups = std::collections::HashMap::new();
        for b in &backups {
            daily_groups.insert(b.date, b.path.clone());
        }

        let mut daily_dates: Vec<chrono::NaiveDate> = daily_groups.keys().copied().collect();
        daily_dates.sort();
        let daily_to_keep: std::collections::HashSet<std::path::PathBuf> = daily_dates
            .iter()
            .rev()
            .take(7)
            .map(|d| daily_groups.get(d).unwrap().clone())
            .collect();

        let mut weekly_groups = std::collections::HashMap::new();
        for b in &backups {
            let week = b.date.iso_week();
            let key = (week.year(), week.week());
            weekly_groups.insert(key, b.path.clone());
        }

        let mut weekly_keys: Vec<(i32, u32)> = weekly_groups.keys().copied().collect();
        weekly_keys.sort();
        let weekly_to_keep: std::collections::HashSet<std::path::PathBuf> = weekly_keys
            .iter()
            .rev()
            .take(4)
            .map(|k| weekly_groups.get(k).unwrap().clone())
            .collect();

        for b in &backups {
            if !daily_to_keep.contains(&b.path) && !weekly_to_keep.contains(&b.path) {
                if let Err(e) = std::fs::remove_file(&b.path) {
                    tracing::error!("Failed to delete old backup {}: {}", b.filename, e);
                } else {
                    tracing::info!("Pruned old backup: {}", b.filename);
                }
            }
        }

        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn save_world_time(
        &self,
        hour: u8,
        minute: u8,
        day: u32,
        season: &str,
        year: u32,
    ) -> Result<(), rusqlite::Error> {
        queries::save_world_time(&self.conn, hour, minute, day, season, year)
    }

    pub fn load_world_time(&self) -> Result<Option<WorldTimeRecord>, rusqlite::Error> {
        queries::load_world_time(&self.conn)
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

fn recreate_player_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "PRAGMA defer_foreign_keys = ON;
         CREATE TABLE components_player_v2 (
             entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
             account_id INTEGER NOT NULL,
             prompt TEXT,
             screen_width INTEGER NOT NULL DEFAULT 80,
             unspent_skill_points INTEGER NOT NULL DEFAULT 0
         );
         INSERT INTO components_player_v2
             (entity_id, account_id, prompt, screen_width, unspent_skill_points)
             SELECT entity_id, account_id, prompt, screen_width, unspent_skill_points
             FROM components_player;
         DROP TABLE components_player;
         ALTER TABLE components_player_v2 RENAME TO components_player;
         PRAGMA defer_foreign_keys = OFF;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_15() {
        // 1. Setup a version 14 database structure
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .unwrap();

        // Execute the SCHEMA. Note that schema::SCHEMA has version 15 (no unspent_skill_points, components_practice_points exists).
        conn.execute_batch(crate::schema::SCHEMA).unwrap();

        // Let's drop components_practice_points to simulate version 14.
        conn.execute_batch("DROP TABLE IF EXISTS components_practice_points;")
            .unwrap();

        // Re-create components_player with the unspent_skill_points column to match version 14.
        conn.execute_batch(
            "
            DROP TABLE IF EXISTS components_player;
            CREATE TABLE components_player (
                entity_id INTEGER PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
                account_id INTEGER NOT NULL,
                prompt TEXT,
                screen_width INTEGER NOT NULL DEFAULT 80,
                unspent_skill_points INTEGER NOT NULL DEFAULT 0
            );
        ",
        )
        .unwrap();

        // Delete version 15 from schema_version.
        conn.execute_batch("DELETE FROM schema_version WHERE version = 15;")
            .unwrap();

        // 2. Insert test data
        let eid = queries::insert_entity(&conn, "player").unwrap();

        // Player components
        conn.execute(
            "INSERT INTO components_player (entity_id, account_id, prompt, screen_width, unspent_skill_points) VALUES (?1, 42, NULL, 80, 5)",
            rusqlite::params![eid],
        ).unwrap();

        // Level component (level = 5)
        queries::save_level_component(&conn, eid, 5).unwrap();

        // Attributes component (wisdom = 14, intelligence = 12)
        queries::save_attributes_component(
            &conn,
            eid,
            &queries::AttributesRow {
                strength: 10,
                dexterity: 10,
                intelligence: 12,
                wisdom: 14,
                constitution: 10,
                charisma: 10,
            },
        )
        .unwrap();

        // 3. Wrap in Database struct and run migrations (which will run Migration 15)
        let mut db = Database { conn };
        db.run_migrations().unwrap();

        // 4. Verify results
        // wis_mod = (14 - 10) / 2 = 2
        // int_mod = (12 - 10) / 2 = 1
        // gain_per_level = max(1, 2 + 2 + 1) = 5
        // expected_points = 5 (level) * 5 (gain) + 5 (unspent) = 30
        let points = queries::load_practice_points(&db.conn, eid)
            .unwrap()
            .unwrap();
        assert_eq!(points, 30);
    }

    #[test]
    fn test_run_migrations_new_db() {
        let db = Database::open_in_memory().unwrap();
        let version: i64 = db
            .conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, schema::VERSION);
    }

    #[test]
    fn test_database_backup_and_pruning() {
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("temp_backups_test");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let db = Database::open_in_memory().unwrap();

        // 1. Verify hot backup can run to completion
        db.run_backup(&temp_dir).unwrap();

        // Count files created by run_backup
        let backup_files_count = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap().starts_with("backup_"))
            .count();
        assert!(backup_files_count >= 1);

        // Clear the directory to start with a clean slate for pruning test
        for entry in std::fs::read_dir(&temp_dir).unwrap() {
            let entry = entry.unwrap();
            std::fs::remove_file(entry.path()).unwrap();
        }

        // 2. Generate dummy backup files to verify daily/weekly retention
        let base_date = chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        for offset in 0..15 {
            let date = base_date + chrono::Duration::days(offset);
            let filename = format!("backup_{}_120000.db", date.format("%Y%m%d"));
            let file_path = temp_dir.join(&filename);
            std::fs::write(&file_path, b"dummy").unwrap();
        }

        // Run pruning
        db.prune_backups(&temp_dir).unwrap();

        let mut remaining: Vec<String> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        remaining.sort();

        let expected = vec![
            "backup_20260705_120000.db",
            "backup_20260709_120000.db",
            "backup_20260710_120000.db",
            "backup_20260711_120000.db",
            "backup_20260712_120000.db",
            "backup_20260713_120000.db",
            "backup_20260714_120000.db",
            "backup_20260715_120000.db",
        ];

        let test_dummies: Vec<String> = remaining
            .into_iter()
            .filter(|f| f.contains("120000"))
            .collect();

        assert_eq!(test_dummies, expected);

        // Clean up
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
