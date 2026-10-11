use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use rusqlite::Connection;

pub fn hash_and_salt_password(password: &str, salt: &str) -> String {
    let mut password = password.to_owned();
    password.push_str(salt);
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
pub fn check_password(
    id: i32,
    user_password_hash: &str,
    db: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT passwordhash FROM auth WHERE user_id = ?1")?;
    let password_hash = stmt.query_row([id], |row| Ok(row.get::<_, String>(0)?))?;
    if password_hash != user_password_hash {
        return Err("Invalid password".into());
    }
    Ok(())
}

pub fn check_sql_injection(input: &str) -> bool {
    let forbidden_keywords = ["'", ";", "--"];
    let input_upper = input.to_uppercase();
    for keyword in forbidden_keywords.iter() {
        if input_upper.contains(keyword) {
            return true;
        }
    }
    false
}
pub fn check_xss(input: &str) -> bool {
    let forbidden_keywords = ["\"", "<", ">"];
    let input_upper = input.to_uppercase();
    for keyword in forbidden_keywords.iter() {
        if input_upper.contains(keyword) {
            return true;
        }
    }
    false
}
pub fn check_authenticity(input: &str) -> bool {
    if check_sql_injection(input) || check_xss(input) {
        return false;
    }
    true
}