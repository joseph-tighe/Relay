use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::auth::{check_authenticity, cached_verify, get_password_hash, needs_rehash, rehash_password};
use crate::db::messages::{add_message, read_messages_by_subspace_id};
use crate::models::{NewMessage, Auth};
use crate::db::club::is_club_member;
use crate::db::subspaces::get_club_id_by_subspace;

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

    // Resolve the owning club: subspace IDs and club IDs share one integer
    // namespace, so never use a subspace ID as a club ID.
    let club_id = get_club_id_by_subspace(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let Some(club_id) = club_id else {
        return Err((StatusCode::NOT_FOUND, "No such subspace".to_string()));
    };
    let member = is_club_member(&db, auth.user_id, club_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    if !member {
        return Err((StatusCode::UNAUTHORIZED, "Not a member".to_string()));
    }

    let messages = read_messages_by_subspace_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let messages_json: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|message| {
            serde_json::json!({
                "id": message.0,
                "text": message.1,
                "userId": message.2,
                "timestamp": message.3,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "messages": messages_json })))
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
    let club_id = get_club_id_by_subspace(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let Some(club_id) = club_id else {
        return Err((StatusCode::NOT_FOUND, "No such subspace".to_string()));
    };
    let member = is_club_member(&db, message.user_id, club_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    if !member {
        return Err((StatusCode::UNAUTHORIZED, "Not a member".to_string()));
    }
    add_message(&db, message.user_id, id, &message.text)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let messages = read_messages_by_subspace_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let Some(last) = messages.last() else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Message not found after insert".to_string(),
        ));
    };
    Ok(Json(serde_json::json!({
        "id": last.0,
        "text": last.1,
        "userId": last.2,
        "timestamp": last.3,
    })))
}
