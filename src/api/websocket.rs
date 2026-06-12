use crate::core::AppState;
use crate::net::connections::{ConnectionManager, ConnectionOptions};
use crate::util::helpers;
use axum::{
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
    pub group: Option<String>,
    pub member: Option<String>,
    pub content: Option<String>,
    pub signature: Option<String>,
    pub timestamp: Option<String>,
    pub nonce: Option<String>,
}

/// WebSocket /ws/{密钥}?token=...&group=...&member=...&content=...
pub async fn ws_by_secret(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(secret): axum::extract::Path<String>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let conns = state.connections.clone();
    let cache = state.cache.clone();
    let stats = state.stats.clone();
    ws.on_upgrade(move |socket| {
        ConnectionManager::handle_connection(
            conns,
            secret,
            socket,
            cache,
            stats,
            ConnectionOptions {
                token: query.token,
                group: query.group,
                member: query.member,
                content: query.content,
            },
        )
    })
}

/// WebSocket /api/ws/{应用ID}?token=...&group=...&member=...&content=...&signature=...&timestamp=...&nonce=...
pub async fn ws_by_appid(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(appid): axum::extract::Path<String>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let secret = match state.accounts.get_secret(&appid) {
        Some(s) => s,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let (sig, ts, nonce) = match (&query.signature, &query.timestamp, &query.nonce) {
        (Some(sig), Some(ts), Some(nonce)) => (sig, ts, nonce),
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if !helpers::verify_signature(&secret, sig, ts, nonce, "") {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let conns = state.connections.clone();
    let cache = state.cache.clone();
    let stats = state.stats.clone();
    ws.on_upgrade(move |socket| {
        ConnectionManager::handle_connection(
            conns,
            secret,
            socket,
            cache,
            stats,
            ConnectionOptions {
                token: query.token,
                group: query.group,
                member: query.member,
                content: query.content,
            },
        )
    })
}
