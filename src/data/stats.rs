use crate::data::db::Database;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;

struct SecretStats {
    ws_success: AtomicI64,
    ws_failure: AtomicI64,
    wh_success: AtomicI64,
    wh_failure: AtomicI64,
}

impl Default for SecretStats {
    fn default() -> Self {
        Self {
            ws_success: AtomicI64::new(0),
            ws_failure: AtomicI64::new(0),
            wh_success: AtomicI64::new(0),
            wh_failure: AtomicI64::new(0),
        }
    }
}

pub struct StatsManager {
    db: Arc<Database>,
    total_messages: AtomicI64,
    ws_success: AtomicI64,
    ws_failure: AtomicI64,
    wh_success: AtomicI64,
    wh_failure: AtomicI64,
    per_secret: DashMap<String, SecretStats>,
    dirty: AtomicBool,
}

impl StatsManager {
    pub fn new(db: Arc<Database>) -> Self {
        let global = db.get_global_stats();
        let per_secret_map = DashMap::new();
        for entry in db.get_all_per_secret_stats() {
            per_secret_map.insert(entry.secret, SecretStats {
                ws_success: AtomicI64::new(entry.ws_success),
                ws_failure: AtomicI64::new(entry.ws_failure),
                wh_success: AtomicI64::new(entry.wh_success),
                wh_failure: AtomicI64::new(entry.wh_failure),
            });
        }
        tracing::info!("已加载统计: {} 条消息, {} 个密钥", global.total_messages, per_secret_map.len());
        Self {
            db,
            total_messages: AtomicI64::new(global.total_messages),
            ws_success: AtomicI64::new(global.ws_success),
            ws_failure: AtomicI64::new(global.ws_failure),
            wh_success: AtomicI64::new(global.wh_success),
            wh_failure: AtomicI64::new(global.wh_failure),
            per_secret: per_secret_map,
            dirty: AtomicBool::new(false),
        }
    }

    pub fn increment_messages(&self) {
        self.total_messages.fetch_add(1, Ordering::Relaxed);
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn update_ws_stats(&self, secret: &str, success: i64, failure: i64) {
        if success > 0 { self.ws_success.fetch_add(success, Ordering::Relaxed); }
        if failure > 0 { self.ws_failure.fetch_add(failure, Ordering::Relaxed); }
        let entry = self.per_secret.entry(secret.to_string()).or_default();
        if success > 0 { entry.ws_success.fetch_add(success, Ordering::Relaxed); }
        if failure > 0 { entry.ws_failure.fetch_add(failure, Ordering::Relaxed); }
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn update_wh_stats(&self, secret: &str, success: i64, failure: i64) {
        if success > 0 { self.wh_success.fetch_add(success, Ordering::Relaxed); }
        if failure > 0 { self.wh_failure.fetch_add(failure, Ordering::Relaxed); }
        let entry = self.per_secret.entry(secret.to_string()).or_default();
        if success > 0 { entry.wh_success.fetch_add(success, Ordering::Relaxed); }
        if failure > 0 { entry.wh_failure.fetch_add(failure, Ordering::Relaxed); }
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn flush_to_db(&self) {
        if !self.dirty.swap(false, Ordering::Relaxed) {
            return;
        }
        self.db.flush_global_stats(
            self.total_messages.load(Ordering::Relaxed),
            self.ws_success.load(Ordering::Relaxed),
            self.ws_failure.load(Ordering::Relaxed),
            self.wh_success.load(Ordering::Relaxed),
            self.wh_failure.load(Ordering::Relaxed),
        );
        let entries: Vec<_> = self.per_secret.iter().map(|entry| {
            (
                entry.key().clone(),
                entry.value().ws_success.load(Ordering::Relaxed),
                entry.value().ws_failure.load(Ordering::Relaxed),
                entry.value().wh_success.load(Ordering::Relaxed),
                entry.value().wh_failure.load(Ordering::Relaxed),
            )
        }).collect();
        if !entries.is_empty() {
            self.db.flush_per_secret_stats(entries);
        }
    }

    pub fn get_global(&self) -> crate::data::db::GlobalStats {
        crate::data::db::GlobalStats {
            total_messages: self.total_messages.load(Ordering::Relaxed),
            ws_success: self.ws_success.load(Ordering::Relaxed),
            ws_failure: self.ws_failure.load(Ordering::Relaxed),
            wh_success: self.wh_success.load(Ordering::Relaxed),
            wh_failure: self.wh_failure.load(Ordering::Relaxed),
        }
    }
}
