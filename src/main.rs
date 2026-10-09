use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::get,
    routing::post,
    Json
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use rusqlite::Connection;

fn add_user(db: &Connection, name: &str, email: &str) -> Result<(), Box<dyn std::error::Error>> {
    db.execute("INSERT INTO users (name, email) VALUES (?1, ?2)", [name, email],)?;
    Ok(())
}
fn add_club(db: &Connection, name: &str, image: &str) -> Result<(), Box<dyn std::error::Error>> {
    db.execute("INSERT INTO clubs (name, image) VALUES (?1, ?2)", [name, image],)?;
    Ok(())
}
fn add_user_to_club(db: &Connection, user_id: i32, club_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    db.execute("INSERT INTO club_memberships (user_id, club_id) VALUES (?1, ?2)", [user_id, club_id],)?;
    Ok(())
}
fn get_user_by_id(db: &Connection, id: i32) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT name, email FROM users WHERE id = ?")?;
    let user = stmt.query_row([id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(user)
}
fn get_club_by_id(db: &Connection, id: i32) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT name, image FROM clubs WHERE id = ?1")?;
    let club = stmt.query_row([id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(club)
}
fn get_clubs_by_user_id(db: &Connection, user_id: i32) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT club_id FROM club_memberships WHERE user_id = ?1")?;
    let clubs_ids_a = stmt.query_map([user_id], |row| row.get::<_, i32>(0))?;
    let mut clubs_ids = Vec::new();
    for club_id in clubs_ids_a {
        clubs_ids.push(club_id?);
    }
    Ok(clubs_ids)
}
fn add_message(db: &Connection, user_id: i32, club_id: i32, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO messages (user_id, club_id, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![user_id, club_id, content],
    )?;
    Ok(())
}
fn read_messages_by_club_id(db: &Connection, club_id: i32) -> Result<Vec<(String, i32, String)>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT content, user_id, timestamp FROM messages WHERE club_id = ?1")?;
    let messages = stmt.query_map([club_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?, row.get::<_, String>(2)?))
    })?;
    let mut messages_vec = Vec::new();
    for message in messages {
        messages_vec.push(message?);
    }
    Ok(messages_vec)
}
async fn get_messages_by_club_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    let messages = read_messages_by_club_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let mut messages_json = Vec::new();
    for message in messages {
        messages_json.push(format!("{{\"text\": \"{}\", \"userId\": {}, \"timestamp\": \"{}\"}}", message.0, message.1, message.2));
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
struct Message {
    text: String,
    user_id: i32,
}
async fn create_message(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(message): Json<Message>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    check_authenticity(&message.text)
        .then(|| ())
        .ok_or((StatusCode::BAD_REQUEST, "Invalid message content".to_string()))?;
    let db = db.lock().await;
    add_message(&db, message.user_id, id, &message.text)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let message = read_messages_by_club_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let json = format!("{{\"text\": \"{}\", \"userId\": {}, \"timestamp\": \"{}\"}}", message[message.len()-1].0, message[message.len()-1].1, message[message.len()-1].2);
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open("example.db")?;
    let _ = add_user(&db, "The handsome one", "Joseph");
    let _ = add_club(&db, "The club", "The image");
    let _ = add_user_to_club(&db, 1, 1);
    let _ = add_message(&db, 1, 1, "Hello");
    let (user_name, user_email) = get_user_by_id(&db, 1)?;
    println!("{user_name}\n{user_email}");

    let db = Arc::new(Mutex::new(db));
    let app = Router::new()
        .route("/api/user/{id}", get(get_user_json_by_id))
        .route("/api/clubs-in/{id}", get(get_clubs_in_by_user_id))
        .route("/api/club/{id}", get(get_club_json_by_id))
        .route("/api/club/{id}/messages", get(get_messages_by_club_id))
        .route("/api/club/{id}/messages/new", post(create_message))
        .with_state(db)
        .fallback_service(ServeDir::new("static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
