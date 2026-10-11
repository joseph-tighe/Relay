use rusqlite::Connection;

use crate::auth::{check_password, hash_and_salt_password};
use crate::db::club::make_new_club;
use crate::db::messages::{add_message, read_messages_by_subspace_id};
use crate::db::schema::ensure_schema;
use crate::db::subspaces::{get_subspaces_by_club_id, make_new_subspace};
use crate::db::user::{add_user, get_clubs_by_user_id};

fn memory_db() -> Connection {
    let db = Connection::open_in_memory().expect("in-memory db");
    ensure_schema(&db).expect("schema");
    db
}

#[test]
fn subspace_message_flow() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = memory_db();
    add_user(&mut db, "T", "t@e.c", "hash")?;
    let club_id = make_new_club(&db, "C", "img", 1)?;
    let subspace_id = make_new_subspace(&db, "S", None, club_id)?;
    add_message(&db, 1, subspace_id, "hi")?;
    let messages = read_messages_by_subspace_id(&db, subspace_id)?;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].1, "hi");
    let subspaces = get_subspaces_by_club_id(&db, club_id)?;
    assert_eq!(subspaces.len(), 1);
    assert_eq!(subspaces[0].1, "S");
    assert_eq!(subspaces[0].2, None);
    let child = make_new_subspace(&db, "Kid", Some(subspace_id), club_id)?;
    let subspaces = get_subspaces_by_club_id(&db, club_id)?;
    assert_eq!(subspaces.len(), 2);
    assert!(subspaces.iter().any(|s| s.0 == child && s.2 == Some(subspace_id)));
    // owner adds a second user by email; duplicates are ignored
    add_user(&mut db, "U", "u@e.c", "hash")?;
    let new_id: i32 = db.query_row("SELECT id FROM users WHERE email = 'u@e.c'", [], |row| {
        row.get(0)
    })?;
    db.execute(
        "INSERT OR IGNORE INTO club_memberships (user_id, club_id) VALUES (?1, ?2)",
        rusqlite::params![new_id, club_id],
    )?;
    db.execute(
        "INSERT OR IGNORE INTO club_memberships (user_id, club_id) VALUES (?1, ?2)",
        rusqlite::params![new_id, club_id],
    )?;
    let members: Vec<i32> = get_clubs_by_user_id(&db, new_id)?;
    assert_eq!(members, vec![club_id]);
    Ok(())
}

#[test]
fn migrates_legacy_tables() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open_in_memory()?;
    db.execute(
        "CREATE TABLE clubs (id INTEGER PRIMARY KEY, name TEXT, image TEXT)",
        [],
    )?;
    db.execute(
        "CREATE TABLE messages (id INTEGER PRIMARY KEY, user_id INTEGER, content TEXT, timestamp DATETIME DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;
    ensure_schema(&db)?;
    db.execute(
        "INSERT INTO clubs (name, image, owner_id) VALUES ('C', 'i', 1)",
        [],
    )?;
    db.execute(
        "INSERT INTO messages (user_id, subspace_id, content) VALUES (1, 1, 'hi')",
        [],
    )?;
    Ok(())
}

#[test]
fn salted_password_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Connection::open_in_memory()?;
    ensure_schema(&db)?;
    // signup stores hash_and_salt_password(client_hash, user_id)
    add_user(&mut db, "T", "t@e.c", "12345")?;
    // login + authed endpoints must salt the presented value the same way
    let salted = hash_and_salt_password("12345", "1");
    assert!(check_password(1, &salted, &db).is_ok());
    assert!(check_password(1, "12345", &db).is_err());
    assert!(check_password(1, "wrong", &db).is_err());
    // salting is deterministic within a build
    assert_eq!(salted, hash_and_salt_password("12345", "1"));
    assert_ne!(salted, hash_and_salt_password("12345", "2"));
    Ok(())
}
