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
}
