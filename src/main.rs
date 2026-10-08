use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use axum::extract::Path;
use rusqlite::{params, Connection};

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
fn get_user_by_id(db: &Connection, id: i32) -> Result<String, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT name FROM users WHERE id = ?")?;
    let user = stmt.query_row([id], |row| row.get::<_, String>(0))?;
    Ok(user)
}
fn get_club_by_id(db: &Connection, id: i32) -> Result<String, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT name FROM clubs WHERE id = ?")?;
    let club = stmt.query_row([id], |row| row.get::<_, String>(0))?;
    Ok(club)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open("example.db")?;
    let app = Router::new()
        .fallback_service(ServeDir::new("static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    let _ = add_user(&db, "The handsome one", "Joseph");
    println!("Listening on http://{addr}");
    let mut stmt = db.prepare("SELECT id, name, email FROM users WHERE 1=1")?;

let users = stmt.query_map([], |row| {
    Ok((
        row.get::<_, i32>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
    ))
})?;

for user in users {
    println!("{:?}", user?);
}

    axum::serve(listener, app).await?;

    Ok(())
}
