use rusqlite::{Connection};

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
            id INTEGER PRIMARY KEY,
            name TEXT,
            image TEXT
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
            club_id INTEGER,
            content TEXT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,

            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (club_id) REFERENCES clubs(id)
        )",
        [],
    )?;
    Ok(())
}