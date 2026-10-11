#[derive(serde::Deserialize)]
pub struct NewSubspace {
    pub name: String,
    pub parent_id: Option<i32>,
    pub club_id: i32,
    pub owner_password: String,
    pub owner_id: i32
}
#[derive(serde::Deserialize)]
pub struct Auth {
    pub user_id: i32,
    pub passwordhash: String,
}
#[derive(serde::Deserialize)]
pub struct NewMessage {
    pub passwordhash: String,
    pub text: String,
    pub user_id: i32,
}
#[derive(Debug, serde::Deserialize)]
pub struct UserPassword {
    pub name: String,
    pub email: String,
    pub password: String,
}
#[derive(Debug, serde::Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}
#[derive(serde::Deserialize)]
pub struct NewClub {
    pub name: String,
    pub image: String,
    pub owner_id: i32,
    pub owner_password: String
}
#[derive(serde::Deserialize)]
pub struct NewMember {
    pub email: String,
    pub user_id: i32,
    pub passwordhash: String,
}
