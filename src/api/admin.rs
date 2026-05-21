#![allow(dead_code)]
use crate::api::auth;
use crate::core::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<String, StatusCode> {
    auth::require_auth(state, headers)
}

/// 获取全局统计信息
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let global = state.stats.get_global();
    let ws_online = state.connections.total_connections();
    let accounts = state.accounts.get_all().len();

    Json(serde_json::json!({
        "global": global,
        "ws_online": ws_online,
        "accounts": accounts,
    }))
    .into_response()
}

/// 获取所有账号列表
pub async fn get_appids(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    Json(state.accounts.get_all()).into_response()
}

/// 创建新账号
pub async fn create_appid(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let appid = match body.get("appid").and_then(|v| v.as_str()) {
        Some(a) => a.to_string(),
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "missing appid"}))).into_response(),
    };
    let secret = match body.get("secret").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "missing secret"}))).into_response(),
    };
    let description = body.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

    state.accounts.create(appid, secret, description);
    Json(serde_json::json!({"status": "created"})).into_response()
}

/// 删除指定账号
pub async fn delete_appid(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(appid): axum::extract::Path<String>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    if state.accounts.delete(&appid) {
        Json(serde_json::json!({"status": "deleted"})).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response()
    }
}

/// 获取当前配置
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let config = state.config.read().clone();
    Json(serde_json::to_value(&config).unwrap_or_default()).into_response()
}

/// 更新配置
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let mut config = state.config.read().clone();

    if let Some(port) = body.get("port").and_then(|v| v.as_u64()) {
        config.port = port as u16;
    }
    if let Some(ttl) = body.get("deduplication_ttl").and_then(|v| v.as_u64()) {
        config.deduplication_ttl = ttl;
    }
    if let Some(level) = body.get("log_level").and_then(|v| v.as_str()) {
        config.log_level = level.to_string();
    }
    if let Some(list) = body.get("no_cache_secrets").and_then(|v| v.as_array()) {
        config.no_cache_secrets = list.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    }

    if let Some(admin) = body.get("admin") {
        if let Some(pwd) = admin.get("password").and_then(|v| v.as_str()) {
            config.admin.password = pwd.to_string();
        }
        if let Some(enabled) = admin.get("enabled").and_then(|v| v.as_bool()) {
            config.admin.enabled = enabled;
        }
    }

    if let Some(cache) = body.get("cache") {
        if let Some(v) = cache.get("max_public_messages").and_then(|v| v.as_u64()) {
            config.cache.max_public_messages = v as usize;
        }
        if let Some(v) = cache.get("max_token_messages").and_then(|v| v.as_u64()) {
            config.cache.max_token_messages = v as usize;
        }
        if let Some(v) = cache.get("message_ttl").and_then(|v| v.as_u64()) {
            config.cache.message_ttl = v;
        }
        if let Some(v) = cache.get("clean_interval").and_then(|v| v.as_u64()) {
            config.cache.clean_interval = v;
        }
    }

    if let Some(fwd) = body.get("webhook_forward") {
        if let Some(v) = fwd.get("timeout").and_then(|v| v.as_u64()) {
            config.webhook_forward.timeout = v;
        }
    }

    if let Some(raw) = body.get("raw_content") {
        if let Some(v) = raw.get("enabled").and_then(|v| v.as_bool()) {
            config.raw_content.enabled = v;
        }
        if let Some(v) = raw.get("path").and_then(|v| v.as_str()) {
            config.raw_content.path = v.to_string();
        }
    }

    config.save();
    *state.config.write() = config;

    Json(serde_json::json!({"status": "updated"})).into_response()
}

/// 获取所有 Webhook 转发目标
pub async fn webhook_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let all = state.db.get_all_webhook_targets();
    let result: Vec<serde_json::Value> = all.into_iter().map(|(appid, url)| {
        serde_json::json!({ "appid": appid, "url": url })
    }).collect();
    Json(result).into_response()
}

/// 添加 Webhook 转发目标
pub async fn webhook_add(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let appid = match body.get("appid").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "missing appid"}))).into_response(),
    };
    let url = match body.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "missing url"}))).into_response(),
    };

    // 检查是否已存在（去重由数据库主键处理）
    let existing = state.accounts.get_webhook_urls(appid);
    if existing.contains(&url.to_string()) {
        return Json(serde_json::json!({"status": "already_exists"})).into_response();
    }

    state.accounts.add_webhook_target(appid, url);
    Json(serde_json::json!({"status": "added"})).into_response()
}

/// 移除 Webhook 转发目标
pub async fn webhook_remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let appid = body.get("appid").and_then(|v| v.as_str()).unwrap_or("");
    let url = body.get("url").and_then(|v| v.as_str()).unwrap_or("");

    state.accounts.remove_webhook_target(appid, url);
    Json(serde_json::json!({"status": "removed"})).into_response()
}

/// 查看数据库表内容
pub async fn db_tables(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if require_auth(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let tables = vec!["accounts", "sessions", "ip_access", "stats_global", "stats_per_secret", "webhook_targets"];
    let mut result = Vec::new();
    for table in tables {
        let rows = state.db.query_table(table);
        result.push(serde_json::json!({
            "table": table,
            "row_count": rows.len(),
            "rows": rows,
        }));
    }
    Json(result).into_response()
}
