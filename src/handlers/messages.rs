use axum::{
    extract::{Path, State, Json},
    http::{header, StatusCode},
    response::IntoResponse,
};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::auth::{check_authenticity, cached_verify, get_password_hash, needs_rehash, rehash_password};
use crate::db::messages::{add_message, read_messages_by_subspace_id};
use crate::models::{NewMessage, Auth};
use crate::db::club::get_club_members;

pub async fn get_messages_by_subspace_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Fetch hash under lock, verify OFF lock so slow Argon never blocks others.
    let stored = {
        let db = db.lock().await;
        get_password_hash(&db, auth.user_id)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    };

    let Some(stored) = stored else {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_string()));
    };
    if !cached_verify(auth.user_id, &auth.passwordhash, &stored) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_string()));
    }
    if needs_rehash(&stored) {
        let db = db.lock().await;
        let _ = rehash_password(&db, auth.user_id, &auth.passwordhash);
    }
    let db = db.lock().await;

    // Check membership
    let members = get_club_members(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    if !members.contains(&auth.user_id) {
        return Err((StatusCode::UNAUTHORIZED, "Not a member".to_string()));
    }

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
    let stored = {
        let db = db.lock().await;
        get_password_hash(&db, message.user_id)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    };
    let Some(stored) = stored else {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_string()));
    };
    if !cached_verify(message.user_id, &message.passwordhash, &stored) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_string()));
    }
    if needs_rehash(&stored) {
        let db = db.lock().await;
        let _ = rehash_password(&db, message.user_id, &message.passwordhash);
    }
    check_authenticity(&message.text).then(|| ()).ok_or((
        StatusCode::BAD_REQUEST,
        "Invalid message content".to_string(),
    ))?;
    let db = db.lock().await;
    let members = get_club_members(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    if !members.contains(&message.user_id) {
        return Err((StatusCode::UNAUTHORIZED, "Not a member".to_string()));
    }
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