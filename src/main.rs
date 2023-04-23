use std::str::FromStr;
use std::env;

use diesel::dsl::exists;
use diesel::{prelude::*, select};
use nostr_sdk::prelude::*;
use regex::Regex;

use nostr_cpn_girl::*;
use self::models::*;

const RELAY_URLS: [&str; 6] = [
    "wss://nostr.bitcoiner.social",
    "wss://relay.snort.social",
    "wss://relay.damus.io",
    "wss://relay.nostr.wirednet.jp",
    "wss://nos.lol",
    "wss://nostr-pub.wellorder.net"
];

#[tokio::main]
async fn main() -> Result<()> {
    use self::schema::users::dsl::*;
    
    // Get keys
    let my_keys = Keys::new(
        SecretKey::from_str(
            env::var("CPN_SECRET_KEY")
                .expect("CPN_SECRET_KEY is not set")
                .as_str()
        )?
    );
    println!("My bot pubkey is {}", my_keys.public_key().to_bech32()?);
    
    // Establish connection
    let connection = &mut establish_connection();

    // Create a new client
    let client = Client::new(&my_keys);
    for relay in RELAY_URLS {
        client.add_relay(relay, None).await?;
    }
    client.connect().await;

    // Set metadata
    let metadata = Metadata::new()
        .name("cpngirl")
        .display_name("ログボちゃん(テスト運用中)")
        .about("本名: 広瀬・ログボ・馬子。このアカウントに「ログボ」「ログインボーナス」とリプしていると、そのうちログインボーナスを配布するかも。")
        .website(Url::parse("https://github.com/erechorse/nostr-cpn-girl")?)
        .picture(Url::parse("https://i.gyazo.com/fe5c0915065b515e509ef351c71b617a.jpg")?)
        .nip05("cpngirl@erechorse.github.io")
        .lud06("viableproduct37@walletofsatoshi.com");
    client.set_metadata(metadata).await?;
    println!("setted metadata");

    let max_last_login_time = users
        .select(diesel::dsl::max(last_login_time))
        .get_result::<Option<i64>>(connection)
        .expect("Error loading users");
    let since_this_time = match max_last_login_time {
        Some(time) => if 10 + time < Timestamp::now().as_i64() {
            Timestamp::now().as_u64() - 10
        } else {
            time as u64 + 1 // +1 to avoid duplication
        }
        None => Timestamp::now().as_u64(),
    };

    // wait for mention
    let subscription = Filter::new()
        .limit(10) // up to 10 mentions
        .since(Timestamp::from(since_this_time))
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
                            if &event.created_at.as_i64() > &(Timestamp::now().as_i64() + 9) {
                                client.publish_text_note(format!(
                                    "未来からログインしないで！"
                                ), &[
                                    Tag::Event(event.id, None, None),
                                    Tag::PubKey(event.pubkey, None)
                                ]).await?;
                                continue;
                            }
                            let exists = select(
                                exists(users.filter(id.eq(&event.pubkey.to_bech32()?))))
                                .get_result::<bool>(connection)
                                .expect("Error loading users");

                            // If user is not new comer
                            if exists {
                                let before_user = users
                                    .find(&event.pubkey.to_bech32()?)
                                    .first::<User>(connection)
                                    .expect("Error loading users");
                                let now = event.created_at.as_i64();
                                let now_day = (now + 9 * 60 * 60) / (60 * 60 * 24);
                                let last_login = before_user.last_login_time;
                                let last_login_day = (last_login + 9 * 60 * 60) / (60 * 60 * 24);
                                let days_since_last_login = now_day - last_login_day;
                                
                                let mut after_user = UpdateUser {
                                    last_login_time: &event.created_at.as_i64(),
                                    total_login_count: &(before_user.total_login_count + 1),
                                    consecutive_login_count: &(before_user.consecutive_login_count + 1),
                                };
                                match days_since_last_login {
                                    0 => { // If user logged in today
                                        after_user.total_login_count = &before_user.total_login_count;
                                        after_user.consecutive_login_count = &before_user.consecutive_login_count;
                                        let user = diesel::update(users.find(event.pubkey.to_bech32()?))
                                            .set(&after_user)
                                            .get_result::<User>(connection)?;
                                        println!("Updated user: {}", user.id);
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
                                        let user = diesel::update(users.find(event.pubkey.to_bech32()?))
                                            .set(&after_user)
                                            .get_result::<User>(connection)?;
                                        println!("Updated user: {}", user.id);
                                    },
                                    _ => { // If user didn't log in for a while
                                        after_user.consecutive_login_count = &1;
                                        let user = diesel::update(users.find(event.pubkey.to_bech32()?))
                                            .set(&after_user)
                                            .get_result::<User>(connection)?;
                                        println!("Updated user: {}", user.id);
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
                            } else { // If user is new comer
                                let user = create_user(
                                    connection,
                                    &event.pubkey.to_bech32()?,
                                    &event.created_at.as_i64()
                                );
                                println!("Created user: {}", user.id);
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
