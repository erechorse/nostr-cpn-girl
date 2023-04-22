CREATE TABLE users (
    id                      TEXT PRIMARY KEY,
    last_login_time         BIGINT NOT NULL,
    total_login_count       INTEGER NOT NULL,
    consecutive_login_count INTEGER NOT NULL
)
