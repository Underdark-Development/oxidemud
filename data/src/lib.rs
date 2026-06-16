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

        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}
