use rusqlite::Connection;

pub fn add_user_to_club(
    db: &Connection,
    user_id: i32,
    club_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    //Insert the club and user to the membership table
    db.execute(
        "INSERT INTO club_memberships (user_id, club_id) VALUES (?1, ?2)",
        [user_id, club_id],
    )?;
    Ok(())
}
pub fn add_club(
    db: &Connection,
    name: &str,
    image: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    //Insert club into club table
    db.execute(
        "INSERT INTO clubs (name, image) VALUES (?1, ?2)",
        [name, image],
    )?;
    Ok(())
}
pub fn get_club_by_id(
    db: &Connection,
    id: i32,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    //Search for club by id, and then return (name, image)
    let mut stmt = db.prepare("SELECT name, image FROM clubs WHERE id = ?1")?;
    let club = stmt.query_row([id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(club)
}

pub fn make_new_club(
    db: &Connection,
    name: &str,
    image: &str,
    owner_id: i32,
) -> Result<i32, Box<dyn std::error::Error>> {
    //Insert name, image, and the owners user id into the club table
    db.execute(
        "INSERT INTO clubs (name, image, owner_id) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, image, owner_id],
    )?;
    Ok(db.last_insert_rowid() as i32)
}
pub fn get_club_members(
    db: &Connection,
    id: i32,
) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let mut stmt = db.prepare("SELECT user_id FROM club_memberships WHERE club_id = ?1")?;
    let members = stmt.query_map([id], |row| row.get(0))?;
    let mut members_vec = Vec::new();
    for member in members {
        members_vec.push(member?);
    }
    Ok(members_vec)
}

/// True when `user_id` is a member of `club_id`. Use for authorization
/// (not authentication): call after verifying the caller's password.
pub fn is_club_member(
    db: &Connection,
    user_id: i32,
    club_id: i32,
) -> Result<bool, Box<dyn std::error::Error>> {
    let member: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM club_memberships WHERE user_id = ?1 AND club_id = ?2)",
        rusqlite::params![user_id, club_id],
        |row| row.get(0),
    )?;
    Ok(member)
}