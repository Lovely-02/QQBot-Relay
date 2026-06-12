use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub ssl: SslConfig,
    #[serde(default)]
    pub admin: AdminConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub stats: StatsConfig,
    #[serde(default)]
    pub raw_content: RawContentConfig,
    #[serde(default)]
    pub webhook_forward: WebhookForwardConfig,
    #[serde(default)]
    pub deduplication_ttl: u64,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub no_cache_secrets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SslConfig {
    pub ssl_keyfile: String,
    pub ssl_certfile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default = "default_admin_password")]
    pub password: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub trust_proxy_headers: bool,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            password: default_admin_password(),
            enabled: true,
            trust_proxy_headers: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_max_public")]
    pub max_public_messages: usize,
    #[serde(default = "default_max_token")]
    pub max_token_messages: usize,
    #[serde(default = "default_message_ttl")]
    pub message_ttl: u64,
    #[serde(default = "default_clean_interval")]
    pub clean_interval: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_public_messages: default_max_public(),
            max_token_messages: default_max_token(),
            message_ttl: default_message_ttl(),
            clean_interval: default_clean_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsConfig {
    #[serde(default = "default_stats_interval")]
    pub write_interval: u64,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            write_interval: default_stats_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawContentConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_raw_path")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookForwardConfig {
    #[serde(default = "default_forward_timeout")]
    pub timeout: u64,
}

impl Default for WebhookForwardConfig {
    fn default() -> Self {
        Self {
            timeout: default_forward_timeout(),
        }
    }
}

fn default_port() -> u16 {
    8000
}
fn default_log_level() -> String {
    "info".into()
}
fn default_admin_password() -> String {
    String::new()
}
fn default_true() -> bool {
    true
}
fn default_max_public() -> usize {
    1000
}
fn default_max_token() -> usize {
    500
}
fn default_message_ttl() -> u64 {
    300
}
fn default_clean_interval() -> u64 {
    120
}
fn default_stats_interval() -> u64 {
    5
}
fn default_raw_path() -> String {
    "logs".into()
}
fn default_forward_timeout() -> u64 {
    5
}

const CONFIG_PATH: &str = "config.toml";

impl AppConfig {
    pub fn load() -> Self {
        if Path::new(CONFIG_PATH).exists() {
            match std::fs::read_to_string(CONFIG_PATH) {
                Ok(content) => match toml::from_str::<AppConfig>(&content) {
                    Ok(mut config) => {
                        config.ensure_secure_admin_password();
                        info!("已加载配置文件 {}", CONFIG_PATH);
                        return config;
                    }
                    Err(e) => tracing::warn!("解析 config.toml 失败: {}, 使用默认配置", e),
                },
                Err(e) => tracing::warn!("读取 config.toml 失败: {}, 使用默认配置", e),
            }
        }
        let mut config = AppConfig::default();
        config.ensure_secure_admin_password();
        config.save();
        info!("已创建默认 config.toml");
        config
    }

    pub fn save(&self) {
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(CONFIG_PATH, content);
        }
    }

    fn ensure_secure_admin_password(&mut self) {
        if self.admin.password.is_empty() || self.admin.password == "admin" {
            use rand::{distributions::Alphanumeric, Rng};

            self.admin.password = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(24)
                .map(char::from)
                .collect();
            self.save();
            eprintln!(
                "管理员密码为空或不安全，已生成一次性初始密码: {}",
                self.admin.password
            );
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            ssl: SslConfig::default(),
            admin: AdminConfig::default(),
            cache: CacheConfig::default(),
            stats: StatsConfig::default(),
            raw_content: RawContentConfig::default(),
            webhook_forward: WebhookForwardConfig::default(),
            deduplication_ttl: 20,
            log_level: default_log_level(),
            no_cache_secrets: Vec::new(),
        }
    }
}
