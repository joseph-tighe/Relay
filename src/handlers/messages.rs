use axum::{
    extract::{Path, State, Json},
    http::{header, StatusCode},
    response::IntoResponse,
};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::auth::{hash_and_salt_password, check_authenticity, check_password};
use crate::db::messages::{add_message, read_messages_by_subspace_id};
use crate::models::{NewMessage, Auth};

pub async fn get_messages_by_subspace_id(
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

pub async fn create_message(
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