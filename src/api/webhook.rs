use crate::core::AppState;
use crate::util::helpers;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::debug;

#[derive(Deserialize)]
pub struct WebhookQuery {
    pub secret: Option<String>,
    #[allow(dead_code)]
    pub appid: Option<String>,
    pub signature: Option<String>,
    pub timestamp: Option<String>,
    pub nonce: Option<String>,
}

/// POST /webhook?secret=密钥
#[axum::debug_handler]
pub async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WebhookQuery>,
    body: String,
) -> axum::response::Response {
    let secret = match query.secret {
        Some(s) => s,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "missing secret"}))).into_response(),
    };

    process_webhook(&state, &secret, &body, None).await
}

/// POST /api/{应用ID}
#[axum::debug_handler]
pub async fn handle_appid_webhook(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(appid): axum::extract::Path<String>,
    Query(query): Query<WebhookQuery>,
    body: String,
) -> axum::response::Response {
    let secret = match state.accounts.get_secret(&appid) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "appid not found"}))).into_response(),
    };

    // 如果提供了签名则验证 HMAC-SHA256
    if let (Some(sig), Some(ts), Some(nonce)) = (&query.signature, &query.timestamp, &query.nonce) {
        debug!("[Webhook验签] appid={}, signature={}, timestamp={}, nonce={}", appid, sig, ts, nonce);
        if !helpers::verify_signature(&secret, sig, ts, nonce, &body) {
            tracing::warn!("[Webhook验签失败] appid={}, 计算结果不匹配", appid);
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid signature"}))).into_response();
        }
    }

    process_webhook(&state, &secret, &body, Some(&appid)).await
}

async fn process_webhook(
    state: &AppState,
    secret: &str,
    body: &str,
    appid: Option<&str>,
) -> axum::response::Response {
    state.stats.increment_messages();

    // 如果未提供应用ID则从密钥解析
    let resolved_appid = match appid {
        Some(a) => Some(a.to_string()),
        None => state.db.get_appid_by_secret(secret),
    };

    // 在任何 await 之前提取配置值（parking_lot 守卫不是 Send）
    let (raw_enabled, raw_path) = {
        let config = state.config.read();
        (
            config.raw_content.enabled,
            config.raw_content.path.clone(),
        )
    };

    // 可选的原始内容日志记录
    if raw_enabled {
        let _ = std::fs::create_dir_all(&raw_path);
        let file_path = format!("{}/webhook_{}.log", raw_path, chrono::Utc::now().format("%Y%m%d"));
        let entry = format!("[{}] {}\n", chrono::Utc::now().to_rfc3339(), body);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(entry.as_bytes())
            });
    }

    // 解析 JSON
    let data: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid json"}))).into_response();
        }
    };

    // 消息去重
    if let Some(msg_id) = data.get("id").and_then(|v| v.as_str()) {
        if state.cache.is_duplicate(msg_id) {
            debug!("重复消息已忽略: {}", msg_id);
            return Json(serde_json::json!({"status": "success"})).into_response();
        }
        state.cache.add_msg_id(msg_id);
    }

    // Ed25519 签名验证回调（QQ 机器人平台握手）
    if let (Some(event_ts), Some(plain_token)) = (
        data.get("d").and_then(|d| d.get("event_ts")).and_then(|v| v.as_str()),
        data.get("d").and_then(|d| d.get("plain_token")).and_then(|v| v.as_str()),
    ) {
        let sig = helpers::generate_ed25519_signature(secret.as_bytes(), event_ts, plain_token);
        let sig_hex = hex::encode(sig);
        debug!("[Ed25519握手] secret={}, event_ts={}, plain_token={}, signature={}", secret, event_ts, plain_token, sig_hex);
        return Json(serde_json::json!({
            "plain_token": plain_token,
            "signature": sig_hex
        }))
        .into_response();
    }

    // 二级 Webhook 转发
    if let Some(ref fwd_appid) = resolved_appid {
        let urls = state.accounts.get_webhook_urls(fwd_appid);
        if !urls.is_empty() {
            let fwd_stats = state.stats.clone();
            let timeout = state.config.read().webhook_forward.timeout;
            state.connections.forward_webhook(&urls, fwd_appid, &data, timeout, fwd_stats).await;
        }
    }

    // 为 WebSocket 中继序列化载荷
    let payload = serde_json::to_vec(&data).unwrap_or_default();

    // 发送给 WebSocket 客户端或缓存
    state.connections.send_to_all(secret, &data, &payload, &state.stats, &state.cache).await;

    Json(serde_json::json!({"status": "success"})).into_response()
}
