use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use pumpkin_plugin_api::boss_bar::BossBar;
use serde::{Deserialize, Serialize};

use crate::sha256::{constant_time_eq, hash_with_salt, sha256_hex};

pub const AUTH_TIMEOUT_SECS: u32 = 60;
pub const SESSION_TTL_SECS: u64 = 24 * 60 * 60; // 24 hours IP auto-login

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountRecord {
    pub username: String,
    pub salt: String,
    pub password_hash: String,
    pub last_ip: String,
    pub last_login_ts: u64,
    pub registered_ts: u64,
    pub login_count: u64,
    pub kit_claimed: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub struct AccountsDb {
    pub accounts: HashMap<String, AccountRecord>,
}

pub struct PlayerSession {
    pub key: String,
    pub authenticated: bool,
    pub pin_buffer: String,
    pub cabinet_open: bool,
    pub remaining_secs: u32,
    pub wrong_attempts: u8,
    pub boss_bar: Option<BossBar>,
    pub bedrock_form_id: Option<u32>,
}

pub struct PluginState {
    pub data_folder: String,
    pub db: AccountsDb,
    pub sessions: HashMap<String, PlayerSession>,
}

static STATE: OnceLock<Mutex<PluginState>> = OnceLock::new();

fn canonical_ip(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                mapped_v4.to_string()
            } else {
                v6.to_string()
            }
        }
    }
}

/// Strips `:port`, brackets, IPv6 zone suffixes, and IPv4-mapped IPv6 prefixes
/// so 24h IP session checks remain stable across ephemeral client/proxy ports.
pub fn normalize_ip(raw: &str) -> String {
    let first = raw.split(',').next().unwrap_or("").trim();
    if first.is_empty() {
        return String::new();
    }

    if let Ok(sock) = first.parse::<SocketAddr>() {
        return canonical_ip(sock.ip());
    }

    if let Some(rest) = first.strip_prefix('[') {
        if let Some((inside, _after)) = rest.split_once(']') {
            let clean_v6 = inside.split('%').next().unwrap_or(inside).trim();
            if let Ok(ip) = clean_v6.parse::<IpAddr>() {
                return canonical_ip(ip);
            }
            return clean_v6.to_lowercase();
        }
    }

    let without_zone = first.split('%').next().unwrap_or(first).trim();
    if let Ok(ip) = without_zone.parse::<IpAddr>() {
        return canonical_ip(ip);
    }

    if let Some((host, port)) = first.rsplit_once(':') {
        if !host.contains(':') && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            return host.trim().to_lowercase();
        }
    }

    without_zone.to_lowercase()
}

pub fn unix_secs_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn generate_salt(username: &str, ip: &str, now: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seed = format!("{username}:{ip}:{now}:{nanos}");
    sha256_hex(seed.as_bytes())[..16].to_string()
}

pub fn init_state(data_folder: String) {
    let _ = fs::create_dir_all(&data_folder);
    let path = format!("{data_folder}/accounts.json");
    let mut db = match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<AccountsDb>(&content).unwrap_or_default(),
        Err(_) => AccountsDb::default(),
    };
    for acc in db.accounts.values_mut() {
        if !acc.last_ip.is_empty() {
            acc.last_ip = normalize_ip(&acc.last_ip);
        }
    }
    let _ = STATE.set(Mutex::new(PluginState {
        data_folder,
        db,
        sessions: HashMap::new(),
    }));
}

pub fn with_state<R>(f: impl FnOnce(&mut PluginState) -> R) -> R {
    let mutex = STATE.get_or_init(|| {
        Mutex::new(PluginState {
            data_folder: ".".to_string(),
            db: AccountsDb::default(),
            sessions: HashMap::new(),
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

impl PluginState {
    pub fn save_db(&self) {
        let _ = fs::create_dir_all(&self.data_folder);
        let path = format!("{}/accounts.json", self.data_folder);
        if let Ok(json) = serde_json::to_string_pretty(&self.db) {
            let _ = fs::write(path, json);
        }
    }

    pub fn is_registered(&self, key: &str) -> bool {
        self.db.accounts.contains_key(key)
    }

    pub fn has_valid_ip_session(&self, key: &str, ip: &str, now: u64) -> bool {
        let norm_ip = normalize_ip(ip);
        if let Some(acc) = self.db.accounts.get(key) {
            !norm_ip.is_empty()
                && normalize_ip(&acc.last_ip) == norm_ip
                && now.saturating_sub(acc.last_login_ts) <= SESSION_TTL_SECS
        } else {
            false
        }
    }

    pub fn register_account(&mut self, username: &str, ip: &str, password: &str) -> AccountRecord {
        let key = username.to_lowercase();
        let norm_ip = normalize_ip(ip);
        let now = unix_secs_now();
        let salt = generate_salt(&key, &norm_ip, now);
        let password_hash = hash_with_salt(&salt, password);
        let record = AccountRecord {
            username: username.to_string(),
            salt,
            password_hash,
            last_ip: norm_ip,
            last_login_ts: now,
            registered_ts: now,
            login_count: 1,
            kit_claimed: false,
        };
        self.db.accounts.insert(key, record.clone());
        self.save_db();
        record
    }

    pub fn verify_and_touch_login(&mut self, key: &str, ip: &str, password: &str) -> Option<AccountRecord> {
        let acc = self.db.accounts.get_mut(key)?;
        let candidate = hash_with_salt(&acc.salt, password);
        if constant_time_eq(&candidate, &acc.password_hash) {
            acc.last_ip = normalize_ip(ip);
            acc.last_login_ts = unix_secs_now();
            acc.login_count = acc.login_count.saturating_add(1);
            let cloned = acc.clone();
            self.save_db();
            Some(cloned)
        } else {
            None
        }
    }

    pub fn change_password(&mut self, key: &str, old_pass: &str, new_pass: &str) -> Result<(), &'static str> {
        let Some(acc) = self.db.accounts.get_mut(key) else {
            return Err("Аккаунт не найден.");
        };
        let check = hash_with_salt(&acc.salt, old_pass);
        if !constant_time_eq(&check, &acc.password_hash) {
            return Err("Старый пароль введён неверно.");
        }
        let now = unix_secs_now();
        let new_salt = generate_salt(key, &acc.last_ip, now);
        acc.salt = new_salt.clone();
        acc.password_hash = hash_with_salt(&new_salt, new_pass);
        self.save_db();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ip_variants() {
        assert_eq!(normalize_ip("192.168.100.10:54321"), "192.168.100.10");
        assert_eq!(normalize_ip("192.168.100.10"), "192.168.100.10");
        assert_eq!(normalize_ip("[::ffff:192.168.100.10]:54321"), "192.168.100.10");
        assert_eq!(normalize_ip("::ffff:192.168.100.10"), "192.168.100.10");
        assert_eq!(normalize_ip("[2001:db8::1]:25565"), "2001:db8::1");
        assert_eq!(normalize_ip("[2001:DB8:0:0::1]"), "2001:db8::1");
        assert_eq!(normalize_ip("2001:DB8::1"), "2001:db8::1");
        assert_eq!(normalize_ip("::1"), "::1");
        assert_eq!(normalize_ip("[::1]:19132"), "::1");
        assert_eq!(normalize_ip("[fe80::1%eth0]:54321"), "fe80::1");
        assert_eq!(normalize_ip("fe80::1%12"), "fe80::1");
        assert_eq!(normalize_ip("203.0.113.7:40123, 127.0.0.1"), "203.0.113.7");
        assert_eq!(normalize_ip(""), "");
    }
}
