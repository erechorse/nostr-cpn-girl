use std::fs::File;
use std::io::{ErrorKind, Write, Read};
use std::str::FromStr;
use std::time::Duration;

use regex::Regex;
use rusqlite::{Connection, params, OptionalExtension};
use nostr_sdk::prelude::*;

const RELAY_URLS: [&str; 1] = [
    "wss://relay.damus.io"
];

fn key_getter() -> Keys {
    // Open secret key file
    let f = File::open("key.txt");

    // If file doesn't exist, create it and write secret key to it
    let mut f = match f {
        Ok(file) => file,
        Err(ref error) if error.kind() == ErrorKind::NotFound => {
            match File::create("key.txt") {
                Ok(mut fc) => {
                    let my_keys: Keys = Keys::generate();
                    let hex_sc_key = format!("{}",
                    my_keys.secret_key().unwrap().display_secret());
                    fc.write_all(hex_sc_key.as_bytes()).unwrap();
                    return my_keys;
                },
                Err(e) => {
                    panic!(
                        "Tried to create file but there was a problem: {:?}",
                        e
                    )
                },
            }
        },
        Err(error) => {
            panic!(
                "There was a problem opening the file: {:?}",
                error
            )
        },
    };

    // read secret key from file
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    Keys::new(
        SecretKey::from_str(s.trim()).unwrap(),
    )
}

fn query_user(conn: &Connection, pubkey: &str) -> Result<Option<User>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT * FROM user WHERE id = ?1"
    )?;
    let user = stmt.query_row(
        params![pubkey],
        |row| {
            Ok(User {
                id: row.get(0)?,
                last_login_time: row.get(1)?,
                total_login_count: row.get(2)?,
                consecutive_login_count: row.get(3)?,
            })
        }
    ).optional()?;
    Ok(user)
}

fn update_user(conn: &Connection, user: &User) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE user SET
                last_login_time = ?1,
                total_login_count = ?2,
                consecutive_login_count = ?3
                WHERE id = ?4",
        params![user.last_login_time, user.total_login_count, user.consecutive_login_count, user.id],
    )?;
    Ok(())
}

fn insert_user(conn: &Connection, user: &User) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO user (id, last_login_time, total_login_count, consecutive_login_count)
                VALUES (?1, ?2, ?3, ?4)",
        params![user.id, user.last_login_time, user.total_login_count, user.consecutive_login_count],
    )?;
    Ok(())
}

struct User {
    id: String,
    last_login_time: i64,
    total_login_count: i32,
    consecutive_login_count: i32,
}

#[tokio::main]
async fn main() -> nostr_sdk::Result<()> { // Result type conflicts with rusqlite::Result type
    // Get keys
    let my_keys = key_getter();
    println!("{}", my_keys.public_key().to_bech32()?);
    
    let conn = Connection::open("user.db")?;
    conn.execute(
        // id: pubkey
        // last_login_time: unix time
        "CREATE TABLE IF NOT EXISTS user (
                  id                      TEXT PRIMARY KEY,
                  last_login_time         INTEGER NOT NULL,
                  total_login_count       INTEGER NOT NULL,
                  consecutive_login_count INTEGER NOT NULL
                  )",
        [],
    ).unwrap();

    // Create a new client
    let client = Client::new(&my_keys);

    for relay in RELAY_URLS {
        client.add_relay(relay, None).await?;
    }

    client.connect().await;

    let subscription = Filter::new()
        .limit(1)
        .kind(Kind::Metadata)
        .author(my_keys.public_key());
    client.subscribe(vec![subscription]).await;

    // Set metadata
    let timeout = Duration::from_secs(1);
    let start = std::time::Instant::now();
    'outer: loop {
        let mut notifications = client.notifications();
        while let Ok(notification) = notifications.recv().await {
            if let RelayPoolNotification::Event(_url, event) = notification {
                match event.kind {
                    Kind::Metadata => {
                        println!("already setted metadata");
                        break 'outer;
                    },
                    _ => {}
                }
            }
            if start.elapsed() > timeout {
                let metadata = Metadata::new()
                    .name("testname")
                    .display_name("My TEST")
                    .about("hoge");
                client.set_metadata(metadata).await?;
                println!("setted metadata");
                break 'outer;
            }
        }
    }

    // wait for mention
    let subscription = Filter::new()
        .limit(0)
        .kind(Kind::TextNote)
        .pubkey(my_keys.public_key());
    client.subscribe(vec![subscription]).await;
    loop {
        let mut notifications = client.notifications();
        while let Ok(notification) = notifications.recv().await {
            if let RelayPoolNotification::Event(_url, event) = notification {
                match event.kind {
                    Kind::TextNote => {
                        let re = Regex::new(r"ログインボーナス|ログボ")?;
                        if re.is_match(&event.content) {
                            let exists = conn.query_row(
                                "SELECT EXISTS(SELECT 1 FROM user WHERE id = ?1)",
                                params![&event.pubkey.to_bech32()?],
                                |row| row.get(0), )?;

                            // If user is not new comer
                            if exists {
                                if let Some(before_user) = query_user(&conn, &event.pubkey.to_bech32()?)? {
                                    let now = event.created_at.as_i64();
                                    let now_day = (now + 9 * 60 * 60) / (60 * 60 * 24);
                                    let last_login_time = before_user.last_login_time;
                                    let last_login_day = (last_login_time + 9 * 60 * 60) / (60 * 60 * 24);
                                    let days_since_last_login = now_day - last_login_day;
                                    
                                    let mut after_user = User {
                                        id: event.pubkey.to_bech32()?,
                                        last_login_time: event.created_at.as_i64(),
                                        total_login_count: before_user.total_login_count + 1,
                                        consecutive_login_count: before_user.consecutive_login_count + 1,
                                    };
                                    match days_since_last_login {
                                        0 => { // If user logged in today
                                            client.publish_text_note(format!(
                                                "今日はもうログイン済みです。\nあなたの合計ログイン回数は{}回です。\nあなたの連続ログイン回数は{}回です。",
                                                before_user.total_login_count,
                                                before_user.consecutive_login_count,
                                            ), &[
                                                Tag::Event(event.id, None, None),
                                                Tag::PubKey(event.pubkey, None)
                                            ]).await?;
                                            continue;
                                        },
                                        1 => { // If user logged in yesterday
                                            match update_user(&conn, &after_user) {
                                                Ok(_) => (),
                                                Err(e) => println!("{}", e),
                                            };
                                        },
                                        _ => { // If user didn't log in for a while
                                            after_user.consecutive_login_count = 1;
                                            match update_user(&conn, &after_user) {
                                                Ok(_) => (),
                                                Err(e) => println!("{}", e),
                                            };
                                        }
                                    }
                                    client.publish_text_note(format!(
                                        "こんにちは！\nあなたの合計ログイン回数は{}回です。\nあなたの連続ログイン回数は{}回です。",
                                        after_user.total_login_count,
                                        after_user.consecutive_login_count
                                    ), &[
                                        Tag::Event(event.id, None, None),
                                        Tag::PubKey(event.pubkey, None)
                                    ]).await?;
                                }
                            } else {
                                let new_comer = User {
                                    id: event.pubkey.to_bech32()?,
                                    last_login_time: event.created_at.as_i64(),
                                    total_login_count: 1,
                                    consecutive_login_count: 1,
                                };
                                match insert_user(&conn, &new_comer) {
                                    Ok(_) => (),
                                    Err(e) => println!("{}", e)
                                };
                                client.publish_text_note(format!(
                                        "はじめまして！\n最初のログインです"
                                    ), &[
                                        Tag::Event(event.id, None, None),
                                        Tag::PubKey(event.pubkey, None)
                                    ]).await?;
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(())
}