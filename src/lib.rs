pub mod models;
pub mod schema;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::env;

use self::models::{NewUser, User};

pub fn establish_connection() -> PgConnection {

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_user(conn: &mut PgConnection, id: &String, last_login_time: &i64) -> User {
    use crate::schema::users;

    let new_user = NewUser {
        id,
        last_login_time,
        total_login_count: &1,
        consecutive_login_count: &1,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .get_result(conn)
        .expect("Error saving new post")
}
