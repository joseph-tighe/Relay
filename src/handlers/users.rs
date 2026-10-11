use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use rusqlite::Connection;
use crate::db::user::{get_user_by_id, get_clubs_by_user_id};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn get_user_json_by_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    let (user_name, user_email) = get_user_by_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let json = format!("{{\"name\": \"{user_name}\", \"email\": \"{user_email}\"}}");
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}

pub async fn get_clubs_in_by_user_id(
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