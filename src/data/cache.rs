#![allow(dead_code)]
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

struct CachedMessage {
    expiry: f64,
    data: Vec<u8>,
}

struct SecretCache {
    public: VecDeque<CachedMessage>,
    tokens: HashMap<String, VecDeque<CachedMessage>>,
}

pub struct CacheManager {
    max_public: usize,
    max_token: usize,
    message_ttl: f64,
    dedup_ttl: f64,
    no_cache_secrets: Vec<String>,
    caches: RwLock<HashMap<String, SecretCache>>,
    dedup: RwLock<HashMap<String, f64>>,
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

impl CacheManager {
    pub fn new(
        max_public: usize,
        max_token: usize,
        message_ttl: u64,
        dedup_ttl: u64,
        no_cache_secrets: Vec<String>,
    ) -> Self {
        Self {
            max_public,
            max_token,
            message_ttl: message_ttl as f64,
            dedup_ttl: dedup_ttl as f64,
            no_cache_secrets,
            caches: RwLock::new(HashMap::new()),
            dedup: RwLock::new(HashMap::new()),
        }
    }

    pub fn should_cache(&self, secret: &str) -> bool {
        !self.no_cache_secrets.contains(&secret.to_string())
    }

    // ── 消息去重 ──

    pub fn is_duplicate(&self, msg_id: &str) -> bool {
        let now = now();
        if let Some(&expiry) = self.dedup.read().get(msg_id) {
            if expiry > now {
                return true;
            }
        }
        false
    }

    pub fn add_msg_id(&self, msg_id: &str) {
        let expiry = now() + self.dedup_ttl;
        self.dedup.write().insert(msg_id.to_string(), expiry);
    }

    // ── 公共缓存 ──

    pub fn add_public(&self, secret: &str, data: Vec<u8>) {
        let now = now();
        let mut caches = self.caches.write();
        let entry = caches.entry(secret.to_string()).or_insert_with(|| SecretCache {
            public: VecDeque::new(),
            tokens: HashMap::new(),
        });
        if entry.public.len() >= self.max_public {
            entry.public.pop_front();
        }
        entry.public.push_back(CachedMessage {
            expiry: now + self.message_ttl,
            data,
        });
    }

    pub fn drain_public(&self, secret: &str) -> Vec<Vec<u8>> {
        let now = now();
        let mut caches = self.caches.write();
        if let Some(entry) = caches.get_mut(secret) {
            let mut result = Vec::new();
            while let Some(msg) = entry.public.pop_front() {
                if msg.expiry > now {
                    result.push(msg.data);
                }
            }
            result
        } else {
            Vec::new()
        }
    }

    // ── 令牌缓存 ──

    pub fn add_token(&self, secret: &str, token: &str, data: Vec<u8>) {
        let now = now();
        let mut caches = self.caches.write();
        let entry = caches.entry(secret.to_string()).or_insert_with(|| SecretCache {
            public: VecDeque::new(),
            tokens: HashMap::new(),
        });
        let deque = entry.tokens.entry(token.to_string()).or_insert_with(|| {
            VecDeque::with_capacity(self.max_token.min(256))
        });
        if deque.len() >= self.max_token {
            deque.pop_front();
        }
        deque.push_back(CachedMessage {
            expiry: now + self.message_ttl,
            data,
        });
    }

    pub fn drain_token(&self, secret: &str, token: &str) -> Vec<Vec<u8>> {
        let now = now();
        let mut caches = self.caches.write();
        if let Some(entry) = caches.get_mut(secret) {
            if let Some(deque) = entry.tokens.get_mut(token) {
                let mut result = Vec::new();
                while let Some(msg) = deque.pop_front() {
                    if msg.expiry > now {
                        result.push(msg.data);
                    }
                }
                return result;
            }
        }
        Vec::new()
    }

    // ── 清理 ──

    pub fn cleanup(&self) {
        let now = now();

        // 清理去重缓存
        {
            let mut dedup = self.dedup.write();
            dedup.retain(|_, expiry| *expiry > now);
        }

        // 清理消息缓存
        {
            let mut caches = self.caches.write();
            caches.retain(|_, entry| {
                entry.public.retain(|m| m.expiry > now);
                for deque in entry.tokens.values_mut() {
                    deque.retain(|m| m.expiry > now);
                }
                entry.tokens.retain(|_, d| !d.is_empty());
                !entry.public.is_empty() || !entry.tokens.is_empty()
            });
        }
    }
}
