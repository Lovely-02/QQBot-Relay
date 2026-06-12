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
    settings: RwLock<CacheSettings>,
    caches: RwLock<HashMap<String, SecretCache>>,
    dedup: RwLock<HashMap<String, f64>>,
}

struct CacheSettings {
    max_public: usize,
    max_token: usize,
    message_ttl: f64,
    dedup_ttl: f64,
    no_cache_secrets: Vec<String>,
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
            settings: RwLock::new(CacheSettings {
                max_public,
                max_token,
                message_ttl: message_ttl as f64,
                dedup_ttl: dedup_ttl as f64,
                no_cache_secrets,
            }),
            caches: RwLock::new(HashMap::new()),
            dedup: RwLock::new(HashMap::new()),
        }
    }

    pub fn reconfigure(
        &self,
        max_public: usize,
        max_token: usize,
        message_ttl: u64,
        dedup_ttl: u64,
        no_cache_secrets: Vec<String>,
    ) {
        *self.settings.write() = CacheSettings {
            max_public,
            max_token,
            message_ttl: message_ttl as f64,
            dedup_ttl: dedup_ttl as f64,
            no_cache_secrets,
        };
        let settings = self.settings.read();
        let mut caches = self.caches.write();
        caches.retain(|secret, entry| {
            if settings.no_cache_secrets.iter().any(|item| item == secret) {
                return false;
            }
            while entry.public.len() > settings.max_public {
                entry.public.pop_front();
            }
            for messages in entry.tokens.values_mut() {
                while messages.len() > settings.max_token {
                    messages.pop_front();
                }
            }
            entry.tokens.retain(|_, messages| !messages.is_empty());
            !entry.public.is_empty() || !entry.tokens.is_empty()
        });
        drop(caches);
        drop(settings);
        self.cleanup();
    }

    pub fn should_cache(&self, secret: &str) -> bool {
        !self
            .settings
            .read()
            .no_cache_secrets
            .iter()
            .any(|item| item == secret)
    }

    // ── 消息去重 ──

    fn dedup_key(secret: &str, msg_id: &str) -> String {
        format!("{secret}\0{msg_id}")
    }

    pub fn is_duplicate(&self, secret: &str, msg_id: &str) -> bool {
        let now = now();
        if let Some(&expiry) = self.dedup.read().get(&Self::dedup_key(secret, msg_id)) {
            if expiry > now {
                return true;
            }
        }
        false
    }

    pub fn add_msg_id(&self, secret: &str, msg_id: &str) {
        let expiry = now() + self.settings.read().dedup_ttl;
        self.dedup
            .write()
            .insert(Self::dedup_key(secret, msg_id), expiry);
    }

    // ── 公共缓存 ──

    pub fn add_public(&self, secret: &str, data: Vec<u8>) {
        let now = now();
        let settings = self.settings.read();
        let mut caches = self.caches.write();
        let entry = caches
            .entry(secret.to_string())
            .or_insert_with(|| SecretCache {
                public: VecDeque::new(),
                tokens: HashMap::new(),
            });
        if settings.max_public == 0 {
            return;
        }
        while entry.public.len() >= settings.max_public {
            entry.public.pop_front();
        }
        entry.public.push_back(CachedMessage {
            expiry: now + settings.message_ttl,
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
        let settings = self.settings.read();
        let mut caches = self.caches.write();
        let entry = caches
            .entry(secret.to_string())
            .or_insert_with(|| SecretCache {
                public: VecDeque::new(),
                tokens: HashMap::new(),
            });
        if settings.max_token == 0 {
            return;
        }
        let deque = entry
            .tokens
            .entry(token.to_string())
            .or_insert_with(|| VecDeque::with_capacity(settings.max_token.min(256)));
        while deque.len() >= settings.max_token {
            deque.pop_front();
        }
        deque.push_back(CachedMessage {
            expiry: now + settings.message_ttl,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplication_is_scoped_by_secret() {
        let cache = CacheManager::new(10, 10, 60, 60, Vec::new());
        cache.add_msg_id("secret-a", "message-1");

        assert!(cache.is_duplicate("secret-a", "message-1"));
        assert!(!cache.is_duplicate("secret-b", "message-1"));
    }

    #[test]
    fn reconfigure_applies_new_cache_limits() {
        let cache = CacheManager::new(10, 10, 60, 60, Vec::new());
        cache.reconfigure(1, 1, 60, 60, Vec::new());
        cache.add_public("secret", vec![1]);
        cache.add_public("secret", vec![2]);

        assert_eq!(cache.drain_public("secret"), vec![vec![2]]);
    }
}
