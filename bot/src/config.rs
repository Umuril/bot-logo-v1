use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_bot_token: String,
    pub discord_public_key: String,
    pub discord_guild_id: u64,
    pub discord_channel_id: u64,
    pub discord_allowed_role_id: u64,
    pub logo_brief: String,
    pub database_path: String,
    pub data_dir: String,
    pub port: u16,
}

fn require_env(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("{key} is not set"))
}

fn require_env_u64(key: &str) -> anyhow::Result<u64> {
    let raw = require_env(key)?;
    raw.parse()
        .with_context(|| format!("{key} must be a valid numeric ID, got {raw:?}"))
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            discord_bot_token: require_env("DISCORD_BOT_TOKEN")?,
            discord_public_key: require_env("DISCORD_PUBLIC_KEY")?,
            discord_guild_id: require_env_u64("DISCORD_GUILD_ID")?,
            discord_channel_id: require_env_u64("DISCORD_CHANNEL_ID")?,
            discord_allowed_role_id: require_env_u64("DISCORD_ALLOWED_ROLE_ID")?,
            logo_brief: require_env("LOGO_BRIEF")?,
            database_path: std::env::var("DATABASE_PATH").unwrap_or_else(|_| "bot.db".to_string()),
            data_dir: std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string()),
            port: match std::env::var("PORT") {
                Ok(raw) => raw.parse().with_context(|| format!("PORT must be numeric, got {raw:?}"))?,
                Err(_) => 8080,
            },
        })
    }
}
