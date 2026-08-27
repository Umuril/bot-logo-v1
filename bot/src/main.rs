mod auth;
mod config;
mod db;
mod svg;

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let _config = config::Config::from_env()?;
    println!("bot: config loaded");
    Ok(())
}
