#![allow(dead_code)]
use crate::core::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const COOKIE_SECRET: &str = "QQBot-Relay-cookie-secret-change-me";
const MAX_SESSIONS: usize = 10;
const SESSION_MAX_AGE: u64 = 7 * 24 * 3600; // 7 天

fn sign_token(token: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(COOKIE_SECRET.as_bytes()).unwrap();
    mac.update(token.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn verify_cookie(cookie_value: &str) -> Option<String> {
    let parts: Vec<&str> = cookie_value.splitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }
    let token = parts[0];
    let sig = parts[1];
    let expected = sign_token(token);
    if sig == expected {
        Some(token.to_string())
    } else {
        None
    }
}

fn get_real_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').next().unwrap_or(v).trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

fn check_ip_banned(state: &AppState, ip: &str) -> bool {
    if let Some((fail_times_str, is_banned)) = state.db.get_ip_access(ip) {
        if is_banned {
            return true;
        }
        let fails: Vec<f64> = serde_json::from_str(&fail_times_str).unwrap_or_default();
        let now = chrono::Utc::now().timestamp() as f64;
        let recent: Vec<_> = fails.into_iter().filter(|&t| now - t < 86400.0).collect();
        if recent.len() >= 5 {
            state.db.update_ip_access(
                ip.to_string(),
                serde_json::to_string(&recent).unwrap(),
                true,
                chrono::Utc::now().to_rfc3339(),
            );
            return true;
        }
    }
    false
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let ip = get_real_ip(&headers);

    if check_ip_banned(&state, &ip) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "IP banned"})),
        )
            .into_response();
    }

    if password != state.config.read().admin.password {
        // 记录失败
        let fail_times = state.db.get_ip_access(&ip)
            .map(|(s, _)| {
                let mut v: Vec<f64> = serde_json::from_str(&s).unwrap_or_default();
                v.push(chrono::Utc::now().timestamp() as f64);
                v
            })
            .unwrap_or_else(|| vec![chrono::Utc::now().timestamp() as f64]);
        state.db.update_ip_access(
            ip.clone(),
            serde_json::to_string(&fail_times).unwrap(),
            false,
            String::new(),
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid password"})),
        )
            .into_response();
    }

    // 创建会话
    let token = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::seconds(SESSION_MAX_AGE as i64);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    state.db.save_session(crate::data::db::Session {
        token: token.clone(),
        created: now.to_rfc3339(),
        expires: expires.to_rfc3339(),
        ip: ip.clone(),
        user_agent,
    });

    // 重置 IP 失败计数
    state.db.update_ip_access(ip, "[]".into(), false, String::new());

    let signed = format!("{}.{}", token, sign_token(&token));
    let cookie = format!(
        "admin_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        signed, SESSION_MAX_AGE
    );

    let mut response = Json(serde_json::json!({"status": "success"})).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    response
}

pub async fn verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match extract_session(&state, &headers) {
        Some(_) => Json(serde_json::json!({"valid": true})).into_response(),
        None => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"valid": false}))).into_response(),
    }
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(token) = get_session_token(&headers) {
        state.db.delete_session(&token);
    }
    let mut response = Json(serde_json::json!({"status": "success"})).into_response();
    response.headers_mut().insert(
        "set-cookie",
        "admin_session=; Path=/; HttpOnly; Max-Age=0".parse().unwrap(),
    );
    response
}

fn get_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get("cookie")?.to_str().ok()?;
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("admin_session=") {
            return verify_cookie(val);
        }
    }
    None
}

pub fn extract_session(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let token = get_session_token(headers)?;
    // 检查会话是否存在于数据库中
    let sessions = state.db.get_all_sessions();
    let now = chrono::Utc::now();
    for session in sessions {
        if session.token == token {
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&session.expires) {
                if now < expires {
                    return Some(token);
                }
            }
        }
    }
    None
}

pub fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<String, StatusCode> {
    extract_session(state, headers).ok_or(StatusCode::UNAUTHORIZED)
}
