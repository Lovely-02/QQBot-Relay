mod api;
mod core;
mod data;
mod net;
mod util;
mod web;

use crate::core::config::AppConfig;
use crate::core::AppState;
use crate::data::accounts::AccountManager;
use crate::data::cache::CacheManager;
use crate::data::db::Database;
use crate::data::stats::StatsManager;
use crate::net::connections::ConnectionManager;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new("%Y/%m/%d %H:%M:%S".to_string()))
        .init();

    info!("QQBot-Relay v{}", env!("CARGO_PKG_VERSION"));

    // 数据库
    std::fs::create_dir_all("data")?;
    let db = Arc::new(Database::new("data/relay.db")?);

    // 核心服务
    let accounts = Arc::new(AccountManager::new(db.clone()));
    let cache = Arc::new(CacheManager::new(
        config.cache.max_public_messages,
        config.cache.max_token_messages,
        config.cache.message_ttl,
        config.deduplication_ttl,
        config.no_cache_secrets.clone(),
    ));
    let stats = Arc::new(StatsManager::new(db.clone()));
    let connections = Arc::new(ConnectionManager::new());

    let config = Arc::new(parking_lot::RwLock::new(config));

    let state = Arc::new(AppState {
        db: db.clone(),
        config: config.clone(),
        accounts,
        cache: cache.clone(),
        stats: stats.clone(),
        connections: connections.clone(),
    });

    // ── 后台任务 ──

    // 统计数据每 N 秒刷新一次
    let stats_bg = stats.clone();
    let flush_interval = config.read().stats.write_interval;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(flush_interval));
        loop {
            interval.tick().await;
            stats_bg.flush_to_db();
        }
    });

    // 缓存每 N 秒清理一次
    let cache_bg = cache.clone();
    let clean_interval = config.read().cache.clean_interval;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(clean_interval));
        loop {
            interval.tick().await;
            cache_bg.cleanup();
        }
    });

    // 会话每小时清理一次
    let db_bg = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            db_bg.cleanup_expired_sessions();
        }
    });

    // 通过文件系统监听器热重载配置
    let config_bg = config.clone();
    tokio::spawn(async move {
        watch_config("config.toml", config_bg).await;
    });

    // 健康监控每 30 秒检查一次
    let connections_bg = connections.clone();
    let cache_bg2 = cache.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        let mut error_count = 0i64;
        loop {
            interval.tick().await;
            let total = connections_bg.total_connections();
            if total == 0 && error_count > 10 {
                tracing::warn!("自动恢复: 清除缓存 (无连接, {} 次错误)", error_count);
                cache_bg2.cleanup();
                error_count = 0;
            }
        }
    });

    // ── 路由 ──

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let admin_routes = Router::new()
        .route("/stats", get(api::admin::get_stats))
        .route("/appids", get(api::admin::get_appids))
        .route("/appids/create", post(api::admin::create_appid))
        .route("/appids/:appid", delete(api::admin::delete_appid))
        .route("/settings", get(api::admin::get_settings))
        .route("/settings/update", post(api::admin::update_settings))
        .route("/webhook/list", get(api::admin::webhook_list))
        .route("/webhook/add", post(api::admin::webhook_add))
        .route("/webhook/remove", post(api::admin::webhook_remove))
        .route("/db/tables", get(api::admin::db_tables));

    let auth_routes = Router::new()
        .route("/login", post(api::auth::login))
        .route("/verify", get(api::auth::verify))
        .route("/logout", post(api::auth::logout));

    let api_routes = Router::new()
        .nest("/admin", admin_routes)
        .nest("/admin", auth_routes);

    let app = Router::new()
        .route("/", get(|| async { axum::response::Json(serde_json::json!({"message": "Webhook", "status": "ok"})) }))
        .route("/webhook", post(api::webhook::handle_webhook))
        .route("/api/:appid", post(api::webhook::handle_appid_webhook))
        .route("/ws/:secret", get(api::websocket::ws_by_secret))
        .route("/api/ws/:appid", get(api::websocket::ws_by_appid))
        .nest("/api", api_routes)
        .layer(cors)
        .with_state(state);

    // 提供嵌入式 Web 管理面板，支持 SPA 回退
    let web_router = axum::Router::new()
        .route("/", get(web::serve_web))
        .fallback(web::serve_web_rest);
    let app = app
        .route("/web", get(|| async { axum::response::Redirect::permanent("/web/") }))
        .nest("/web/", web_router);

    // ── 启动服务器 ──

    let port = config.read().port;
    let addr = format!("0.0.0.0:{}", port);

    let ssl_config = config.read().ssl.clone();
    if !ssl_config.ssl_certfile.is_empty() && !ssl_config.ssl_keyfile.is_empty() {
        info!("HTTPS 服务启动于 {}", addr);
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &ssl_config.ssl_certfile,
            &ssl_config.ssl_keyfile,
        )
        .await?;
        axum_server::bind_rustls(addr.parse()?, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        info!("HTTP 服务启动于 {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}

async fn watch_config(path: &str, config: Arc<parking_lot::RwLock<AppConfig>>) {
    use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel::<Event>(16);

    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        },
        NotifyConfig::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("创建配置监听器失败: {}", e);
            return;
        }
    };

    if let Err(e) = watcher.watch(path.as_ref(), RecursiveMode::NonRecursive) {
        tracing::error!("监听 {} 失败: {}", path, e);
        return;
    }

    info!("正在监听 {} 文件变更", path);

    let mut last_event = Instant::now();
    let debounce = Duration::from_millis(300);

    while let Some(event) = rx.recv().await {
        if !matches!(event.kind, EventKind::Modify(_)) {
            continue;
        }
        let now = Instant::now();
        if now.duration_since(last_event) < debounce {
            continue;
        }
        last_event = now;

        tokio::time::sleep(Duration::from_millis(100)).await;

        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<AppConfig>(&content) {
                Ok(new_config) => {
                    *config.write() = new_config;
                    info!("已热重载 {}", path);
                }
                Err(e) => {
                    tracing::warn!("重载 {} 解析失败: {}", path, e);
                }
            },
            Err(e) => {
                tracing::warn!("重载 {} 读取失败: {}", path, e);
            }
        }
    }
}
