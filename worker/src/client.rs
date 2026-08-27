use crate::config::Config;
use shared::{ContextResponse, SubmitRequest, SubmitResponse};

pub struct BotClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl BotClient {
    pub fn new(config: &Config) -> Self {
        BotClient { base_url: config.bot_api_url.clone(), token: config.worker_token.clone(), http: reqwest::Client::new() }
    }

    pub async fn fetch_context(&self) -> anyhow::Result<ContextResponse> {
        let response = self.http.get(format!("{}/context", self.base_url)).bearer_auth(&self.token).send().await?.error_for_status()?;
        Ok(response.json().await?)
    }

    pub async fn submit(&self, request: &SubmitRequest) -> anyhow::Result<SubmitResponse> {
        let response = self.http.post(format!("{}/submit", self.base_url)).bearer_auth(&self.token).json(request).send().await?.error_for_status()?;
        Ok(response.json().await?)
    }

    pub async fn download(&self, url_path: &str) -> anyhow::Result<Vec<u8>> {
        let response = self.http.get(format!("{}{}", self.base_url, url_path)).bearer_auth(&self.token).send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }
}
