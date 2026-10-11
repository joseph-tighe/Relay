use rusqlite::Connection;
use rusqlite::OptionalExtension;
use crate::auth::hash_and_salt_password;

pub fn add_user(
    db: &mut Connection,
    name: &str,
    email: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let transaction = db.transaction()?;
    transaction.execute(
        "INSERT INTO users (name, email) VALUES (?1, ?2)",
        [name, email],
    )?;
    let user_id = transaction.last_insert_rowid();
    transaction.execute(
        "INSERT INTO auth (user_id, passwordhash) VALUES (?1, ?2)",
        rusqlite::params![user_id, hash_and_salt_password(password, &user_id.to_string())],
    )?;
    transaction.commit()?;
    Ok(())
}
pub fn get_user_by_id(
    db: &Connection,
    id: i32,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT name, email FROM users WHERE id = ?")?;
    let user = stmt.query_row([id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(user)
}
pub fn get_clubs_by_user_id(
    db: &Connection,
    user_id: i32,
) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT club_id FROM club_memberships WHERE user_id = ?1")?;
    let clubs_ids_a = stmt.query_map([user_id], |row| row.get::<_, i32>(0))?;
    let mut clubs_ids = Vec::new();
    for club_id in clubs_ids_a {
        clubs_ids.push(club_id?);
    }
    Ok(clubs_ids)
}
pub fn email_exists(db: &Connection, email: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let exists = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = ?1)",
        [email],
        |row| row.get(0),
    )?;
    Ok(exists)
}
pub fn get_user_auth_by_email(
    db: &Connection,
    email: &str,
) -> Result<Option<(i32, String, String, String)>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare(
        "SELECT users.id, users.name, users.email, auth.passwordhash
         FROM users JOIN auth ON auth.user_id = users.id
         WHERE users.email = ?1",
    )?;
    let user = stmt
        .query_row([email], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .optional()?;
    Ok(user)
}