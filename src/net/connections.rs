#![allow(dead_code)]
use crate::data::cache::CacheManager;
use crate::data::stats::StatsManager;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{info, warn};

const HELLO_PAYLOAD: &str = r#"{"op":10,"d":{"heartbeat_interval":30000}}"#;
const HB_ACK: &str = r#"{"op":11}"#;
const READY_PAYLOAD: &str = r#"{"op":0,"s":1,"t":"READY","d":{"version":1,"session_id":"open-connection","user":{"bot":true},"shard":[0,0]}}"#;
const RESUMED_PAYLOAD: &str = r#"{"op":0,"s":1,"t":"RESUMED","d":{}}"#;

pub struct ConnInfo {
    pub token: Option<String>,
    pub failure_count: AtomicI64,
    pub group: Option<String>,
    pub member: Option<String>,
    pub content: Option<String>,
    pub is_sandbox: bool,
    pub last_activity: AtomicI64,
}

type WsSender = futures_util::stream::SplitSink<
    axum::extract::ws::WebSocket,
    axum::extract::ws::Message,
>;

pub struct ConnectionManager {
    pub tx: broadcast::Sender<(String, Vec<u8>)>,
    conns: DashMap<String, DashMap<usize, ConnInfo>>,
    next_id: AtomicI64,
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            tx,
            conns: DashMap::new(),
            next_id: AtomicI64::new(0),
        }
    }

    pub fn has_connections(&self, secret: &str) -> bool {
        self.conns
            .get(secret)
            .map(|entry| !entry.is_empty())
            .unwrap_or(false)
    }

    pub fn next_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::Relaxed) as usize
    }

    pub fn register(
        &self,
        secret: &str,
        id: usize,
        token: Option<String>,
        group: Option<String>,
        member: Option<String>,
        content: Option<String>,
    ) {
        let is_sandbox = group.is_some() || member.is_some() || content.is_some();
        let info = ConnInfo {
            token,
            failure_count: AtomicI64::new(0),
            group,
            member,
            content,
            is_sandbox,
            last_activity: AtomicI64::new(now_ts()),
        };
        self.conns
            .entry(secret.to_string())
            .or_insert_with(DashMap::new)
            .insert(id, info);
    }

    pub fn unregister(&self, secret: &str, id: usize) {
        if let Some(entry) = self.conns.get(secret) {
            entry.remove(&id);
            if entry.is_empty() {
                drop(entry);
                self.conns.remove(secret);
            }
        }
    }

    pub fn connection_count(&self, secret: &str) -> usize {
        self.conns
            .get(secret)
            .map(|entry| entry.len())
            .unwrap_or(0)
    }

    pub fn total_connections(&self) -> usize {
        self.conns.iter().map(|entry| entry.len()).sum()
    }

    pub fn get_token(&self, secret: &str, id: usize) -> Option<String> {
        self.conns
            .get(secret)
            .and_then(|entry| entry.get(&id).map(|c| c.token.clone()))
            .flatten()
    }

    pub fn check_sandbox(&self, secret: &str, id: usize, data: &serde_json::Value) -> bool {
        self.conns
            .get(secret)
            .and_then(|entry| {
                entry.get(&id).map(|info| {
                    if !info.is_sandbox {
                        return true;
                    }
                    if let Some(ref group) = info.group {
                        if data.get("group_openid").and_then(|v| v.as_str()) != Some(group.as_str()) {
                            return false;
                        }
                    }
                    if let Some(ref member) = info.member {
                        let actual = data
                            .get("author")
                            .and_then(|a| a.get("member_openid"))
                            .and_then(|v| v.as_str());
                        if actual != Some(member.as_str()) {
                            return false;
                        }
                    }
                    if let Some(ref content_filter) = info.content {
                        let msg_content = data.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        if !msg_content.contains(content_filter.as_str()) {
                            return false;
                        }
                    }
                    true
                })
            })
            .unwrap_or(true)
    }

    pub fn record_failure(&self, secret: &str, id: usize) -> bool {
        self.conns
            .get(secret)
            .and_then(|entry| {
                entry.get(&id).map(|info| {
                    info.failure_count.fetch_add(1, Ordering::Relaxed) + 1 >= 5
                })
            })
            .unwrap_or(false)
    }

    pub fn update_activity(&self, secret: &str, id: usize) {
        if let Some(entry) = self.conns.get(secret) {
            if let Some(info) = entry.get(&id) {
                info.last_activity.store(now_ts(), Ordering::Relaxed);
            }
        }
    }

    pub async fn send_to_all(
        &self,
        secret: &str,
        data: &serde_json::Value,
        payload: &[u8],
        stats: &StatsManager,
        cache: &CacheManager,
    ) {
        if !self.has_connections(secret) {
            if cache.should_cache(secret) {
                cache.add_public(secret, payload.to_vec());
            }
            return;
        }

        let mut success = 0i64;
        let mut failure = 0i64;

        // Collect connection IDs to avoid borrow issues
        let ids: Vec<usize> = self
            .conns
            .get(secret)
            .map(|entry| entry.iter().map(|r| *r.key()).collect())
            .unwrap_or_default();

        for id in ids {
            if !self.check_sandbox(secret, id, data) {
                continue;
            }
            if self.tx.send((secret.to_string(), payload.to_vec())).is_ok() {
                success += 1;
            } else {
                failure += 1;
                if self.record_failure(secret, id) {
                    warn!("连接 {} (密钥 '{}') 超过失败上限, 已断开", id, secret);
                    self.unregister(secret, id);
                }
            }
        }

        stats.update_ws_stats(secret, success, failure);
    }

    pub async fn handle_connection(
        self: Arc<Self>,
        secret: String,
        ws: axum::extract::ws::WebSocket,
        cache: Arc<CacheManager>,
        _stats: Arc<StatsManager>,
        token: Option<String>,
        group: Option<String>,
        member: Option<String>,
        content: Option<String>,
    ) {
        let (mut ws_tx, mut ws_rx) = ws.split();
        let id = self.next_id();

        // Send HELLO
        if ws_tx.send(axum::extract::ws::Message::Text(HELLO_PAYLOAD.into())).await.is_err() {
            return;
        }

        self.register(&secret, id, token.clone(), group, member, content);
        info!("WebSocket 已连接: 密钥='{}', id={}, token={:?}", secret, id, token);

        // Subscribe to broadcast
        let mut rx = self.tx.subscribe();

        // Cache resend after 3s delay
        let secret_clone = secret.clone();
        let cache_clone = cache.clone();
        let token_clone = token.clone();
        let self_clone = self.clone();

        let cache_resend_handle: JoinHandle<()> = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            // Resend public cache
            let public_msgs = cache_clone.drain_public(&secret_clone);
            if !public_msgs.is_empty() {
                info!("重发 {} 条公共缓存消息 '{}'", public_msgs.len(), secret_clone);
                for (i, chunk) in public_msgs.chunks(10).enumerate() {
                    if i > 0 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    for _msg in chunk {
                        // Messages are broadcast via the channel
                        let _ = self_clone.tx.send((secret_clone.clone(), _msg.clone()));
                    }
                }
            }

            // Resend token cache
            if let Some(ref t) = token_clone {
                let token_msgs = cache_clone.drain_token(&secret_clone, t);
                if !token_msgs.is_empty() {
                    info!("重发 {} 条令牌缓存消息 '{}'", token_msgs.len(), secret_clone);
                    for (i, chunk) in token_msgs.chunks(10).enumerate() {
                        if i > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                        for _msg in chunk {
                            let _ = self_clone.tx.send((secret_clone.clone(), _msg.clone()));
                        }
                    }
                }
            }
        });

        // Heartbeat task
        let secret_hb = secret.clone();
        let self_hb = self.clone();
        let heartbeat_handle: JoinHandle<()> = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(35));
            let mut failures = 0i64;
            loop {
                interval.tick().await;
                if self_hb.tx.send((secret_hb.clone(), HB_ACK.as_bytes().to_vec())).is_err() {
                    failures += 1;
                    if failures >= 3 {
                        break;
                    }
                } else {
                    failures = 0;
                }
            }
        });

        // Forward broadcast messages to this WebSocket
        let secret_fwd = secret.clone();
        let fwd_handle: JoinHandle<()> = tokio::spawn(async move {
            while let Ok((msg_secret, payload)) = rx.recv().await {
                if msg_secret != secret_fwd {
                    continue;
                }
                if ws_tx.send(axum::extract::ws::Message::Binary(payload)).await.is_err() {
                    break;
                }
            }
        });

        // Receive loop
        let idle_timeout = std::time::Duration::from_secs(90);
        loop {
            match tokio::time::timeout(idle_timeout, ws_rx.next()).await {
                Ok(Some(Ok(msg))) => {
                    self.update_activity(&secret, id);
                    match msg {
                        axum::extract::ws::Message::Text(text) => {
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                                self.handle_client_message(&data, &secret);
                            }
                        }
                        axum::extract::ws::Message::Close(_) => break,
                        _ => {}
                    }
                }
                Ok(Some(Err(_))) => break,
                Ok(None) => break,
                Err(_) => {
                    warn!("WebSocket {} ('{}') 空闲超时", id, secret);
                    break;
                }
            }
        }

        // Cleanup
        cache_resend_handle.abort();
        heartbeat_handle.abort();
        fwd_handle.abort();
        self.unregister(&secret, id);
        info!("WebSocket 已断开: 密钥='{}', id={}", secret, id);
    }

    fn handle_client_message(&self, data: &serde_json::Value, secret: &str) {
        if let Some(op) = data.get("op").and_then(|v| v.as_i64()) {
            match op {
                1 => { /* heartbeat -> HB_ACK already sent by heartbeat task */ }
                2 => { /* identify -> READY */
                    let _ = self.tx.send((secret.to_string(), READY_PAYLOAD.as_bytes().to_vec()));
                }
                6 => { /* resume -> RESUMED */
                    let _ = self.tx.send((secret.to_string(), RESUMED_PAYLOAD.as_bytes().to_vec()));
                }
                _ => {}
            }
        }
    }

    pub async fn forward_webhook(
        &self,
        urls: &[String],
        appid: &str,
        body: &serde_json::Value,
        _timeout: u64,
        stats: Arc<StatsManager>,
    ) {
        if urls.is_empty() {
            return;
        }

        let body_str = serde_json::to_string(body).unwrap_or_default();
        let client = reqwest::Client::new();
        let max_retry = 180u64;
        let retry_interval = std::time::Duration::from_secs(1);
        let push_timeout = std::time::Duration::from_secs(10);

        for url in urls {
            let url = url.clone();
            let body_clone = body_str.clone();
            let client_clone = client.clone();
            let secret = appid.to_string();
            let stats_clone = stats.clone();

            tokio::spawn(async move {
                let start = SystemTime::now();
                let mut attempts = 0i64;

                loop {
                    attempts += 1;
                    match client_clone
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .body(body_clone.clone())
                        .timeout(push_timeout)
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            let duration = start.elapsed().unwrap_or_default();
                            tracing::debug!(
                                "Webhook 转发成功: {} -> {} ({}ms, {} 次尝试)",
                                secret, url, duration.as_millis(), attempts
                            );
                            stats_clone.update_wh_stats(&secret, 1, 0);
                            break;
                        }
                        _ => {
                            if start.elapsed().unwrap_or_default().as_secs() >= max_retry {
                                tracing::warn!(
                                    "Webhook 转发失败 ({}s): {} -> {}",
                                    max_retry, secret, url
                                );
                                stats_clone.update_wh_stats(&secret, 0, 1);
                                break;
                            }
                            tokio::time::sleep(retry_interval).await;
                        }
                    }
                }
            });
        }
    }
}
