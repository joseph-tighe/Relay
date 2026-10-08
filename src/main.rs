use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use axum::extract::Path;
use gluesql_core::prelude::Glue;
use gluesql_memory_storage::MemoryStorage;

async fn get_club(Path(club): Path<String>) -> String {
    format!("Robotics data")
}
async fn get_user_clubs(Path(club): Path<String>) -> String {
    format!("Robotics")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = MemoryStorage::default();
    let mut glue = Glue::new(storage);

    glue.execute(
        "CREATE TABLE clubs (id INTEGER, name TEXT, photo_url TEXT)"
    )?;

    glue.execute(
        "INSERT INTO clubs VALUES
         (100, 'Art Club', 'null'),
         (200, 'Robotics Club', 'https://external-content.duckduckgo.com/iu/?u=https%3A%2F%2Ftse2.mm.bing.net%2Fth%2Fid%2FOIP.kUWd0lZyt1RufYbfR1prLwHaE8%3Fr%3D0%26pid%3DApi&f=1&ipt=3b559678de45563686890052caf1d66df4c01c1f17cae1142ecd4eca5c924bba&ipo=images')"
    )?;
    println!(
        "{:?}",
        glue.execute("SELECT name, photo_url FROM clubs WHERE id = 200")?
    );
    let app = Router::new()
        .route("/api/get_club/{club}", axum::routing::get(get_club))
        .route("/api/get_user_clubs/{club}", axum::routing::get(get_user_clubs))
        .fallback_service(ServeDir::new("static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on http://{addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
