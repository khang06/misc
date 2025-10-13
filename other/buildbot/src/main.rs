use std::{path::PathBuf, sync::Arc};

use serde::Deserialize;

mod discord;
mod server;
mod verifier;

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub discord_token: String,
    pub owner_id: u64,
    pub updates_channel: u64,
    pub approval_channel: u64,
    pub game_activity: Option<String>,

    pub bind_addr: String,
    pub password: String,
    pub upload_path: PathBuf,
    pub base_url: String,
    pub archive_prefix: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let config = Arc::new(envy::from_env::<Config>().expect("loading config"));
    discord::run_bot(config).await;
}
