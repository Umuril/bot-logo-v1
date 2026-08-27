use crate::routes::extractors::AuthenticatedWorker;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

async fn serve_file(state: &AppState, short_name: &str, path_field: impl Fn(&crate::db::Candidate) -> &str, content_type: &str) -> Response {
    let candidate = match state.db.find_candidate_by_short_name(short_name) {
        Ok(Some(c)) => c,
        Ok(None) => return (StatusCode::NOT_FOUND, "no such candidate").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "lookup failed").into_response(),
    };

    match std::fs::read(path_field(&candidate)) {
        Ok(bytes) => ([(header::CONTENT_TYPE, content_type)], bytes).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "candidate file missing on disk").into_response(),
    }
}

pub async fn svg_handler(State(state): State<AppState>, AuthenticatedWorker(_worker): AuthenticatedWorker, Path(short_name): Path<String>) -> Response {
    serve_file(&state, &short_name, |c| &c.svg_path, "image/svg+xml").await
}

pub async fn png_handler(State(state): State<AppState>, AuthenticatedWorker(_worker): AuthenticatedWorker, Path(short_name): Path<String>) -> Response {
    serve_file(&state, &short_name, |c| &c.png_path, "image/png").await
}
