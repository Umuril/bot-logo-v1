use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct InteractionPayload {
    #[serde(rename = "type")]
    interaction_type: u8,
    data: Option<CommandData>,
    user: Option<InteractionUser>,
    member: Option<InteractionMember>,
}

#[derive(Debug, Deserialize)]
struct CommandData {
    name: String,
    options: Option<Vec<CommandOption>>,
}

#[derive(Debug, Deserialize)]
struct CommandOption {
    value: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct InteractionUser {
    id: String,
}

#[derive(Debug, Deserialize)]
struct InteractionMember {
    user: InteractionUser,
}

const TYPE_PING: u8 = 1;
const TYPE_APPLICATION_COMMAND: u8 = 2;

pub async fn handler(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    let signature = headers.get("X-Signature-Ed25519").and_then(|v| v.to_str().ok()).unwrap_or("");
    let timestamp = headers.get("X-Signature-Timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");

    if !crate::signature::verify(&state.config.discord_public_key, signature, timestamp, &body) {
        return (StatusCode::UNAUTHORIZED, "invalid request signature").into_response();
    }

    let Ok(payload) = serde_json::from_slice::<InteractionPayload>(&body) else {
        return (StatusCode::BAD_REQUEST, "malformed interaction payload").into_response();
    };

    if payload.interaction_type == TYPE_PING {
        return Json(json!({ "type": 1 })).into_response();
    }

    if payload.interaction_type != TYPE_APPLICATION_COMMAND {
        return (StatusCode::BAD_REQUEST, "unsupported interaction type").into_response();
    }

    let Some(data) = payload.data else {
        return (StatusCode::BAD_REQUEST, "missing command data").into_response();
    };

    let invoking_user_id = payload.user.map(|u| u.id).or_else(|| payload.member.map(|m| m.user.id));
    let Some(invoking_user_id) = invoking_user_id else {
        return (StatusCode::BAD_REQUEST, "missing invoking user").into_response();
    };

    match data.name.as_str() {
        "token" => handle_token_command(&state, &invoking_user_id).await,
        "revoke" => handle_revoke_command(&state, &data).await,
        _ => (StatusCode::BAD_REQUEST, "unknown command").into_response(),
    }
}

fn ephemeral_reply(content: &str) -> Json<Value> {
    Json(json!({ "type": 4, "data": { "content": content, "flags": 64 } })) // flags: 64 = EPHEMERAL
}

async fn handle_token_command(state: &AppState, invoking_user_id: &str) -> axum::response::Response {
    let Ok(user_id) = invoking_user_id.parse::<u64>() else {
        return (StatusCode::BAD_REQUEST, "malformed user id").into_response();
    };

    let has_role = state
        .discord
        .has_role(state.config.discord_guild_id, user_id, state.config.discord_allowed_role_id)
        .await
        .unwrap_or(false);

    if !has_role {
        return ephemeral_reply("You need the logo-team role on the server to get a worker token.").into_response();
    }

    match crate::auth::issue_token(&state.db, invoking_user_id) {
        Ok(token) => ephemeral_reply(&format!(
            "Your worker token (put this in `worker.env` as `WORKER_TOKEN`):\n```\n{token}\n```\nRunning `/token` again will invalidate this one."
        ))
        .into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "failed to issue token").into_response(),
    }
}

async fn handle_revoke_command(state: &AppState, data: &CommandData) -> axum::response::Response {
    let Some(target_user_id) = data
        .options
        .as_ref()
        .and_then(|opts| opts.first())
        .and_then(|opt| opt.value.as_ref())
        .and_then(|v| v.as_str())
    else {
        return (StatusCode::BAD_REQUEST, "missing target user option").into_response();
    };

    match state.db.revoke_worker(target_user_id) {
        Ok(true) => ephemeral_reply("Token revoked.").into_response(),
        Ok(false) => ephemeral_reply("That user has no active token.").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "failed to revoke token").into_response(),
    }
}
