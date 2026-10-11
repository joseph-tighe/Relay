use rusqlite::Connection;

pub fn get_subspaces_by_club_id(
    db: &Connection,
    club_id: i32,
) -> Result<Vec<(i32, String, Option<i32>)>, Box<dyn std::error::Error>> {
    //Select all subspaces from the subspaces table in a club
    let mut stmt = db.prepare(
        "SELECT id, name, parent_id FROM subspaces WHERE club_id = ?1 ORDER BY id ASC",
    )?;
    let subspaces = stmt.query_map([club_id], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
        ))
    })?;
    let mut subspaces_vec = Vec::new();
    for subspace in subspaces {
        subspaces_vec.push(subspace?);
    }
    Ok(subspaces_vec)
}
pub fn get_subspace_by_id(
    db: &Connection,
    id: i32,
) -> Result<(i32, String, Option<i32>, i32), Box<dyn std::error::Error>> {
    //Select a subspace based on id
    let mut stmt =
        db.prepare("SELECT id, name, parent_id, club_id FROM subspaces WHERE id = ?1")?;
    let subspace = stmt.query_row([id], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;
    Ok(subspace)
}
pub fn make_new_subspace(
    db: &Connection,
    name: &str,
    parent_id: Option<i32>,
    club_id: i32,
) -> Result<i32, Box<dyn std::error::Error>> {
    //Insert a subspace into the subspaces table
    db.execute(
        "INSERT INTO subspaces (name, parent_id, club_id) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, parent_id, club_id],
    )?;
    Ok(db.last_insert_rowid() as i32)
}