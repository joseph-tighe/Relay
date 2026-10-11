use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::get,
    routing::post,
};
use rusqlite::{Connection, OptionalExtension};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
fn hash_and_salt_password(password: &str, salt: &str) -> String {
    let mut password = password.to_owned();
    password.push_str(salt);
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
fn add_user(
    db: &mut Connection,
    name: &str,
    email: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let transaction = db.transaction()?;
    transaction.execute(
        "INSERT INTO users (name, email) VALUES (?1, ?2)",
        [name, email],
    )?;
    let user_id = transaction.last_insert_rowid();
    transaction.execute(
        "INSERT INTO auth (user_id, passwordhash) VALUES (?1, ?2)",
        rusqlite::params![user_id, hash_and_salt_password(password, &user_id.to_string())],
    )?;
    transaction.commit()?;
    Ok(())
}
fn add_club(db: &Connection, name: &str, image: &str) -> Result<(), Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO clubs (name, image) VALUES (?1, ?2)",
        [name, image],
    )?;
    Ok(())
}
fn add_user_to_club(
    db: &Connection,
    user_id: i32,
    club_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO club_memberships (user_id, club_id) VALUES (?1, ?2)",
        [user_id, club_id],
    )?;
    Ok(())
}
fn get_user_by_id(
    db: &Connection,
    id: i32,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT name, email FROM users WHERE id = ?")?;
    let user = stmt.query_row([id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(user)
}
fn get_club_by_id(
    db: &Connection,
    id: i32,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT name, image FROM clubs WHERE id = ?1")?;
    let club = stmt.query_row([id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(club)
}
fn get_clubs_by_user_id(
    db: &Connection,
    user_id: i32,
) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT club_id FROM club_memberships WHERE user_id = ?1")?;
    let clubs_ids_a = stmt.query_map([user_id], |row| row.get::<_, i32>(0))?;
    let mut clubs_ids = Vec::new();
    for club_id in clubs_ids_a {
        clubs_ids.push(club_id?);
    }
    Ok(clubs_ids)
}
fn add_message(
    db: &Connection,
    user_id: i32,
    subspace_id: i32,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO messages (user_id, subspace_id, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![user_id, subspace_id, content],
    )?;
    Ok(())
}
fn read_messages_by_subspace_id(
    db: &Connection,
    subspace_id: i32,
) -> Result<Vec<(i32, String, i32, String)>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare(
        "SELECT id, content, user_id, timestamp FROM messages WHERE subspace_id = ?1 ORDER BY id ASC",
    )?;
    let messages = stmt.query_map([subspace_id], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut messages_vec = Vec::new();
    for message in messages {
        messages_vec.push(message?);
    }
    Ok(messages_vec)
}
#[derive(serde::Deserialize)]
struct Auth {
    user_id: i32,
    passwordhash: String,
}
async fn get_messages_by_subspace_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(auth.user_id, &hash_and_salt_password(&auth.passwordhash, &auth.user_id.to_string()), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    let messages = read_messages_by_subspace_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let mut messages_json = Vec::new();
    for message in messages {
        messages_json.push(format!(
            "{{\"id\": {}, \"text\": \"{}\", \"userId\": {}, \"timestamp\": \"{}\"}}",
            message.0, message.1, message.2, message.3
        ));
    }
    let json = format!("{{\"messages\": [{}]}}", messages_json.join(", "));
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
fn check_sql_injection(input: &str) -> bool {
    let forbidden_keywords = ["'", ";", "--"];
    let input_upper = input.to_uppercase();
    for keyword in forbidden_keywords.iter() {
        if input_upper.contains(keyword) {
            return true;
        }
    }
    false
}
fn check_xss(input: &str) -> bool {
    let forbidden_keywords = ["\"", "<", ">"];
    let input_upper = input.to_uppercase();
    for keyword in forbidden_keywords.iter() {
        if input_upper.contains(keyword) {
            return true;
        }
    }
    false
}
fn check_authenticity(input: &str) -> bool {
    if check_sql_injection(input) || check_xss(input) {
        return false;
    }
    true
}
fn check_password(
    id: i32,
    user_password_hash: &str,
    db: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT passwordhash FROM auth WHERE user_id = ?1")?;
    let password_hash = stmt.query_row([id], |row| Ok(row.get::<_, String>(0)?))?;
    if password_hash != user_password_hash {
        return Err("Invalid password".into());
    }
    Ok(())
}
async fn get_user_json_by_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    let (user_name, user_email) = get_user_by_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let json = format!("{{\"name\": \"{user_name}\", \"email\": \"{user_email}\"}}");
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
async fn get_club_json_by_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    let (club_name, club_image) = get_club_by_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let json = format!("{{\"name\": \"{club_name}\", \"image\": \"{club_image}\"}}");
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
fn get_subspaces_by_club_id(
    db: &Connection,
    club_id: i32,
) -> Result<Vec<(i32, String, Option<i32>)>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare(
        "SELECT id, name, parent_id FROM subspaces WHERE club_id = ?1 ORDER BY id ASC",
    )?;
    let subspaces = stmt.query_map([club_id], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
        ))
    })?;
    let mut subspaces_vec = Vec::new();
    for subspace in subspaces {
        subspaces_vec.push(subspace?);
    }
    Ok(subspaces_vec)
}
fn get_subspace_by_id(
    db: &Connection,
    id: i32,
) -> Result<(i32, String, Option<i32>, i32), Box<dyn std::error::Error>> {
    let mut stmt =
        db.prepare("SELECT id, name, parent_id, club_id FROM subspaces WHERE id = ?1")?;
    let subspace = stmt.query_row([id], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;
    Ok(subspace)
}
fn parent_id_json(parent_id: Option<i32>) -> String {
    match parent_id {
        Some(id) => id.to_string(),
        None => "null".to_string(),
    }
}
async fn get_subspaces_json_by_club_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(auth.user_id, &hash_and_salt_password(&auth.passwordhash, &auth.user_id.to_string()), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    let subspaces = get_subspaces_by_club_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let mut subspaces_json = Vec::new();
    for subspace in subspaces {
        subspaces_json.push(format!(
            "{{\"id\": {}, \"name\": \"{}\", \"parent_id\": {}}}",
            subspace.0,
            subspace.1,
            parent_id_json(subspace.2)
        ));
    }
    let json = format!("{{\"subspaces\": [{}]}}", subspaces_json.join(", "));
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
async fn get_subspace_json_by_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(auth.user_id, &hash_and_salt_password(&auth.passwordhash, &auth.user_id.to_string()), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    let subspace = get_subspace_by_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let json = format!(
        "{{\"id\": {}, \"name\": \"{}\", \"parent_id\": {}, \"club_id\": {}}}",
        subspace.0,
        subspace.1,
        parent_id_json(subspace.2),
        subspace.3
    );
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
async fn get_clubs_in_by_user_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    let clubs_ids = get_clubs_by_user_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let clubs = clubs_ids
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let json = format!("{{\"clubs\": [{clubs}]}}");
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
#[derive(serde::Deserialize)]
struct NewMessage {
    passwordhash: String,
    text: String,
    user_id: i32,
}
async fn create_message(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(message): Json<NewMessage>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(message.user_id, &hash_and_salt_password(&message.passwordhash, &message.user_id.to_string()), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    check_authenticity(&message.text).then(|| ()).ok_or((
        StatusCode::BAD_REQUEST,
        "Invalid message content".to_string(),
    ))?;
    add_message(&db, message.user_id, id, &message.text)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let message = read_messages_by_subspace_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let json = format!(
        "{{\"id\": {}, \"text\": \"{}\", \"userId\": {}, \"timestamp\": \"{}\"}}",
        message[message.len() - 1].0,
        message[message.len() - 1].1,
        message[message.len() - 1].2,
        message[message.len() - 1].3
    );
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
fn email_exists(db: &Connection, email: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let exists = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = ?1)",
        [email],
        |row| row.get(0),
    )?;
    Ok(exists)
}
#[derive(Debug, serde::Deserialize)]
struct UserPassword {
    name: String,
    email: String,
    password: String,
}
#[derive(Debug, serde::Deserialize)]
struct Login {
    email: String,
    password: String,
}
fn get_user_auth_by_email(
    db: &Connection,
    email: &str,
) -> Result<Option<(i32, String, String, String)>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare(
        "SELECT users.id, users.name, users.email, auth.passwordhash
         FROM users JOIN auth ON auth.user_id = users.id
         WHERE users.email = ?1",
    )?;
    let user = stmt
        .query_row([email], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .optional()?;
    Ok(user)
}
async fn sign_up(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(user): Json<UserPassword>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    check_authenticity(&user.name)
        .then(|| ())
        .ok_or((StatusCode::BAD_REQUEST, "Invalid user name".to_string()))?;

    let mut db = db.lock().await;
    if email_exists(&db, &user.email)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    {
        return Err((StatusCode::BAD_REQUEST, "Email already exists".to_string()));
    }
    add_user(&mut db, &user.name, &user.email, &user.password)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let user_id = db.last_insert_rowid();
    let _ = add_user_to_club(&mut db, user_id.to_string().parse::<i32>().unwrap(), 1);
    let json = format!(
        "{{\"name\": \"{}\", \"email\": \"{}\", \"id\": {}}}",
        user.name, user.email, user_id
    );
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
async fn login(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(user): Json<Login>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    if !email_exists(&db, &user.email)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    {
        return Err((StatusCode::BAD_REQUEST, "Email doesn't exists".to_string()));
    }
    let stored_user = get_user_auth_by_email(&db, &user.email)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let Some((user_id, user_name, user_email, password_hash)) = stored_user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        ));
    };
    if hash_and_salt_password(&user.password, &user_id.to_string()) != password_hash {
        return Err((StatusCode::BAD_REQUEST, "Invalid password".to_string()));
    }
    let json =
        format!("{{\"id\": {user_id}, \"name\": \"{user_name}\", \"email\": \"{user_email}\"}}");
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
fn add_subspace(
    db: &Connection,
    name: &str,
    parent_id: Option<i32>,
    club_id: i32,
) -> Result<i32, Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO subspaces (name, parent_id, club_id) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, parent_id, club_id],
    )?;
    Ok(db.last_insert_rowid() as i32)
}
fn ensure_schema(db: &Connection) -> Result<(), Box<dyn std::error::Error>> {
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
fn ensure_column(
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
fn seed_if_empty(db: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let club_count: i32 =
        db.query_row("SELECT COUNT(*) FROM clubs", [], |row| row.get(0))?;
    if club_count == 0 {
        add_club(db, "Testing", "The image")?;
        let club_id = db.last_insert_rowid() as i32;
        let subspace_id = add_subspace(db, "Main", None, club_id)?;
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
fn make_new_club(
    db: &Connection,
    name: &str,
    image: &str,
    owner_id: i32,
) -> Result<i32, Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO clubs (name, image, owner_id) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, image, owner_id],
    )?;
    Ok(db.last_insert_rowid() as i32)
}
fn make_new_subspace(
    db: &Connection,
    name: &str,
    parent_id: Option<i32>,
    club_id: i32,
) -> Result<i32, Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO subspaces (name, parent_id, club_id) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, parent_id, club_id],
    )?;
    Ok(db.last_insert_rowid() as i32)
}
#[derive(serde::Deserialize)]
struct NewClub {
    name: String,
    image: String,
    owner_id: i32,
    owner_password: String
}
async fn api_make_club(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(new_club): Json<NewClub>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(new_club.owner_id, &hash_and_salt_password(&new_club.owner_password, &new_club.owner_id.to_string()), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    let owner_id = new_club.owner_id;
    get_user_by_id(&db, owner_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let club_id = make_new_club(&db, &new_club.name, &new_club.image, owner_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let subspace_id = make_new_subspace(&db, "Main", None, club_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    add_message(
        &db,
        owner_id,
        subspace_id,
        &format!("Club {} created", new_club.name),
    )
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    add_user_to_club(&db, owner_id, club_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    println!("Created club {} with subspace {} for user {}", club_id, subspace_id, owner_id);
    Ok(Json(serde_json::json!({ "club_id": club_id, "subspace_id": subspace_id })))
}
#[derive(serde::Deserialize)]
struct NewSubspace {
    name: String,
    parent_id: Option<i32>,
    club_id: i32,
    owner_password: String
}
async fn api_make_subspace(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(new_subspace): Json<NewSubspace>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_authenticity(&new_subspace.owner_password)
        .then(|| ())
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid password".to_string()))?;
    let subspace_id = make_new_subspace(
        &db,
        &new_subspace.name,
        new_subspace.parent_id,
        new_subspace.club_id,
    )
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(serde_json::json!({ "subspace_id": subspace_id })))
}
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open("example.db")?;
    ensure_schema(&db)?;
    seed_if_empty(&db)?;
    let db = Arc::new(Mutex::new(db));
    let app = Router::new()
        .route("/api/user/{id}", get(get_user_json_by_id))
        .route("/api/clubs-in/{id}", get(get_clubs_in_by_user_id))
        .route("/api/club/{id}", get(get_club_json_by_id))
        .route("/api/club/new", post(api_make_club))
        .route("/api/subspace/new", post(api_make_subspace))
        .route("/api/club/{id}/subspaces", post(get_subspaces_json_by_club_id))
        .route("/api/subspace/{id}", post(get_subspace_json_by_id))
        .route("/api/subspace/{id}/messages", post(get_messages_by_subspace_id))
        .route("/api/subspace/{id}/messages/new", post(create_message))
        .route("/api/signup", post(sign_up))
        .route("/api/login", post(login))
        .with_state(db)
        .fallback_service(ServeDir::new("static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
