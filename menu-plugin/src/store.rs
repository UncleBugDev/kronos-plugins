use std::collections::{HashMap, HashSet};
use std::fs;
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

#[derive(Default, Serialize, Deserialize)]
pub struct SettingsDb {
    #[serde(default)]
    pub players: HashMap<String, PlayerPrefs>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPage {
    Main,
    Settings,
    Rules,
    RanksInfo,
    ServerInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BedrockFormKind {
    Main,
    Settings,
    Rules,
    RanksInfo,
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
    let _ = fs::create_dir_all(&data_folder);
    let path = format!("{data_folder}/settings.json");
    let db = match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<SettingsDb>(&content).unwrap_or_default(),
        Err(_) => SettingsDb::default(),
    };
    let _ = STATE.set(Mutex::new(PluginState {
        data_folder,
        db,
        open_pages: HashMap::new(),
        bedrock_forms: HashMap::new(),
        delivered_this_session: HashSet::new(),
        last_interact_ms: HashMap::new(),
    }));
}

/// Accesses pure-Rust plugin state under a short-lived lock.
/// NEVER call any WIT host API or drop any WIT resource inside `f`.
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

impl PluginState {
    pub fn save_db(&self) {
        let _ = fs::create_dir_all(&self.data_folder);
        let path = format!("{}/settings.json", self.data_folder);
        if let Ok(json) = serde_json::to_string_pretty(&self.db) {
            let _ = fs::write(path, json);
        }
    }

    pub fn prefs_for(&self, key: &str) -> PlayerPrefs {
        self.db.players.get(key).cloned().unwrap_or_default()
    }

    pub fn update_prefs(&mut self, key: &str, f: impl FnOnce(&mut PlayerPrefs)) -> PlayerPrefs {
        let entry = self.db.players.entry(key.to_string()).or_default();
        f(entry);
        let updated = entry.clone();
        self.save_db();
        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_and_time_cycles() {
        let w = WeatherPref::Server;
        assert_eq!(w.cycle(), WeatherPref::Clear);
        assert_eq!(w.cycle().cycle(), WeatherPref::Downfall);
        assert_eq!(w.cycle().cycle().cycle(), WeatherPref::Server);

        let t = TimePref::Server;
        assert_eq!(t.cycle(), TimePref::Day);
        assert_eq!(t.cycle().cycle(), TimePref::Sunset);
        assert_eq!(t.cycle().cycle().cycle(), TimePref::Midnight);
        assert_eq!(t.cycle().cycle().cycle().cycle(), TimePref::Server);
    }

    #[test]
    fn test_default_prefs_serde() {
        let parsed: PlayerPrefs = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(parsed.weather, WeatherPref::Server);
        assert_eq!(parsed.time, TimePref::Server);
        assert!(!parsed.hud_compass);
        assert!(parsed.chat_mentions);
        assert!(!parsed.hide_players);
        assert!(parsed.auto_compass);
    }
}
