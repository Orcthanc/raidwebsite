use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct Cookie {
    pub id: String,
    pub user_id: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, Serialize)]
pub struct Character {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub class_id: i32,
    pub item_level: i32,
}

#[derive(Deserialize, Serialize)]
pub struct Class {
    pub id: i32,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Raid {
    pub id: i32,
    pub name: String,
    pub difficulty: String,
    pub required_item_level: i32,
}

#[derive(Deserialize)]
pub struct UserRaid {
    pub user_id: i32,
    pub character_id: i32,
    pub raid_id: i32,
}

#[derive(Deserialize)]
pub struct Group {
    pub id: i32,
    pub name: String,
    pub creator_id: i32,
}

#[derive(Deserialize)]
pub struct GroupMember {
    pub group_id: i32,
    pub user_id: i32,
}

