#![allow(dead_code)]
use crate::data::db::{Account, Database};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AccountManager {
    db: Arc<Database>,
    cache: RwLock<HashMap<String, Account>>,
    webhook_targets: RwLock<HashMap<String, Vec<String>>>,
}

impl AccountManager {
    pub fn new(db: Arc<Database>) -> Self {
        let accounts = db.get_all_accounts();
        let cache: HashMap<_, _> = accounts.into_iter().map(|a| (a.appid.clone(), a)).collect();
        let count = cache.len();

        let all_targets = db.get_all_webhook_targets();
        let mut webhook_targets: HashMap<String, Vec<String>> = HashMap::new();
        for (appid, url) in all_targets {
            webhook_targets.entry(appid).or_default().push(url);
        }
        let target_count: usize = webhook_targets.values().map(|v| v.len()).sum();

        tracing::info!(
            "已从数据库加载 {} 个账号, {} 个转发目标",
            count,
            target_count
        );
        Self {
            db,
            cache: RwLock::new(cache),
            webhook_targets: RwLock::new(webhook_targets),
        }
    }

    pub fn get_all(&self) -> Vec<Account> {
        self.cache.read().values().cloned().collect()
    }

    pub fn get(&self, appid: &str) -> Option<Account> {
        self.cache.read().get(appid).cloned()
    }

    pub fn get_secret(&self, appid: &str) -> Option<String> {
        self.cache.read().get(appid).map(|a| a.secret.clone())
    }

    pub fn create(&self, appid: String, secret: String, description: String) -> bool {
        if self.cache.read().contains_key(&appid) {
            return false;
        }
        let account = Account {
            appid: appid.clone(),
            secret,
            description,
            create_time: chrono::Utc::now().timestamp() as f64,
        };
        self.db.create_account(
            account.appid.clone(),
            account.secret.clone(),
            account.description.clone(),
        );
        self.cache.write().insert(appid, account);
        true
    }

    pub fn delete(&self, appid: &str) -> bool {
        if self.cache.write().remove(appid).is_some() {
            self.db.delete_account(appid);
            self.webhook_targets.write().remove(appid);
            true
        } else {
            false
        }
    }

    pub fn verify_signature(
        &self,
        appid: &str,
        signature: &str,
        timestamp: &str,
        nonce: &str,
        body: &str,
    ) -> bool {
        self.db
            .verify_appid_signature(appid, signature, timestamp, nonce, body)
    }

    pub fn get_webhook_urls(&self, appid: &str) -> Vec<String> {
        self.webhook_targets
            .read()
            .get(appid)
            .cloned()
            .unwrap_or_default()
    }

    pub fn add_webhook_target(&self, appid: &str, url: &str) {
        self.db
            .add_webhook_target(appid.to_string(), url.to_string());
        self.webhook_targets
            .write()
            .entry(appid.to_string())
            .or_default()
            .push(url.to_string());
    }

    pub fn remove_webhook_target(&self, appid: &str, url: &str) {
        self.db.remove_webhook_target(appid, url);
        if let Some(urls) = self.webhook_targets.write().get_mut(appid) {
            urls.retain(|u| u != url);
            if urls.is_empty() {
                self.webhook_targets.write().remove(appid);
            }
        }
    }
}
