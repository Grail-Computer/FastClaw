use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "grail-server")]
pub struct Config {
    #[arg(long, env = "PORT", default_value = "3000")]
    pub port: u16,

    #[arg(long, env = "GRAIL_DATA_DIR", default_value = "./data")]
    pub data_dir: PathBuf,

    /// Basic-auth password for the admin dashboard.
    #[arg(long, env = "ADMIN_PASSWORD")]
    pub admin_password: String,

    #[arg(long, env = "SLACK_SIGNING_SECRET")]
    pub slack_signing_secret: Option<String>,

    #[arg(long, env = "SLACK_BOT_TOKEN")]
    pub slack_bot_token: Option<String>,

    /// Optional base URL used when rendering links in the dashboard.
    #[arg(long, env = "BASE_URL")]
    pub base_url: Option<String>,
}

