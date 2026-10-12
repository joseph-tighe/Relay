use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{auth::{cached_verify, needs_rehash, rehash_password}, db::user::{add_user, email_exists, get_user_auth_by_email}};
use crate::db::club::add_user_to_club;
use crate::models::{UserPassword, Login};
use crate::auth::{check_authenticity};

pub async fn sign_up(
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
    Ok(Json(serde_json::json!({
        "name": user.name,
        "email": user.email,
        "id": user_id,
    })))
}
pub async fn login(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(user): Json<Login>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Fetch identity under a short lock, verify OFF lock.
    let stored_user = {
        let db = db.lock().await;
        if !email_exists(&db, &user.email)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        {
            return Err((StatusCode::BAD_REQUEST, "Email doesn't exist".to_string()));
        }
        get_user_auth_by_email(&db, &user.email)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    };
    let Some((user_id, user_name, user_email, password_hash)) = stored_user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        ));
    };
    if !cached_verify(user_id, &user.password, &password_hash) {
        return Err((StatusCode::BAD_REQUEST, "Invalid password".to_string()));
    }
    if needs_rehash(&password_hash) {
        let db = db.lock().await;
        let _ = rehash_password(&db, user_id, &user.password);
    }
    let json = serde_json::json!({
        "id": user_id,
        "name": user_name,
        "email": user_email,
    });
    Ok(Json(json))
}
