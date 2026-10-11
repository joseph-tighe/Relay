use rusqlite::{Connection};
use crate::db::schema::ensure_schema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open("example.db")?;

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
            owner_id INTEGER,
            id INTEGER PRIMARY KEY,
            name TEXT,
            image TEXT,
            FOREIGN KEY (owner_id) REFERENCES users(id)
        )",
        [],
    )?;
    db.execute("CREATE TABLE IF NOT EXISTS subspaces (
        id INTEGER PRIMARY KEY,
        name TEXT,
        parent_id INTEGER,
        club_id INTEGER,
        FOREIGN KEY (parent_id) REFERENCES subspaces(id),
        FOREIGN KEY (club_id) REFERENCES clubs(id)
    )", [])?;
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
    Ok(())
}