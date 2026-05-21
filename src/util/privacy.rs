#![allow(dead_code)]
use regex::Regex;
use std::sync::LazyLock;

static SECRET_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"secret=[^&\s]+").unwrap());
static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"token=[^&\s]+").unwrap());
static KEY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"key=[^&\s]+").unwrap());
static PASSWORD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"password=[^&\s]+").unwrap());
static SK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"sk-[a-zA-Z0-9]{8,}").unwrap());
static BEARER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Bearer\s+[a-zA-Z0-9._-]{8,}").unwrap());

pub fn sanitize_secret(s: &str) -> String {
    if s.len() <= 2 {
        return "***".into();
    }
    format!("{}***", &s[..2])
}

pub fn sanitize_ip(ip: &str) -> String {
    if ip.contains('.') {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            return format!("{}.***.***.{}", parts[0], parts[3]);
        }
    }
    if ip.contains(':') {
        let parts: Vec<&str> = ip.split(':').collect();
        if parts.len() >= 2 {
            return format!("{}:***:***:{}", parts[0], parts.last().unwrap());
        }
    }
    ip.to_string()
}

pub fn sanitize_path(path: &str) -> String {
    let mut result = SECRET_RE.replace_all(path, "secret=***").to_string();
    result = TOKEN_RE.replace_all(&result, "token=***").to_string();
    result = KEY_RE.replace_all(&result, "key=***").to_string();
    result = PASSWORD_RE.replace_all(&result, "password=***").to_string();
    result
}

pub fn sanitize_logs(msg: &str) -> String {
    let result = SK_RE.replace_all(msg, "sk-***");
    BEARER_RE.replace_all(&result, "Bearer ***").to_string()
}
