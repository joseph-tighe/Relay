use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::OptionalExtension; // for .optional()
use crate::auth::{cached_verify, get_password_hash, needs_rehash, rehash_password};
use crate::db::subspaces::{get_subspaces_by_club_id, get_subspace_by_id, make_new_subspace, get_club_id_by_subspace};
use crate::db::club::is_club_member;
use crate::models::{NewSubspace, Auth};
use crate::auth::{check_authenticity};

pub async fn get_subspaces_json_by_club_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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
    let member = is_club_member(&db, auth.user_id, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    if !member {
        return Err((StatusCode::UNAUTHORIZED, "Not a member".to_string()));
    }
    let subspaces = get_subspaces_by_club_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let subspaces_json: Vec<serde_json::Value> = subspaces
        .into_iter()
        .map(|subspace| {
            serde_json::json!({
                "id": subspace.0,
                "name": subspace.1,
                "parent_id": subspace.2,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "subspaces": subspaces_json })))
}
pub async fn get_subspace_json_by_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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
    let subspace = get_subspace_by_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(serde_json::json!({
        "id": subspace.0,
        "name": subspace.1,
        "parent_id": subspace.2,
        "club_id": subspace.3,
    })))
}
pub async fn api_make_subspace(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(new_subspace): Json<NewSubspace>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let stored = {
        let db = db.lock().await;
        get_password_hash(&db, new_subspace.owner_id)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    };
    let Some(stored) = stored else {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_string()));
    };
    if !cached_verify(new_subspace.owner_id, &new_subspace.owner_password, &stored) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_string()));
    }
    if needs_rehash(&stored) {
        let db = db.lock().await;
        let _ = rehash_password(&db, new_subspace.owner_id, &new_subspace.owner_password);
    }
    let db = db.lock().await;
    let owner: Option<i32> = db
        .query_row(
            "SELECT owner_id FROM clubs WHERE id = ?1",
            [new_subspace.club_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    match owner {
        Some(id) if id != new_subspace.owner_id => {
            return Err((StatusCode::UNAUTHORIZED, "Not the owner of club".to_string()));
        }
        // Owner match, or unclaimed club (NULL owner from before ownership existed).
        _ => {}
    }
    check_authenticity(&new_subspace.name)
        .then(|| ())
        .ok_or((StatusCode::BAD_REQUEST, "Invalid subspace name".to_string()))?;
    let subspace_id = make_new_subspace(
        &db,
        &new_subspace.name,
        new_subspace.parent_id,
        new_subspace.club_id,
    )
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(serde_json::json!({ "subspace_id": subspace_id })))
}
