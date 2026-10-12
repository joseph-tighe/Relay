use rusqlite::{Connection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open("example.db")?;

    db.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS clubs (
            owner_id INTEGER,
            id INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            image TEXT,
            FOREIGN KEY (owner_id) REFERENCES users(id)
        )",
        [],
    )?;
    db.execute("CREATE TABLE IF NOT EXISTS subspaces (
        id INTEGER PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        parent_id INTEGER,
        club_id INTEGER NOT NULL,
        FOREIGN KEY (parent_id) REFERENCES subspaces(id),
        FOREIGN KEY (club_id) REFERENCES clubs(id)
    )", [])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS club_memberships (
            user_id INTEGER NOT NULL,
            club_id INTEGER NOT NULL,
            PRIMARY KEY (user_id, club_id),

            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (club_id) REFERENCES clubs(id)
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY NOT NULL,
            user_id INTEGER NOT NULL,
            subspace_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,

            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (subspace_id) REFERENCES subspaces(id)
        )",
        [],
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS auth (
            user_id INTEGER PRIMARY KEY NOT NULL,
            passwordhash TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )",
        [],
    )?;
    // Seed a welcome club so first-run isn't empty. Signup auto-joins new
    // users to club 1, which must exist. Idempotent: skipped when clubs exist.
    let club_count: i32 =
        db.query_row("SELECT COUNT(*) FROM clubs", [], |row| row.get(0))?;
    if club_count == 0 {
        db.execute(
            "INSERT INTO clubs (name, image) VALUES ('Testing', 'The image')",
            [],
        )?;
        let club_id = db.last_insert_rowid();
        db.execute(
            "INSERT INTO subspaces (name, parent_id, club_id) VALUES ('Main', NULL, ?1)",
            [club_id],
        )?;
    }
    Ok(())
}