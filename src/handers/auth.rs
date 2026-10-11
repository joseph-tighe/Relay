use axum::{
    extract::{State, Json},
    http::{header, StatusCode},
    response::IntoResponse,
};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::db::user::{email_exists, get_user_auth_by_email, add_user};
use crate::auth::{check_authenticity, hash_and_salt_password};
use crate::db::club::add_user_to_club;
use crate::models::{UserPassword, Login};

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
    let json = format!(
        "{{\"name\": \"{}\", \"email\": \"{}\", \"id\": {}}}",
        user.name, user.email, user_id
    );
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
pub async fn login(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(user): Json<Login>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    if !email_exists(&db, &user.email)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    {
        return Err((StatusCode::BAD_REQUEST, "Email doesn't exists".to_string()));
    }
    let stored_user = get_user_auth_by_email(&db, &user.email)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let Some((user_id, user_name, user_email, password_hash)) = stored_user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        ));
    };
    if hash_and_salt_password(&user.password, &user_id.to_string()) != password_hash {
        return Err((StatusCode::BAD_REQUEST, "Invalid password".to_string()));
    }
    let json =
        format!("{{\"id\": {user_id}, \"name\": \"{user_name}\", \"email\": \"{user_email}\"}}");
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}