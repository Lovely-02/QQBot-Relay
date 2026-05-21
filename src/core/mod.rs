pub mod config;

use crate::data::accounts::AccountManager;
use crate::data::cache::CacheManager;
use crate::data::db::Database;
use crate::data::stats::StatsManager;
use crate::net::connections::ConnectionManager;
use crate::core::config::AppConfig;
use std::sync::Arc;

pub struct AppState {
    pub db: Arc<Database>,
    pub config: Arc<parking_lot::RwLock<AppConfig>>,
    pub accounts: Arc<AccountManager>,
    pub cache: Arc<CacheManager>,
    pub stats: Arc<StatsManager>,
    pub connections: Arc<ConnectionManager>,
}
