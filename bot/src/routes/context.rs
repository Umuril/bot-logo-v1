use crate::routes::extractors::AuthenticatedWorker;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use shared::{CandidateInfo, ContextResponse};

pub async fn handler(State(state): State<AppState>, AuthenticatedWorker(_worker): AuthenticatedWorker) -> impl IntoResponse {
    let candidates = match state.db.list_candidates() {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to list candidates").into_response(),
    };

    let mut candidate_infos = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let reactions = match candidate.message_id.parse::<u64>() {
            Ok(message_id) => state
                .discord
                .reactions_for_message(state.config.discord_channel_id, message_id)
                .await
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        candidate_infos.push(CandidateInfo {
            short_name: candidate.short_name.clone(),
            prompt: candidate.prompt.clone(),
            pipeline: candidate.pipeline.clone(),
            model: candidate.model.clone(),
            variant_of: candidate.variant_of.clone(),
            svg_url: format!("/candidates/{}/svg", candidate.short_name),
            png_url: format!("/candidates/{}/png", candidate.short_name),
            reactions,
        });
    }

    let chat = state.discord.recent_messages(state.config.discord_channel_id, 50).await.unwrap_or_default();

    Json(ContextResponse { brief: state.config.logo_brief.clone(), candidates: candidate_infos, chat }).into_response()
}
