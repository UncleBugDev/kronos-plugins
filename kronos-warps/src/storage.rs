use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::models::{format_cost_minor, normalize_key, WarpError, WarpStore, WarpsState};

pub const STORE_FILENAME: &str = "warps.json";

static STATE: OnceLock<Mutex<WarpsState>> = OnceLock::new();

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn with_state<R>(f: impl FnOnce(&mut WarpsState) -> R) -> R {
    let lock = STATE.get_or_init(|| {
        Mutex::new(WarpsState::new(".".to_string(), WarpStore::default()))
    });
    let mut guard = match lock.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    f(&mut guard)
}

pub fn init_storage(data_folder: String) {
    let folder = PathBuf::from(&data_folder);
    let _ = fs::create_dir_all(&folder);

    let store_path = folder.join(STORE_FILENAME);
    let store = match load_store_from_path(&store_path) {
        Ok(parsed) => parsed,
        Err(_) => {
            let fresh = WarpStore::default();
            let _ = save_store_to_path(&store_path, &fresh);
            fresh
        }
    };

    with_state(|st| {
        st.data_folder = data_folder;
        st.store = store;
    });
}

pub fn load_store_from_path(path: &Path) -> Result<WarpStore, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str::<WarpStore>(&raw).map_err(|e| e.to_string())
}

pub fn save_store_to_path(path: &Path, store: &WarpStore) -> Result<(), String> {
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    atomic_write(path, &json)
}

/// Атомарная запись: временный файл `.json.tmp` -> `fs::rename`.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::write(path, content);
        let _ = fs::remove_file(&tmp_path);
        e.to_string()
    })
}

/// Сохраняет `warps.json` на диск строго после освобождения `Mutex`.
pub fn save_current_store() {
    let (data_folder, snapshot) = with_state(|st| (st.data_folder.clone(), st.store.clone()));
    let path = PathBuf::from(data_folder).join(STORE_FILENAME);
    if let Err(err) = save_store_to_path(&path, &snapshot) {
        tracing::warn!("KRONOS Warps: ошибка сохранения warps.json: {}", err);
    }
}

// =============================================================================
// Интеграция платного входа с kronos-economy (`../kronos-economy/economy.json`)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EconomyAccountBridge {
    player_key: String,
    player_name: String,
    balance: u64,
    #[serde(default)]
    total_received: u64,
    #[serde(default)]
    total_sent: u64,
    #[serde(default)]
    updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EconomyStoreBridge {
    #[serde(default = "default_eco_ver")]
    version: u32,
    #[serde(default = "default_eco_sym")]
    currency_symbol: String,
    #[serde(default = "default_eco_name")]
    currency_name: String,
    #[serde(default)]
    starting_balance: u64,
    #[serde(default)]
    accounts: HashMap<String, EconomyAccountBridge>,
}

fn default_eco_ver() -> u32 {
    1
}
fn default_eco_sym() -> String {
    "⛃".to_string()
}
fn default_eco_name() -> String {
    "Кронов".to_string()
}

fn resolve_economy_path(warps_data_folder: &str) -> Option<PathBuf> {
    let parent = Path::new(warps_data_folder).parent()?;
    Some(parent.join("kronos-economy").join("economy.json"))
}

/// Списывает плату за вход на пользовательский варп через файл `kronos-economy/economy.json`.
pub fn charge_warp_entry(
    warps_data_folder: &str,
    visitor_name: &str,
    owner_name: &str,
    cost_minor: u64,
) -> Result<(), WarpError> {
    if cost_minor == 0 || normalize_key(visitor_name) == normalize_key(owner_name) {
        return Ok(());
    }
    let Some(eco_path) = resolve_economy_path(warps_data_folder) else {
        return Err(WarpError::EconomyError(
            "Хранилище kronos-economy недоступно.".to_string(),
        ));
    };
    charge_warp_entry_at_path(&eco_path, visitor_name, owner_name, cost_minor, now_ms())
}

pub fn charge_warp_entry_at_path(
    eco_path: &Path,
    visitor_name: &str,
    owner_name: &str,
    cost_minor: u64,
    ts_ms: u64,
) -> Result<(), WarpError> {
    if cost_minor == 0 || normalize_key(visitor_name) == normalize_key(owner_name) {
        return Ok(());
    }
    let raw = fs::read_to_string(eco_path).map_err(|_| {
        WarpError::EconomyError(
            "Плагин kronos-economy не найден (отсутствует economy.json).".to_string(),
        )
    })?;
    let mut eco: EconomyStoreBridge = serde_json::from_str(&raw).map_err(|_| {
        WarpError::EconomyError("Ошибка чтения балансов из kronos-economy.".to_string())
    })?;

    let visitor_key = normalize_key(visitor_name);
    let owner_key = normalize_key(owner_name);

    let visitor_bal = eco
        .accounts
        .get(&visitor_key)
        .map(|a| a.balance)
        .unwrap_or(eco.starting_balance);
    let new_visitor_bal = visitor_bal.checked_sub(cost_minor).ok_or_else(|| {
        WarpError::EconomyError(format!(
            "Недостаточно средств для входа: нужно {}, у вас {}.",
            format_cost_minor(cost_minor),
            format_cost_minor(visitor_bal)
        ))
    })?;

    let owner_bal = eco
        .accounts
        .get(&owner_key)
        .map(|a| a.balance)
        .unwrap_or(eco.starting_balance);
    let new_owner_bal = owner_bal.checked_add(cost_minor).ok_or_else(|| {
        WarpError::EconomyError("Переполнение баланса владельца варпа.".to_string())
    })?;

    let v_acc = eco
        .accounts
        .entry(visitor_key.clone())
        .or_insert_with(|| EconomyAccountBridge {
            player_key: visitor_key,
            player_name: visitor_name.to_string(),
            balance: eco.starting_balance,
            total_received: 0,
            total_sent: 0,
            updated_at_ms: ts_ms,
        });
    v_acc.balance = new_visitor_bal;
    v_acc.total_sent = v_acc.total_sent.saturating_add(cost_minor);
    v_acc.updated_at_ms = ts_ms;

    let o_acc = eco
        .accounts
        .entry(owner_key.clone())
        .or_insert_with(|| EconomyAccountBridge {
            player_key: owner_key,
            player_name: owner_name.to_string(),
            balance: eco.starting_balance,
            total_received: 0,
            total_sent: 0,
            updated_at_ms: ts_ms,
        });
    o_acc.balance = new_owner_bal;
    o_acc.total_received = o_acc.total_received.saturating_add(cost_minor);
    o_acc.updated_at_ms = ts_ms;

    let json = serde_json::to_string_pretty(&eco)
        .map_err(|e| WarpError::EconomyError(e.to_string()))?;
    atomic_write(eco_path, &json).map_err(WarpError::EconomyError)
}
