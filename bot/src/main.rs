mod auth;
mod commands;
mod config;
mod db;
mod discord;
mod routes;
mod signature;
mod state;
mod svg;

use axum::routing::{get, post};
use axum::Router;
use state::AppState;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = config::Config::from_env()?;
    std::fs::create_dir_all(&config.data_dir)?;

    let db = db::Db::open(&config.database_path)?;
    let discord = discord::DiscordClient::new(&config.discord_bot_token);

    commands::register(&discord, config.discord_guild_id).await?;
    println!("bot: slash commands registered");

    let port = config.port;
    let state = AppState { db, discord: Arc::new(discord), config: Arc::new(config) };

    let app = Router::new()
        .route("/context", get(routes::context::handler))
        .route("/submit", post(routes::submit::handler))
        .route("/candidates/:short_name/svg", get(routes::candidates::svg_handler))
        .route("/candidates/:short_name/png", get(routes::candidates::png_handler))
        .route("/discord/interactions", post(routes::interactions::handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("bot: listening on port {port}");
    axum::serve(listener, app).await?;

    Ok(())
}
