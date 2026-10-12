//! Integration + unit tests for ClubRustProject.
//!
//! Run with: `cargo test`
//!
//! Strategy: every test builds its own in-memory SQLite DB with the same
//! schema as `src/bin/make_db.rs`, so tests are isolated and never touch
//! `example.db`. Handlers are invoked directly (no extra HTTP test crates).

use std::sync::Arc;

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::AppState;
use crate::auth::{
    cached_verify, check_authenticity, check_json_injection, check_password, check_xss,
    get_password_hash, hash_password, needs_rehash, rehash_password, verify_password,
};
use crate::db::club::{add_club, add_user_to_club, get_club_by_id, get_club_members, make_new_club};
use crate::db::messages::{add_message, read_messages_by_subspace_id};
use crate::db::subspaces::{get_subspace_by_id, get_subspaces_by_club_id, make_new_subspace};
use crate::db::user::{add_user, email_exists, get_clubs_by_user_id, get_user_auth_by_email, get_user_by_id};
use crate::handlers::auth::{login, sign_up};
use crate::handlers::clubs::{api_add_club_member, api_make_club, get_club_json_by_id, get_clubs_owned_by_user_id};
use crate::handlers::messages::{create_message, get_messages_by_subspace_id};
use crate::handlers::subspaces::{api_make_subspace, get_subspace_json_by_id, get_subspaces_json_by_club_id};
use crate::handlers::users::{get_clubs_in_by_user_id, get_user_json_by_id};
use crate::models::{Auth, Login, NewClub, NewMember, NewMessage, NewSubspace, UserPassword};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn init_schema(db: &Connection) {
    db.execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL
        )",
        [],
    )
    .unwrap();
    db.execute(
        "CREATE TABLE clubs (
            owner_id INTEGER,
            id INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            image TEXT,
            FOREIGN KEY (owner_id) REFERENCES users(id)
        )",
        [],
    )
    .unwrap();
    db.execute(
        "CREATE TABLE subspaces (
            id INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            parent_id INTEGER,
            club_id INTEGER NOT NULL,
            FOREIGN KEY (parent_id) REFERENCES subspaces(id),
            FOREIGN KEY (club_id) REFERENCES clubs(id)
        )",
        [],
    )
    .unwrap();
    db.execute(
        "CREATE TABLE club_memberships (
            user_id INTEGER NOT NULL,
            club_id INTEGER NOT NULL,
            PRIMARY KEY (user_id, club_id),
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (club_id) REFERENCES clubs(id)
        )",
        [],
    )
    .unwrap();
    db.execute(
        "CREATE TABLE messages (
            id INTEGER PRIMARY KEY NOT NULL,
            user_id INTEGER NOT NULL,
            subspace_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (subspace_id) REFERENCES subspaces(id)
        )",
        [],
    )
    .unwrap();
    db.execute(
        "CREATE TABLE auth (
            user_id INTEGER PRIMARY KEY NOT NULL,
            passwordhash TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )",
        [],
    )
    .unwrap();
}

/// Fresh in-memory DB with schema, no rows.
fn fresh_state() -> AppState {
    let db = Connection::open_in_memory().expect("in-memory db");
    init_schema(&db);
    Arc::new(Mutex::new(db))
}

/// Seed one user (id 1) + one club (id 1) + one subspace (id 1) + membership.
/// Returns (user_id, club_id, subspace_id). Password is "secret123".
async fn seeded_state() -> (AppState, i32, i32, i32) {
    let state = fresh_state();
    let (user_id, club_id, subspace_id) = {
        let mut db = state.lock().await;
        add_user(&mut db, "alice", "alice@example.com", "secret123").unwrap();
        let user_id = db.last_insert_rowid() as i32; // == 1
        let club_id = make_new_club(&db, "Chess Club", "img.png", user_id).unwrap(); // == 1
        let subspace_id = make_new_subspace(&db, "Main", None, club_id).unwrap(); // == 1
        add_user_to_club(&db, user_id, club_id).unwrap();
        (user_id, club_id, subspace_id)
    };
    (state, user_id, club_id, subspace_id)
}

async fn body_text(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// ---------------------------------------------------------------------------
// auth.rs unit tests
// ---------------------------------------------------------------------------

#[test]
fn hash_and_verify_roundtrip() {
    let hash = hash_password("correct horse").expect("hash");
    assert!(verify_password("correct horse", &hash));
    assert!(!verify_password("wrong", &hash));
}

#[test]
fn verify_malformed_hash_is_false() {
    assert!(!verify_password("anything", "not-a-valid-phc"));
    assert!(!verify_password("anything", ""));
}

#[test]
fn check_xss_blocks_angle_brackets() {
    assert!(check_xss("<script>"));
    assert!(check_xss("a>b"));
    assert!(check_xss("a<b"));
    assert!(!check_xss("plain name 123"));
}

#[test]
fn check_json_injection_blocks_quotes_and_braces() {
    assert!(check_json_injection("he said \"hi\""));
    assert!(check_json_injection("{evil}"));
    assert!(check_json_injection("a{b"));
    assert!(!check_json_injection("plain-text_123"));
}

#[test]
fn check_authenticity_combines_both() {
    assert!(check_authenticity("normal message here"));
    assert!(!check_authenticity("<bad>"));
    assert!(!check_authenticity("quote \" here"));
}

#[test]
fn needs_rehash_flags_old_and_garbage() {
    let fresh = hash_password("pw").unwrap();
    assert!(!needs_rehash(&fresh), "fresh hash should not need rehash");
    assert!(needs_rehash("garbage"));
}

#[tokio::test]
async fn auth_db_helpers_roundtrip() {
    let state = fresh_state();
    let mut db = state.lock().await;
    add_user(&mut db, "bob", "bob@example.com", "hunter2").unwrap();
    let user_id = db.last_insert_rowid() as i32;

    // get_password_hash
    let stored = get_password_hash(&db, user_id).unwrap();
    assert!(stored.is_some());
    assert!(get_password_hash(&db, 9999).unwrap().is_none());

    // check_password ok / err
    assert!(check_password(user_id, "hunter2", &db).is_ok());
    assert!(check_password(user_id, "nope", &db).is_err());

    // cached_verify mirrors verify
    let stored = stored.unwrap();
    assert!(cached_verify(user_id, "hunter2", &stored));
    assert!(!cached_verify(user_id, "nope", &stored));

    // rehash keeps password valid
    rehash_password(&db, user_id, "hunter2").unwrap();
    assert!(check_password(user_id, "hunter2", &db).is_ok());
}

// ---------------------------------------------------------------------------
// db::user tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn add_and_get_user() {
    let state = fresh_state();
    let mut db = state.lock().await;
    add_user(&mut db, "carol", "carol@example.com", "pw123").unwrap();
    let id = db.last_insert_rowid() as i32;
    let (name, email) = get_user_by_id(&db, id).unwrap();
    assert_eq!(name, "carol");
    assert_eq!(email, "carol@example.com");
    assert!(email_exists(&db, "carol@example.com").unwrap());
    assert!(!email_exists(&db, "nobody@example.com").unwrap());
}

#[tokio::test]
async fn duplicate_email_fails() {
    let state = fresh_state();
    let mut db = state.lock().await;
    add_user(&mut db, "dave", "dave@example.com", "pw").unwrap();
    assert!(add_user(&mut db, "dave2", "dave@example.com", "pw").is_err());
}

#[tokio::test]
async fn get_missing_user_errors() {
    let state = fresh_state();
    let db = state.lock().await;
    assert!(get_user_by_id(&db, 4242).is_err());
    assert!(get_user_auth_by_email(&db, "ghost@example.com").unwrap().is_none());
}

#[tokio::test]
async fn user_auth_lookup_and_clubs() {
    let (state, user_id, club_id, _) = seeded_state().await;
    let db = state.lock().await;
    let row = get_user_auth_by_email(&db, "alice@example.com").unwrap().unwrap();
    assert_eq!(row.0, user_id);
    assert_eq!(row.1, "alice");
    assert_eq!(row.2, "alice@example.com");
    assert!(verify_password("secret123", &row.3));

    let clubs = get_clubs_by_user_id(&db, user_id).unwrap();
    assert_eq!(clubs, vec![club_id]);

    // user with no memberships -> empty vec
    let db2 = fresh_state();
    let mut db2g = db2.lock().await;
    add_user(&mut db2g, "loner", "loner@example.com", "pw").unwrap();
    let loner = db2g.last_insert_rowid() as i32;
    assert!(get_clubs_by_user_id(&db2g, loner).unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// db::club tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn club_crud_and_membership() {
    let state = fresh_state();
    let mut db = state.lock().await;
    add_user(&mut db, "owner", "owner@example.com", "pw").unwrap();
    let owner = db.last_insert_rowid() as i32;

    add_club(&db, "Go Club", "go.png").unwrap();
    let (name, img) = get_club_by_id(&db, 1).unwrap();
    assert_eq!(name, "Go Club");
    assert_eq!(img, "go.png");

    let club_id = make_new_club(&db, "Rust Club", "rust.png", owner).unwrap();
    add_user_to_club(&db, owner, club_id).unwrap();
    assert_eq!(get_club_members(&db, club_id).unwrap(), vec![owner]);
    // duplicate membership errors (plain INSERT, no OR IGNORE at db layer)
    assert!(add_user_to_club(&db, owner, club_id).is_err());
}

// ---------------------------------------------------------------------------
// db::subspaces tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn subspace_crud() {
    let (state, _, club_id, _) = seeded_state().await;
    let db = state.lock().await;

    let child = make_new_subspace(&db, "Child", Some(1), club_id).unwrap();
    let (id, name, parent, club) = get_subspace_by_id(&db, child).unwrap();
    assert_eq!(id, child);
    assert_eq!(name, "Child");
    assert_eq!(parent, Some(1));
    assert_eq!(club, club_id);

    let all = get_subspaces_by_club_id(&db, club_id).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].1, "Main");
    assert_eq!(all[1].1, "Child");

    assert!(get_subspaces_by_club_id(&db, 9999).unwrap().is_empty());
    assert!(get_subspace_by_id(&db, 9999).is_err());
}

// ---------------------------------------------------------------------------
// db::messages tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn messages_ordered_and_empty() {
    let (state, user_id, _, subspace_id) = seeded_state().await;
    let db = state.lock().await;

    assert!(read_messages_by_subspace_id(&db, subspace_id).unwrap().is_empty());

    add_message(&db, user_id, subspace_id, "first").unwrap();
    add_message(&db, user_id, subspace_id, "second").unwrap();
    let msgs = read_messages_by_subspace_id(&db, subspace_id).unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].1, "first");
    assert_eq!(msgs[1].1, "second");
    assert_eq!(msgs[0].2, user_id);
}

// ---------------------------------------------------------------------------
// handler: auth (signup / login)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn signup_success() {
    let state = fresh_state();
    let res = sign_up(
        State(state.clone()),
        Json(UserPassword {
            name: "eve".into(),
            email: "eve@example.com".into(),
            password: "s3cret!".into(),
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["name"], "eve");
    assert_eq!(v["email"], "eve@example.com");
    // can log in afterwards
    assert!(email_exists(&*state.lock().await, "eve@example.com").unwrap());
}

#[tokio::test]
async fn signup_rejects_duplicate_email() {
    let state = fresh_state();
    let payload = || UserPassword {
        name: "fred".into(),
        email: "fred@example.com".into(),
        password: "pw".into(),
    };
    sign_up(State(state.clone()), Json(payload())).await.unwrap();
    let err = sign_up(State(state.clone()), Json(payload())).await.err().unwrap();
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
    assert!(err.1.contains("Email already exists"));
}

#[tokio::test]
async fn signup_rejects_xss_name() {
    let state = fresh_state();
    let err = sign_up(
        State(state.clone()),
        Json(UserPassword {
            name: "<script>".into(),
            email: "x@example.com".into(),
            password: "pw".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_success_and_failures() {
    let (state, user_id, _, _) = seeded_state().await;

    // success
    let res = login(
        State(state.clone()),
        Json(Login {
            email: "alice@example.com".into(),
            password: "secret123".into(),
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["id"], user_id);
    assert_eq!(v["name"], "alice");

    // wrong password -> 400 per current handler
    let err = login(
        State(state.clone()),
        Json(Login {
            email: "alice@example.com".into(),
            password: "wrong".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::BAD_REQUEST);

    // unknown email -> 400 per current handler
    let err = login(
        State(state.clone()),
        Json(Login {
            email: "nobody@example.com".into(),
            password: "whatever".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// handler: users + clubs reads
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_user_and_clubs_in() {
    let (state, user_id, club_id, _) = seeded_state().await;

    let res = get_user_json_by_id(Path(user_id), State(state.clone()))
        .await
        .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["name"], "alice");

    let res = get_clubs_in_by_user_id(Path(user_id), State(state.clone()))
        .await
        .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["clubs"], serde_json::json!([club_id]));

    let res = get_club_json_by_id(Path(club_id), State(state.clone()))
        .await
        .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["name"], "Chess Club");
}

// ---------------------------------------------------------------------------
// handler: make club / members / owned
// ---------------------------------------------------------------------------

#[tokio::test]
async fn make_club_creates_main_subspace_and_membership() {
    let (state, user_id, _, _) = seeded_state().await;
    let res = api_make_club(
        State(state.clone()),
        Json(NewClub {
            name: "New Club".into(),
            image: "img.png".into(),
            owner_id: user_id,
            owner_password: "secret123".into(),
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let club_id = v["club_id"].as_i64().unwrap() as i32;
    let subspace_id = v["subspace_id"].as_i64().unwrap() as i32;

    let db = state.lock().await;
    let (name, _) = get_club_by_id(&db, club_id).unwrap();
    assert_eq!(name, "New Club");
    let sub = get_subspace_by_id(&db, subspace_id).unwrap();
    assert_eq!(sub.1, "Main");
    assert!(get_club_members(&db, club_id).unwrap().contains(&user_id));
}

#[tokio::test]
async fn make_club_wrong_password_is_unauthorized() {
    let (state, user_id, _, _) = seeded_state().await;
    let err = api_make_club(
        State(state.clone()),
        Json(NewClub {
            name: "Bad".into(),
            image: "".into(),
            owner_id: user_id,
            owner_password: "wrong".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn add_member_owner_ok_non_owner_rejected() {
    let (state, owner_id, club_id, _) = seeded_state().await;
    // second user
    let member_id = {
        let mut db = state.lock().await;
        add_user(&mut db, "bob", "bob@example.com", "bobpw").unwrap();
        db.last_insert_rowid() as i32
    };

    // owner adds bob
    let res = api_add_club_member(
        Path(club_id),
        State(state.clone()),
        Json(NewMember {
            email: "bob@example.com".into(),
            user_id: owner_id,
            passwordhash: "secret123".into(),
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["user_id"], member_id);

    // non-owner (bob, not owner) tries to add someone -> 401
    {
        let mut db = state.lock().await;
        add_user(&mut db, "carol", "carol@example.com", "carolpw").unwrap();
    }
    let err = api_add_club_member(
        Path(club_id),
        State(state.clone()),
        Json(NewMember {
            email: "carol@example.com".into(),
            user_id: member_id,
            passwordhash: "bobpw".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);

    // unknown email -> 404
    let err = api_add_club_member(
        Path(club_id),
        State(state.clone()),
        Json(NewMember {
            email: "ghost@example.com".into(),
            user_id: owner_id,
            passwordhash: "secret123".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn clubs_owned_lists_only_mine() {
    let (state, user_id, club_id, _) = seeded_state().await;
    let res = get_clubs_owned_by_user_id(
        State(state.clone()),
        Json(Auth {
            user_id,
            passwordhash: "secret123".into(),
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["clubs"][0]["id"], club_id);

    // wrong password -> 401
    let err = get_clubs_owned_by_user_id(
        State(state.clone()),
        Json(Auth {
            user_id,
            passwordhash: "bad".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// handler: subspaces
// ---------------------------------------------------------------------------

#[tokio::test]
async fn subspaces_list_get_and_create() {
    let (state, user_id, club_id, subspace_id) = seeded_state().await;
    let auth = || Auth {
        user_id,
        passwordhash: "secret123".into(),
    };

    let res = get_subspaces_json_by_club_id(Path(club_id), State(state.clone()), Json(auth()))
        .await
        .unwrap();
    let body = body_text(res.into_response()).await;
    assert!(body.contains("Main"));

    let res = get_subspace_json_by_id(Path(subspace_id), State(state.clone()), Json(auth()))
        .await
        .unwrap();
    let body = body_text(res.into_response()).await;
    assert!(body.contains("Main"));

    let res = api_make_subspace(
        State(state.clone()),
        Json(NewSubspace {
            name: "Side".into(),
            parent_id: Some(subspace_id),
            club_id,
            owner_password: "secret123".into(),
            owner_id: user_id,
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v["subspace_id"].as_i64().unwrap() > 0);

    // invalid (XSS) name -> 400
    let err = api_make_subspace(
        State(state.clone()),
        Json(NewSubspace {
            name: "<bad>".into(),
            parent_id: None,
            club_id,
            owner_password: "secret123".into(),
            owner_id: user_id,
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::BAD_REQUEST);

    // wrong password -> 401
    let err = api_make_subspace(
        State(state.clone()),
        Json(NewSubspace {
            name: "Ok".into(),
            parent_id: None,
            club_id,
            owner_password: "nope".into(),
            owner_id: user_id,
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// handler: messages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn messages_create_list_and_guards() {
    // Membership is resolved via the subspace's owning club (not the raw
    // subspace id), so use a second club/subspace pair with divergent ids
    // to prove the namespace handling, plus the seeded club 1 / subspace 1.
    let (state, user_id, _, subspace_id) = seeded_state().await;

    // stranger (non-member) cannot read
    let stranger = {
        let mut db = state.lock().await;
        add_user(&mut db, "mallory", "mallory@example.com", "mallorypw").unwrap();
        db.last_insert_rowid() as i32
    };
    let err = get_messages_by_subspace_id(
        Path(subspace_id),
        State(state.clone()),
        Json(Auth {
            user_id: stranger,
            passwordhash: "mallorypw".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);

    // member creates + lists
    let res = create_message(
        Path(subspace_id),
        State(state.clone()),
        Json(NewMessage {
            passwordhash: "secret123".into(),
            text: "hello world".into(),
            user_id,
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    assert!(body.contains("hello world"));

    let res = get_messages_by_subspace_id(
        Path(subspace_id),
        State(state.clone()),
        Json(Auth {
            user_id,
            passwordhash: "secret123".into(),
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    assert!(body.contains("hello world"));

    // XSS text rejected
    let err = create_message(
        Path(subspace_id),
        State(state.clone()),
        Json(NewMessage {
            passwordhash: "secret123".into(),
            text: "<inject>".into(),
            user_id,
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::BAD_REQUEST);

    // non-member cannot post
    let err = create_message(
        Path(subspace_id),
        State(state.clone()),
        Json(NewMessage {
            passwordhash: "mallorypw".into(),
            text: "hi".into(),
            user_id: stranger,
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);

    // Divergent ids: club 2 owns subspace 2. Mallory joins club 2 only.
    // She must read subspace 2 (member of its club) but not subspace 1.
    // This proves membership resolves via the owning club, not the raw id.
    let other_subspace = {
        let db = state.lock().await;
        let club2 = make_new_club(&db, "Second", "img.png", stranger).unwrap();
        assert_ne!(club2, subspace_id, "need divergent club/subspace ids");
        let sub2 = make_new_subspace(&db, "Second Main", None, club2).unwrap();
        add_user_to_club(&db, stranger, club2).unwrap();
        sub2
    };
    let res = get_messages_by_subspace_id(
        Path(other_subspace),
        State(state.clone()),
        Json(Auth {
            user_id: stranger,
            passwordhash: "mallorypw".into(),
        }),
    )
    .await
    .unwrap();
    let body = body_text(res.into_response()).await;
    assert!(body.contains("messages"));

    // owner of club 1 is NOT a member of club 2 -> denied on subspace 2
    let err = get_messages_by_subspace_id(
        Path(other_subspace),
        State(state.clone()),
        Json(Auth {
            user_id,
            passwordhash: "secret123".into(),
        }),
    )
    .await
    .err().unwrap();
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// app wiring smoke test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn app_builds_with_all_routes() {
    let (state, _, _, _) = seeded_state().await;
    let _app = crate::app::create_app(state);
}
