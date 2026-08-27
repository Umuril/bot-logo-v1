use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_api_url: String,
    pub worker_token: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            bot_api_url: std::env::var("BOT_API_URL").context("BOT_API_URL is not set")?,
            worker_token: std::env::var("WORKER_TOKEN").context("WORKER_TOKEN is not set")?,
        })
    }
}
