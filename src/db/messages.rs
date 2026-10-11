use rusqlite::Connection;

pub fn add_message(
    db: &Connection,
    user_id: i32,
    subspace_id: i32,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    db.execute(
        "INSERT INTO messages (user_id, subspace_id, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![user_id, subspace_id, content],
    )?;
    Ok(())
}
pub fn read_messages_by_subspace_id(
    db: &Connection,
    subspace_id: i32,
) -> Result<Vec<(i32, String, i32, String)>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare(
        "SELECT id, content, user_id, timestamp FROM messages WHERE subspace_id = ?1 ORDER BY id ASC",
    )?;
    let messages = stmt.query_map([subspace_id], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut messages_vec = Vec::new();
    for message in messages {
        messages_vec.push(message?);
    }
    Ok(messages_vec)
}