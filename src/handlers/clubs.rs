use axum::{
     extract::{Path, State, Json},
     http::{header, StatusCode},
     response::IntoResponse,
};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::OptionalExtension; // for .optional() at :59, :75
use crate::models::{NewClub, NewMember, Auth};
use crate::auth::{check_password};
use crate::db::user::{get_user_by_id};
use crate::db::club::{make_new_club, add_user_to_club, get_club_by_id};
use crate::db::subspaces::make_new_subspace;
use crate::db::messages::add_message;

pub async fn get_club_json_by_id(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    let (club_name, club_image) = get_club_by_id(&db, id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let json = format!("{{\"name\": \"{club_name}\", \"image\": \"{club_image}\"}}");
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}
pub async fn api_make_club(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(new_club): Json<NewClub>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(new_club.owner_id, &new_club.owner_password.to_string(), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    let owner_id = new_club.owner_id;
    get_user_by_id(&db, owner_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let club_id = make_new_club(&db, &new_club.name, &new_club.image, owner_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let subspace_id = make_new_subspace(&db, "Main", None, club_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    add_message(
        &db,
        owner_id,
        subspace_id,
        &format!("Club {} created", new_club.name),
    )
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    add_user_to_club(&db, owner_id, club_id)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    println!("Created club {} with subspace {} for user {}", club_id, subspace_id, owner_id);
    Ok(Json(serde_json::json!({ "club_id": club_id, "subspace_id": subspace_id })))
}
pub async fn api_add_club_member(
    Path(id): Path<i32>,
    State(db): State<Arc<Mutex<Connection>>>,
    Json(member): Json<NewMember>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(member.user_id, &member.passwordhash.to_string(), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    let owner: Option<Option<i32>> = db
        .query_row(
            "SELECT owner_id FROM clubs WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let owner = owner.ok_or((StatusCode::NOT_FOUND, "No such club".to_string()))?;
    match owner {
        Some(owner_id) if owner_id != member.user_id => {
            return Err((StatusCode::UNAUTHORIZED, "Not the owner of club".to_string()));
        }
        // Owner match, or unclaimed club (NULL owner from before ownership existed).
        _ => {}
    }
    let new_id: Option<i32> = db
        .query_row(
            "SELECT id FROM users WHERE email = ?1",
            [&member.email],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let new_id = new_id.ok_or((StatusCode::NOT_FOUND, "No user with that email".to_string()))?;
    db.execute(
        "INSERT OR IGNORE INTO club_memberships (user_id, club_id) VALUES (?1, ?2)",
        rusqlite::params![new_id, id],
    )
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(serde_json::json!({ "user_id": new_id })))
}
pub async fn get_clubs_owned_by_user_id(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = db.lock().await;
    check_password(auth.user_id, &auth.passwordhash.to_string(), &db)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    let mut stmt = db
        .prepare("SELECT id, name, image FROM clubs WHERE owner_id = ?1 ORDER BY id ASC")
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let clubs = stmt
        .query_map([auth.user_id], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let mut clubs_json = Vec::new();
    for club in clubs {
        let (club_id, club_name, club_image) = club
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
        clubs_json.push(format!(
            "{{\"id\": {}, \"name\": \"{}\", \"image\": \"{}\"}}",
            club_id, club_name, club_image
        ));
    }
    let json = format!("{{\"clubs\": [{}]}}", clubs_json.join(", "));
    Ok(([(header::CONTENT_TYPE, "application/json")], json))
}