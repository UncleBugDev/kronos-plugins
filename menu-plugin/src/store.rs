use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherPref {
    #[default]
    Server,
    Clear,
    Downfall,
}

impl WeatherPref {
    pub const fn cycle(self) -> Self {
        match self {
            Self::Server => Self::Clear,
            Self::Clear => Self::Downfall,
            Self::Downfall => Self::Server,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Server => "Серверная",
            Self::Clear => "Всегда ясно ☀",
            Self::Downfall => "Дождь 🌧",
        }
    }

    pub const fn colored_label(self) -> &'static str {
        match self {
            Self::Server => "§7Серверная",
            Self::Clear => "§eВсегда ясно ☀",
            Self::Downfall => "§bДождь 🌧",
        }
    }

    pub const fn to_index(self) -> u32 {
        match self {
            Self::Server => 0,
            Self::Clear => 1,
            Self::Downfall => 2,
        }
    }

    pub const fn from_index(idx: u32) -> Self {
        match idx {
            1 => Self::Clear,
            2 => Self::Downfall,
            _ => Self::Server,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePref {
    #[default]
    Server,
    Day,
    Sunset,
    Midnight,
}

impl TimePref {
    pub const fn cycle(self) -> Self {
        match self {
            Self::Server => Self::Day,
            Self::Day => Self::Sunset,
            Self::Sunset => Self::Midnight,
            Self::Midnight => Self::Server,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Server => "Серверное",
            Self::Day => "День ☀ (6000)",
            Self::Sunset => "Закат 🌅 (12500)",
            Self::Midnight => "Полночь 🌙 (18000)",
        }
    }

    pub const fn colored_label(self) -> &'static str {
        match self {
            Self::Server => "§7Серверное",
            Self::Day => "§eДень ☀ (6000)",
            Self::Sunset => "§6Закат 🌅 (12500)",
            Self::Midnight => "§dПолночь 🌙 (18000)",
        }
    }

    pub const fn ticks(self) -> Option<u64> {
        match self {
            Self::Server => None,
            Self::Day => Some(6000),
            Self::Sunset => Some(12500),
            Self::Midnight => Some(18000),
        }
    }

    pub const fn to_index(self) -> u32 {
        match self {
            Self::Server => 0,
            Self::Day => 1,
            Self::Sunset => 2,
            Self::Midnight => 3,
        }
    }

    pub const fn from_index(idx: u32) -> Self {
        match idx {
            1 => Self::Day,
            2 => Self::Sunset,
            3 => Self::Midnight,
            _ => Self::Server,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerPrefs {
    #[serde(default)]
    pub weather: WeatherPref,
    #[serde(default)]
    pub time: TimePref,
    #[serde(default)]
    pub hud_compass: bool,
    #[serde(default = "default_true")]
    pub chat_mentions: bool,
    #[serde(default)]
    pub hide_players: bool,
    #[serde(default = "default_true")]
    pub auto_compass: bool,
}

impl Default for PlayerPrefs {
    fn default() -> Self {
        Self {
            weather: WeatherPref::Server,
            time: TimePref::Server,
            hud_compass: false,
            chat_mentions: true,
            hide_players: false,
            auto_compass: true,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SettingsDb {
    #[serde(default)]
    pub players: HashMap<String, PlayerPrefs>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPage {
    Main,
    Navigation,
    Economy,
    ChatGuide,
    RanksInfo,
    Settings,
    Rules,
    ServerInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BedrockFormKind {
    Main,
    Navigation,
    Economy,
    ChatGuide,
    RanksInfo,
    Settings,
    Rules,
    ServerInfo,
}

pub struct PluginState {
    pub data_folder: String,
    pub db: SettingsDb,
    pub open_pages: HashMap<String, MenuPage>,
    pub bedrock_forms: HashMap<String, (u32, BedrockFormKind)>,
    pub delivered_this_session: HashSet<String>,
    pub last_interact_ms: HashMap<String, u128>,
}

static STATE: OnceLock<Mutex<PluginState>> = OnceLock::new();

pub fn unix_ms_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn init_state(data_folder: String) {
    let folder = PathBuf::from(&data_folder);
    let _ = fs::create_dir_all(&folder);
    let path = folder.join("settings.json");
    let db = load_db_from_path(&path).unwrap_or_default();
    let _ = STATE.set(Mutex::new(PluginState {
        data_folder,
        db,
        open_pages: HashMap::new(),
        bedrock_forms: HashMap::new(),
        delivered_this_session: HashSet::new(),
        last_interact_ms: HashMap::new(),
    }));
}

pub fn with_state<R>(f: impl FnOnce(&mut PluginState) -> R) -> R {
    let mutex = STATE.get_or_init(|| {
        Mutex::new(PluginState {
            data_folder: ".".to_string(),
            db: SettingsDb::default(),
            open_pages: HashMap::new(),
            bedrock_forms: HashMap::new(),
            delivered_this_session: HashSet::new(),
            last_interact_ms: HashMap::new(),
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

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

pub fn load_db_from_path(path: &Path) -> Result<SettingsDb, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str::<SettingsDb>(&raw).map_err(|e| e.to_string())
}

pub fn save_db_to_path(path: &Path, db: &SettingsDb) -> Result<(), String> {
    let json = serde_json::to_string_pretty(db).map_err(|e| e.to_string())?;
    atomic_write(path, &json)
}

/// Сохраняет `settings.json` атомарно строго ПОСЛЕ освобождения Mutex.
pub fn save_current_db() {
    let (folder, snapshot) = with_state(|st| (st.data_folder.clone(), st.db.clone()));
    let path = PathBuf::from(folder).join("settings.json");
    if let Err(err) = save_db_to_path(&path, &snapshot) {
        tracing::warn!("KRONOS Menu: ошибка сохранения settings.json: {}", err);
    }
}

impl PluginState {
    pub fn prefs_for(&self, key: &str) -> PlayerPrefs {
        self.db.players.get(key).cloned().unwrap_or_default()
    }

    pub fn update_prefs(&mut self, key: &str, f: impl FnOnce(&mut PlayerPrefs)) -> PlayerPrefs {
        let entry = self.db.players.entry(key.to_string()).or_default();
        f(entry);
        entry.clone()
    }
}

/// Читает актуальный баланс игрока из соседнего `../kronos-economy/economy.json` (если установлен).
pub fn read_player_balance_display(menu_data_folder: &str, player_name: &str) -> String {
    let Some(parent) = Path::new(menu_data_folder).parent() else {
        return "— (/bal)".to_string();
    };
    let eco_path = parent.join("kronos-economy").join("economy.json");
    read_balance_from_economy_file(&eco_path, player_name)
        .unwrap_or_else(|| "— (/bal)".to_string())
}

pub fn read_balance_from_economy_file(eco_path: &Path, player_name: &str) -> Option<String> {
    let raw = fs::read_to_string(eco_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let symbol = json
        .get("currency_symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("⛃");
    let key = player_name.trim().to_ascii_lowercase();
    let minor = json
        .get("accounts")
        .and_then(|a| a.get(&key))
        .and_then(|u| u.get("balance"))
        .and_then(|b| b.as_u64())
        .or_else(|| json.get("starting_balance").and_then(|b| b.as_u64()))?;
    let whole = minor / 100;
    let frac = minor % 100;
    if frac == 0 {
        Some(format!("{whole} {symbol}"))
    } else {
        Some(format!("{whole}.{frac:02} {symbol}"))
    }
}
