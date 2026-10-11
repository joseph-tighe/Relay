use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, PasswordHash, rand_core::OsRng};
use rusqlite::Connection;

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, stored_phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_phc) else { return false };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn check_password(
    id: i32,
    candidate_plaintext: &str,
    db: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT passwordhash FROM auth WHERE user_id = ?1")?;
    let stored: String = stmt.query_row([id], |row| row.get(0))?;
    if verify_password(candidate_plaintext, &stored) {
        Ok(())
    } else {
        Err("Invalid password".into())
    }
}

pub fn check_sql_injection(input: &str) -> bool {
    let forbidden_keywords = ["'", ";", "--"];//Illegal keywords
    let input_upper = input.to_uppercase();
    for keyword in forbidden_keywords.iter() {
        if input_upper.contains(keyword) {
            return true;
        }
    }
    false
}
pub fn check_xss(input: &str) -> bool {
    let forbidden_keywords = ["\"", "<", ">"];//Illegal keywords
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