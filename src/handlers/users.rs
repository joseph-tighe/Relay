use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
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
    Ok(Json(serde_json::json!({ "name": user_name, "email": user_email })))
}

pub async fn get_clubs_in_by_user_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    let clubs_ids = get_clubs_by_user_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(serde_json::json!({ "clubs": clubs_ids })))
}
