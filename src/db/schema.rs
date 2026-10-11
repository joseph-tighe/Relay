use rusqlite::Connection;
use crate::db::subspaces::make_new_subspace;
use crate::db::messages::add_message;
use crate::db::club::add_club;


pub fn ensure_schema(db: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT,
            email TEXT
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS clubs (
            id INTEGER PRIMARY KEY,
            name TEXT,
            image TEXT,
            owner_id INTEGER
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS subspaces (
            id INTEGER PRIMARY KEY,
            name TEXT,
            parent_id INTEGER,
            club_id INTEGER,
            FOREIGN KEY (parent_id) REFERENCES subspaces(id),
            FOREIGN KEY (club_id) REFERENCES clubs(id)
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS club_memberships (
            user_id INTEGER,
            club_id INTEGER,
            PRIMARY KEY (user_id, club_id),
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (club_id) REFERENCES clubs(id)
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY,
            user_id INTEGER,
            subspace_id INTEGER,
            content TEXT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (subspace_id) REFERENCES subspaces(id)
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS auth (
            user_id INTEGER PRIMARY KEY,
            passwordhash TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )",
        [],
    )?;
    // Migrate databases created before the current schema.
    ensure_column(db, "messages", "subspace_id", "subspace_id INTEGER")?;
    ensure_column(db, "clubs", "owner_id", "owner_id INTEGER")?;
    Ok(())
}
pub fn ensure_column(
    db: &Connection,
    table: &str,
    column: &str,
    ddl: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    db.execute(&format!("ALTER TABLE {table} ADD COLUMN {ddl}"), [])?;
    Ok(())
}
pub fn seed_if_empty(db: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let club_count: i32 =
        db.query_row("SELECT COUNT(*) FROM clubs", [], |row| row.get(0))?;
    if club_count == 0 {
        add_club(db, "Testing", "The image")?;
        let club_id = db.last_insert_rowid() as i32;
        let subspace_id = make_new_subspace(db, "Main", None, club_id)?;
        let user_exists: bool =
            db.query_row("SELECT EXISTS(SELECT 1 FROM users WHERE id = 1)", [], |row| {
                row.get(0)
            })?;
        if user_exists {
            add_message(db, 1, subspace_id, "test")?;
        }
    }
    Ok(())
}