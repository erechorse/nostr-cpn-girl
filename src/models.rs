use crate::schema::users;
use diesel::prelude::*;

#[derive(Queryable)]
pub struct User {
    pub id: String,
    pub last_login_time: i64,
    pub total_login_count: i32,
    pub consecutive_login_count: i32,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub id: &'a String,
    pub last_login_time: &'a i64,
    pub total_login_count: &'a i32,
    pub consecutive_login_count: &'a i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
pub struct UpdateUser<'a> {
    pub last_login_time: &'a i64,
    pub total_login_count: &'a i32,
    pub consecutive_login_count: &'a i32,
}
