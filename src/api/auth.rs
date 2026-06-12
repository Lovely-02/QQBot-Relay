#![allow(dead_code)]
use crate::core::AppState;
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const MAX_SESSIONS: usize = 10;
const SESSION_MAX_AGE: u64 = 7 * 24 * 3600; // 7 天

fn sign_token(token: &str, password: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(password.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(token.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub(crate) fn session_cookie(token: &str, password: &str, secure: bool) -> String {
    let signed = format!("{}.{}", token, sign_token(token, password));
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "admin_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        signed, SESSION_MAX_AGE, secure
    )
}

fn expired_session_cookie(secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "admin_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        secure
    )
}

fn verify_cookie(cookie_value: &str, password: &str) -> Option<String> {
    let parts: Vec<&str> = cookie_value.splitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }
    let token = parts[0];
    let signature = hex::decode(parts[1]).ok()?;
    let mut mac = HmacSha256::new_from_slice(password.as_bytes()).ok()?;
    mac.update(token.as_bytes());
    if mac.verify_slice(&signature).is_ok() {
        Some(token.to_string())
    } else {
        None
    }
}

fn get_real_ip(headers: &HeaderMap, peer: SocketAddr, trust_proxy_headers: bool) -> String {
    if trust_proxy_headers {
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return ip.to_string();
        }
        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return ip.to_string();
        }
    }
    peer.ip().to_string()
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
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let admin = state.config.read().admin.clone();
    if !admin.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "admin disabled"})),
        )
            .into_response();
    }
    let ip = get_real_ip(&headers, peer, admin.trust_proxy_headers);

    if check_ip_banned(&state, &ip) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "IP banned"})),
        )
            .into_response();
    }

    if password != admin.password {
        // 记录失败
        let fail_times = state
            .db
            .get_ip_access(&ip)
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
    state.db.limit_sessions(MAX_SESSIONS);

    // 重置 IP 失败计数
    state
        .db
        .update_ip_access(ip, "[]".into(), false, String::new());

    let secure = !state.config.read().ssl.ssl_certfile.is_empty();
    let cookie = session_cookie(&token, &admin.password, secure);

    let mut response = Json(serde_json::json!({"status": "success"})).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    response
}

pub async fn verify(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    match extract_session(&state, &headers) {
        Some(_) => Json(serde_json::json!({"valid": true})).into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"valid": false})),
        )
            .into_response(),
    }
}

pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(token) = get_session_token(&state, &headers) {
        state.db.delete_session(&token);
    }
    let secure = !state.config.read().ssl.ssl_certfile.is_empty();
    let mut response = Json(serde_json::json!({"status": "success"})).into_response();
    response.headers_mut().insert(
        "set-cookie",
        expired_session_cookie(secure).parse().unwrap(),
    );
    response
}

fn get_session_token(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get("cookie")?.to_str().ok()?;
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("admin_session=") {
            return verify_cookie(val, &state.config.read().admin.password);
        }
    }
    None
}

pub fn extract_session(state: &AppState, headers: &HeaderMap) -> Option<String> {
    if !state.config.read().admin.enabled {
        return None;
    }
    let token = get_session_token(state, headers)?;
    let now = chrono::Utc::now();
    if let Some(session) = state.db.get_session(&token) {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&session.expires) {
            if now < expires {
                return Some(token);
            }
        }
    }
    None
}

pub fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<String, StatusCode> {
    extract_session(state, headers).ok_or(StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_signature_depends_on_admin_password() {
        let token = "session-token";
        let cookie = format!("{}.{}", token, sign_token(token, "password-a"));

        assert_eq!(
            verify_cookie(&cookie, "password-a"),
            Some(token.to_string())
        );
        assert_eq!(verify_cookie(&cookie, "password-b"), None);
    }

    #[test]
    fn session_cookie_uses_consistent_security_attributes() {
        let cookie = session_cookie("session-token", "password", true);
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Secure"));

        let expired = expired_session_cookie(true);
        assert!(expired.contains("Max-Age=0"));
        assert!(expired.contains("SameSite=Lax"));
        assert!(expired.contains("Secure"));
    }
}
