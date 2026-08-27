use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionCount {
    pub emoji: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInfo {
    pub short_name: String,
    pub prompt: String,
    pub pipeline: String,
    pub model: String,
    pub variant_of: Option<String>,
    pub svg_url: String,
    pub png_url: String,
    pub reactions: Vec<ReactionCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub author: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    pub brief: String,
    pub candidates: Vec<CandidateInfo>,
    pub chat: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitRequest {
    pub svg: String,
    pub prompt: String,
    pub pipeline: String,
    pub model: String,
    pub variant_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResponse {
    pub short_name: String,
    pub message_id: String,
}
