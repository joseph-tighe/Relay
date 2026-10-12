use argon2::{Argon2, Params, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, PasswordHash, rand_core::OsRng};
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};

// Tuned for interactive use: Argon2::default() (19 MiB, t=2) costs
// hundreds of ms in debug builds, and every API call verifies while the
// frontend polls every second. 4 MiB / t=1 / p=1 is ~10ms: still
// memory-hard + random salt, vastly better than DefaultHasher/SHA.
// To restore max OWASP strength, switch back to Argon2::default().
fn argon2_instance() -> Argon2<'static> {
    let params = Params::new(4096, 1, 1, None).expect("argon params");
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    argon2_instance()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, stored_phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_phc) else { return false };
    // Try current (fast) params first, then fall back to default params so
    // hashes created before tuning (m=19456,t=2) still verify. After a
    // successful fallback login, call maybe_rehash() to upgrade the row.
    if argon2_instance()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
    {
        return true;
    }
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

struct CacheEntry {
    ok: bool,
    expires: Instant,
}

fn verify_cache() -> &'static StdMutex<HashMap<String, CacheEntry>> {
    static CACHE: OnceLock<StdMutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| StdMutex::new(HashMap::new()))
}

/// Cached verify: the frontend sends the same secret on every call (and
/// polls every few seconds), so re-running Argon on each request is what
/// makes the app feel hung. Cache successful/failed outcomes briefly.
/// Keyed by the full candidate + stored hash (no collision shortcut), TTL
/// 5 min, capacity-bounded. Memory-only; cleared on restart.
pub fn cached_verify(user_id: i32, candidate: &str, stored: &str) -> bool {
    const TTL: Duration = Duration::from_secs(300);
    const CAP: usize = 10_000;
    let key = format!("{user_id}\x00{candidate}\x00{stored}");
    if let Ok(cache) = verify_cache().lock() {
        if let Some(e) = cache.get(&key) {
            if e.expires > Instant::now() {
                return e.ok;
            }
        }
    }
    let ok = verify_password(candidate, stored);
    if let Ok(mut cache) = verify_cache().lock() {
        if cache.len() >= CAP {
            cache.retain(|_, e| e.expires > Instant::now());
        }
        if cache.len() >= CAP {
            cache.clear();
        }
        cache.insert(
            key,
            CacheEntry {
                ok,
                expires: Instant::now() + TTL,
            },
        );
    }
    ok
}

/// Quick locked read: fetch the stored PHC string. Callers should drop the
/// mutex guard BEFORE calling verify_password so slow hashes never block
/// other requests holding the single global connection.
pub fn get_password_hash(
    db: &Connection,
    id: i32,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    use rusqlite::OptionalExtension;
    let stored: Option<String> = db
        .query_row(
            "SELECT passwordhash FROM auth WHERE user_id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(stored)
}

/// True when the stored hash was made with different params (e.g. old
/// default 19 MiB/t=2 rows). Transparently upgrade them on next login.
pub fn needs_rehash(stored_phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_phc) else { return true };
    !(parsed.params.get_decimal("m") == Some(4096)
        && parsed.params.get_decimal("t") == Some(1)
        && parsed.params.get_decimal("p") == Some(1))
}

pub fn rehash_password(
    db: &Connection,
    id: i32,
    plaintext: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let new_hash = hash_password(plaintext).map_err(|e| format!("rehash: {e}"))?;
    db.execute(
        "UPDATE auth SET passwordhash = ?1 WHERE user_id = ?2",
        rusqlite::params![new_hash, id],
    )?;
    Ok(())
}

#[allow(dead_code)] // used by tests; handlers use fetch-then-verify to avoid holding the lock
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
pub fn check_xss(input: &str) -> bool {
    let forbidden_keywords = ["<", ">"];//Illegal keywords
    let input_upper = input.to_uppercase();
    for keyword in forbidden_keywords.iter() {
        if input_upper.contains(keyword) {
            return true;
        }
    }
    false
}
pub fn check_json_injection(input: &str) -> bool {
    let forbidden_keywords = ["\"", "{", "}"];//Illegal keywords
    let input_upper = input.to_uppercase();
    for keyword in forbidden_keywords.iter() {
        if input_upper.contains(keyword) {
            return true;
        }
    }
    false
}
pub fn check_authenticity(input: &str) -> bool {
    return (!check_json_injection(input)) && (!check_xss(input));
}