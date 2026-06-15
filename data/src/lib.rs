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

        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}
