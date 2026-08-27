use crate::{config::Config, db::Db, discord::DiscordClient};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub discord: Arc<DiscordClient>,
    pub config: Arc<Config>,
}
