use crate::routes::extractors::AuthenticatedWorker;
use crate::state::AppState;
use crate::svg;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use shared::{SubmitRequest, SubmitResponse};

pub async fn handler(
    State(state): State<AppState>,
    AuthenticatedWorker(worker): AuthenticatedWorker,
    Json(request): Json<SubmitRequest>,
) -> impl IntoResponse {
    let sanitized_svg = svg::sanitize(&request.svg);
    let png_bytes = match svg::render_png(&sanitized_svg) {
        Ok(bytes) => bytes,
        Err(_) => return (StatusCode::BAD_REQUEST, "submitted SVG could not be rendered").into_response(),
    };

    let short_name = match state.db.next_short_name() {
        Ok(name) => name,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to allocate a candidate name").into_response(),
    };

    let svg_path = format!("{}/{}.svg", state.config.data_dir, short_name);
    let png_path = format!("{}/{}.png", state.config.data_dir, short_name);
    if std::fs::create_dir_all(&state.config.data_dir).is_err()
        || std::fs::write(&svg_path, &sanitized_svg).is_err()
        || std::fs::write(&png_path, &png_bytes).is_err()
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to store candidate files").into_response();
    }

    let variant_note = request.variant_of.as_deref().map(|v| format!(" · variant of {v}")).unwrap_or_default();
    let caption = format!(
        "**{short_name}** — generated via `{}` (model: `{}`) by <@{}>{variant_note}\nPrompt: {}",
        request.pipeline, request.model, worker.discord_user_id, request.prompt
    );

    let message_id = match state
        .discord
        .post_candidate(state.config.discord_channel_id, png_bytes, sanitized_svg.clone().into_bytes(), &short_name, &caption)
        .await
    {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_GATEWAY, "failed to post to Discord").into_response(),
    };

    let insert_result = state.db.insert_candidate(
        &short_name,
        &request.prompt,
        &request.pipeline,
        &request.model,
        request.variant_of.as_deref(),
        worker.id,
        &message_id,
        &state.config.discord_channel_id.to_string(),
        &svg_path,
        &png_path,
    );

    if insert_result.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "posted to Discord but failed to save candidate record").into_response();
    }

    Json(SubmitResponse { short_name, message_id }).into_response()
}
