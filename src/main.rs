use rusqlite::{Connection};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
mod app;
mod auth;
mod models;
mod db;
mod handers;
#[cfg(test)]
mod tests;

pub type AppState = Arc<Mutex<Connection>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open("example.db")?;
    crate::db::schema::ensure_schema(&db)?;
    crate::db::schema::seed_if_empty(&db)?;
    let db = Arc::new(Mutex::new(db));
    let app = crate::app::create_app(db);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}