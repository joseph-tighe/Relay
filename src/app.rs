use axum::{Router, routing::{get, post}};
use tower_http::services::ServeDir;

use crate::AppState;
use crate::handers::{auth::{sign_up, login}, users::{get_user_json_by_id, get_clubs_in_by_user_id}, clubs::{get_club_json_by_id, api_make_club, api_add_club_member, get_clubs_owned_by_user_id}, subspaces::{get_subspaces_json_by_club_id, get_subspace_json_by_id, api_make_subspace}, messages::{get_messages_by_subspace_id, create_message}};
pub fn create_app(state: AppState) -> Router {
    let app = Router::new()
        .route("/api/user/{id}", get(get_user_json_by_id))
        .route("/api/clubs-in/{id}", get(get_clubs_in_by_user_id))
        .route("/api/club/{id}", get(get_club_json_by_id))
        .route("/api/clubs/mine", post(get_clubs_owned_by_user_id))
        .route("/api/club/new", post(api_make_club))
        .route("/api/subspace/new", post(api_make_subspace))
        .route("/api/club/{id}/subspaces", post(get_subspaces_json_by_club_id))
        .route("/api/club/{id}/members", post(api_add_club_member))
        .route("/api/subspace/{id}", post(get_subspace_json_by_id))
        .route("/api/subspace/{id}/messages", post(get_messages_by_subspace_id))
        .route("/api/subspace/{id}/messages/new", post(create_message))
        .route("/api/signup", post(sign_up))
        .route("/api/login", post(login))
        .with_state(state)
        .fallback_service(ServeDir::new("static"));
    app
}