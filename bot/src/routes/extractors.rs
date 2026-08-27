use crate::auth;
use crate::db::Worker;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};

pub struct AuthenticatedWorker(pub Worker);

impl FromRequestParts<AppState> for AuthenticatedWorker {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing Authorization header"))?;

        let token = header_value
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "Authorization header must be a Bearer token"))?;

        let worker = auth::authenticate(&state.db, token)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "auth lookup failed"))?
            .ok_or((StatusCode::UNAUTHORIZED, "invalid or revoked token"))?;

        Ok(AuthenticatedWorker(worker))
    }
}
