// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Text,
        last_login_time -> Int8,
        total_login_count -> Int4,
        consecutive_login_count -> Int4,
    }
}
