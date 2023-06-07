use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct NostrConfig {
    pub secretkey: String,
    pub relays: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct NostrMetadata {
    pub name: String,
    pub display_name: String,
    pub about: String,
    pub website: String,
    pub picture: String,
    pub nip05: String,
    pub lud06: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub nostr: NostrConfig,
    pub metadata: NostrMetadata,
}
