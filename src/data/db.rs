#![allow(dead_code)]
use anyhow::Result;
use rusqlite::{params, Connection};
use std::sync::mpsc;
use std::thread;
use tracing::info;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub enum DbError {
    Sqlite(rusqlite::Error),
    Custom(String),
}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self { DbError::Sqlite(e) }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Sqlite(e) => write!(f, "SQLite 错误: {}", e),
            DbError::Custom(s) => write!(f, "{}", s),
        }
    }
}

type DbCallback = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

struct DbRequest {
    callback: DbCallback,
}

pub struct Database {
    tx: mpsc::Sender<DbRequest>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub appid: String,
    pub secret: String,
    pub description: String,
    pub create_time: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub token: String,
    pub created: String,
    pub expires: String,
    pub ip: String,
    pub user_agent: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct GlobalStats {
    pub total_messages: i64,
    pub ws_success: i64,
    pub ws_failure: i64,
    pub wh_success: i64,
    pub wh_failure: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PerSecretStats {
    pub secret: String,
    pub ws_success: i64,
    pub ws_failure: i64,
    pub wh_success: i64,
    pub wh_failure: i64,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        Self::init_schema(&mut conn)?;
        info!("数据库初始化完成 {}", path);

        let (tx, rx) = mpsc::channel::<DbRequest>();
        thread::spawn(move || {
            for req in rx {
                (req.callback)(&mut conn);
            }
        });

        Ok(Self { tx })
    }

    fn init_schema(conn: &mut Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS accounts (
                appid TEXT PRIMARY KEY,
                secret TEXT NOT NULL,
                description TEXT DEFAULT '',
                create_time REAL NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                created TEXT NOT NULL,
                expires TEXT NOT NULL,
                ip TEXT NOT NULL,
                user_agent TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ip_access (
                ip TEXT PRIMARY KEY,
                last_access TEXT NOT NULL,
                password_fail_times TEXT DEFAULT '[]',
                is_banned INTEGER DEFAULT 0,
                ban_time TEXT DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS stats_global (
                key TEXT PRIMARY KEY,
                value INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS stats_per_secret (
                secret TEXT PRIMARY KEY,
                ws_success INTEGER DEFAULT 0,
                ws_failure INTEGER DEFAULT 0,
                wh_success INTEGER DEFAULT 0,
                wh_failure INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS webhook_targets (
                appid TEXT NOT NULL,
                url TEXT NOT NULL,
                PRIMARY KEY(appid, url)
            );"
        )?;
        Ok(())
    }

    fn send<F>(&self, f: F)
    where
        F: FnOnce(&mut Connection) + Send + 'static,
    {
        let _ = self.tx.send(DbRequest { callback: Box::new(f) });
    }

    fn query<T: Send + 'static, F>(&self, f: F) -> T
    where
        F: FnOnce(&mut Connection) -> T + Send + 'static,
        T: Default,
    {
        let (resp_tx, resp_rx) = std::sync::mpsc::channel();
        self.send(move |conn| {
            let result = f(conn);
            let _ = resp_tx.send(result);
        });
        resp_rx.recv().unwrap_or_default()
    }

    // ── 账号 ──

    pub fn get_all_accounts(&self) -> Vec<Account> {
        self.query(|conn| {
            let mut stmt = conn.prepare(
                "SELECT appid, secret, description, create_time FROM accounts ORDER BY create_time DESC"
            ).unwrap();
            stmt.query_map([], |row| {
                Ok(Account {
                    appid: row.get(0)?,
                    secret: row.get(1)?,
                    description: row.get(2)?,
                    create_time: row.get(3)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default()
        })
    }

    pub fn get_account(&self, appid: &str) -> Option<Account> {
        let appid = appid.to_string();
        self.query(move |conn| {
            conn.query_row(
                "SELECT appid, secret, description, create_time FROM accounts WHERE appid = ?1",
                params![appid],
                |row| {
                    Ok(Account {
                        appid: row.get(0)?,
                        secret: row.get(1)?,
                        description: row.get(2)?,
                        create_time: row.get(3)?,
                    })
                },
            )
            .ok()
        })
    }

    pub fn create_account(&self, appid: String, secret: String, description: String) {
        self.send(move |conn| {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO accounts (appid, secret, description, create_time) VALUES (?1, ?2, ?3, ?4)",
                params![appid, secret, description, chrono::Utc::now().timestamp() as f64],
            );
        });
    }

    pub fn delete_account(&self, appid: &str) {
        let appid = appid.to_string();
        self.send(move |conn| {
            let _ = conn.execute("DELETE FROM accounts WHERE appid = ?1", params![appid]);
        });
    }

    pub fn get_secret_by_appid(&self, appid: &str) -> Option<String> {
        let appid = appid.to_string();
        self.query(move |conn| {
            conn.query_row(
                "SELECT secret FROM accounts WHERE appid = ?1",
                params![appid],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
    }

    pub fn verify_appid_signature(&self, appid: &str, signature: &str, timestamp: &str, nonce: &str, body: &str) -> bool {
        let appid = appid.to_string();
        let signature = signature.to_string();
        let timestamp = timestamp.to_string();
        let nonce = nonce.to_string();
        let body = body.to_string();
        self.query(move |conn| {
            let secret: String = match conn.query_row(
                "SELECT secret FROM accounts WHERE appid = ?1",
                params![appid],
                |row| row.get(0),
            ) {
                Ok(s) => s,
                Err(_) => return false,
            };
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
                Ok(m) => m,
                Err(_) => return false,
            };
            mac.update(timestamp.as_bytes());
            mac.update(nonce.as_bytes());
            mac.update(body.as_bytes());
            let computed = hex::encode(mac.finalize().into_bytes());
            computed == signature
        })
    }

    pub fn get_appid_by_secret(&self, secret: &str) -> Option<String> {
        let secret = secret.to_string();
        self.query(move |conn| {
            conn.query_row(
                "SELECT appid FROM accounts WHERE secret = ?1",
                params![secret],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
    }

    // ── Webhook 转发目标 ──

    pub fn get_webhook_urls(&self, appid: &str) -> Vec<String> {
        let appid = appid.to_string();
        self.query(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT url FROM webhook_targets WHERE appid = ?1"
            ).unwrap();
            stmt.query_map(params![appid], |row| row.get::<_, String>(0))
                .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
                .unwrap_or_default()
        })
    }

    pub fn add_webhook_target(&self, appid: String, url: String) {
        self.send(move |conn| {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO webhook_targets (appid, url) VALUES (?1, ?2)",
                params![appid, url],
            );
        });
    }

    pub fn remove_webhook_target(&self, appid: &str, url: &str) {
        let appid = appid.to_string();
        let url = url.to_string();
        self.send(move |conn| {
            let _ = conn.execute(
                "DELETE FROM webhook_targets WHERE appid = ?1 AND url = ?2",
                params![appid, url],
            );
        });
    }

    pub fn get_all_webhook_targets(&self) -> Vec<(String, String)> {
        self.query(|conn| {
            let mut stmt = conn.prepare(
                "SELECT appid, url FROM webhook_targets"
            ).unwrap();
            stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
                .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
                .unwrap_or_default()
        })
    }

    // ── 会话 ──

    pub fn get_all_sessions(&self) -> Vec<Session> {
        self.query(|conn| {
            let mut stmt = conn.prepare(
                "SELECT token, created, expires, ip, user_agent FROM sessions ORDER BY created DESC"
            ).unwrap();
            stmt.query_map([], |row| {
                Ok(Session {
                    token: row.get(0)?,
                    created: row.get(1)?,
                    expires: row.get(2)?,
                    ip: row.get(3)?,
                    user_agent: row.get(4)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default()
        })
    }

    pub fn save_session(&self, session: Session) {
        self.send(move |conn| {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO sessions (token, created, expires, ip, user_agent) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session.token, session.created, session.expires, session.ip, session.user_agent],
            );
        });
    }

    pub fn delete_session(&self, token: &str) {
        let token = token.to_string();
        self.send(move |conn| {
            let _ = conn.execute("DELETE FROM sessions WHERE token = ?1", params![token]);
        });
    }

    pub fn cleanup_expired_sessions(&self) {
        let now = chrono::Utc::now().to_rfc3339();
        self.send(move |conn| {
            let _ = conn.execute("DELETE FROM sessions WHERE expires < ?1", params![now]);
        });
    }

    // ── IP 访问控制 ──

    pub fn get_ip_access(&self, ip: &str) -> Option<(String, bool)> {
        let ip = ip.to_string();
        self.query(move |conn| {
            conn.query_row(
                "SELECT password_fail_times, is_banned FROM ip_access WHERE ip = ?1",
                params![ip],
                |row| {
                    let fails: String = row.get(0)?;
                    let banned: i32 = row.get(1)?;
                    Ok((fails, banned != 0))
                },
            )
            .ok()
        })
    }

    pub fn update_ip_access(&self, ip: String, fail_times: String, is_banned: bool, ban_time: String) {
        self.send(move |conn| {
            let now = chrono::Utc::now().to_rfc3339();
            let _ = conn.execute(
                "INSERT OR REPLACE INTO ip_access (ip, last_access, password_fail_times, is_banned, ban_time) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![ip, now, fail_times, is_banned as i32, ban_time],
            );
        });
    }

    // ── 统计 ──

    pub fn get_global_stats(&self) -> GlobalStats {
        self.query(|conn| {
            let get_val = |key: &str| -> i64 {
                conn.query_row(
                    "SELECT value FROM stats_global WHERE key = ?1",
                    params![key],
                    |row| row.get(0),
                )
                .unwrap_or(0)
            };
            GlobalStats {
                total_messages: get_val("total_messages"),
                ws_success: get_val("ws_success"),
                ws_failure: get_val("ws_failure"),
                wh_success: get_val("wh_success"),
                wh_failure: get_val("wh_failure"),
            }
        })
    }

    pub fn get_all_per_secret_stats(&self) -> Vec<PerSecretStats> {
        self.query(|conn| {
            let mut stmt = conn.prepare(
                "SELECT secret, ws_success, ws_failure, wh_success, wh_failure FROM stats_per_secret"
            ).unwrap();
            stmt.query_map([], |row| {
                Ok(PerSecretStats {
                    secret: row.get(0)?,
                    ws_success: row.get(1)?,
                    ws_failure: row.get(2)?,
                    wh_success: row.get(3)?,
                    wh_failure: row.get(4)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default()
        })
    }

    pub fn flush_global_stats(&self, total: i64, ws_ok: i64, ws_fail: i64, wh_ok: i64, wh_fail: i64) {
        self.send(move |conn| {
            for (key, val) in [
                ("total_messages", total),
                ("ws_success", ws_ok),
                ("ws_failure", ws_fail),
                ("wh_success", wh_ok),
                ("wh_failure", wh_fail),
            ] {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO stats_global (key, value) VALUES (?1, ?2)",
                    params![key, val],
                );
            }
        });
    }

    pub fn flush_per_secret_stats(&self, entries: Vec<(String, i64, i64, i64, i64)>) {
        self.send(move |conn| {
            for (secret, ws_ok, ws_fail, wh_ok, wh_fail) in entries {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO stats_per_secret (secret, ws_success, ws_failure, wh_success, wh_failure) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![secret, ws_ok, ws_fail, wh_ok, wh_fail],
                );
            }
        });
    }

    // ── 数据库查看器 ──

    pub fn query_table(&self, table: &str) -> Vec<serde_json::Value> {
        let table = table.to_string();
        self.query(move |conn| {
            let allowed = ["accounts", "sessions", "ip_access", "stats_global", "stats_per_secret", "webhook_targets"];
            if !allowed.contains(&table.as_str()) {
                return Vec::new();
            }
            let sql = format!("SELECT * FROM {}", table);
            let mut stmt = match conn.prepare(&sql) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            let rows = stmt.query_map([], |row| {
                let mut map = serde_json::Map::new();
                for (i, col) in columns.iter().enumerate() {
                    let val: serde_json::Value = row.get_ref(i).ok().and_then(|v| match v {
                        rusqlite::types::ValueRef::Null => Some(serde_json::Value::Null),
                        rusqlite::types::ValueRef::Integer(i) => Some(serde_json::Value::Number(i.into())),
                        rusqlite::types::ValueRef::Real(f) => {
                            serde_json::Number::from_f64(f).map(serde_json::Value::Number)
                        }
                        rusqlite::types::ValueRef::Text(t) => {
                            Some(serde_json::Value::String(String::from_utf8_lossy(t).into()))
                        }
                        rusqlite::types::ValueRef::Blob(b) => {
                            Some(serde_json::Value::String(hex::encode(b)))
                        }
                    }).unwrap_or(serde_json::Value::Null);
                    map.insert(col.clone(), val);
                }
                Ok(serde_json::Value::Object(map))
            });
            rows.and_then(|r| r.collect::<Result<Vec<_>, _>>()).unwrap_or_default()
        })
    }
}
