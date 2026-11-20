use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub fn normalize_key(input: &str) -> String {
    input.trim().to_lowercase()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Local,
    Global,
    Trade,
}

impl Channel {
    /// Определяет канал по первому символу сообщения (`!` — Глобальный, `$` — Торговый, иначе Локальный).
    pub fn parse_message(raw: &str) -> (Self, &str) {
        let trimmed = raw.trim();
        if let Some(rest) = trimmed.strip_prefix('!') {
            (Channel::Global, rest.trim_start())
        } else if let Some(rest) = trimmed.strip_prefix('$') {
            (Channel::Trade, rest.trim_start())
        } else {
            (Channel::Local, trimmed)
        }
    }

    pub fn format_line(self, prefix: &str, player_name: &str, text: &str) -> String {
        match self {
            Channel::Local => format!("§8[§fЛ§8] {prefix}{player_name} §8» §f{text}"),
            Channel::Global => format!("§8[§6Г§8] {prefix}{player_name} §8» §f{text}"),
            Channel::Trade => format!("§8[§e§lТОРГ§8] {prefix}{player_name} §8» §e{text}"),
        }
    }
}

/// Проверяет, находится ли получатель в пределах радиуса локального чата в том же мире.
pub fn is_within_local_radius(
    speaker_world: &str,
    speaker_pos: (f64, f64, f64),
    listener_world: &str,
    listener_pos: (f64, f64, f64),
    radius: f64,
) -> bool {
    if !speaker_world.eq_ignore_ascii_case(listener_world) {
        return false;
    }
    let dx = speaker_pos.0 - listener_pos.0;
    let dy = speaker_pos.1 - listener_pos.1;
    let dz = speaker_pos.2 - listener_pos.2;
    dx * dx + dy * dy + dz * dz <= radius * radius
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    #[serde(default = "default_local_radius")]
    pub local_radius: f64,
    #[serde(default = "default_global_cd_ms")]
    pub global_cooldown_ms: u64,
    #[serde(default = "default_trade_cd_ms")]
    pub trade_cooldown_ms: u64,
    #[serde(default = "default_bubble_duration_ms")]
    pub bubble_duration_ms: u64,
    #[serde(default = "default_bubble_max_chars")]
    pub bubble_max_chars: usize,
}

fn default_local_radius() -> f64 { 100.0 }
fn default_global_cd_ms() -> u64 { 3_000 }
fn default_trade_cd_ms() -> u64 { 30_000 }
fn default_bubble_duration_ms() -> u64 { 4_500 }
fn default_bubble_max_chars() -> usize { 40 }

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            local_radius: default_local_radius(),
            global_cooldown_ms: default_global_cd_ms(),
            trade_cooldown_ms: default_trade_cd_ms(),
            bubble_duration_ms: default_bubble_duration_ms(),
            bubble_max_chars: default_bubble_max_chars(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatStore {
    #[serde(default)]
    pub config: ChatConfig,
    /// Черный список: `player_key -> set of ignored player_keys`.
    #[serde(default)]
    pub ignored: HashMap<String, HashSet<String>>,
    /// Администраторы с включённым `/socialspy`.
    #[serde(default)]
    pub social_spy: HashSet<String>,
}

impl ChatStore {
    pub fn is_ignoring(&self, receiver_name: &str, sender_name: &str) -> bool {
        let r_key = normalize_key(receiver_name);
        let s_key = normalize_key(sender_name);
        self.ignored.get(&r_key).is_some_and(|set| set.contains(&s_key))
    }

    /// Переключает игнор для пары игроков. Возвращает `true`, если игрок добавлен в ЧС.
    pub fn toggle_ignore(&mut self, owner_name: &str, target_name: &str) -> bool {
        let o_key = normalize_key(owner_name);
        let t_key = normalize_key(target_name);
        let entry = self.ignored.entry(o_key).or_default();
        if entry.contains(&t_key) {
            entry.remove(&t_key);
            false
        } else {
            entry.insert(t_key);
            true
        }
    }

    pub fn toggle_social_spy(&mut self, admin_name: &str) -> bool {
        let key = normalize_key(admin_name);
        if self.social_spy.contains(&key) {
            self.social_spy.remove(&key);
            false
        } else {
            self.social_spy.insert(key);
            true
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveBubble {
    pub player_key: String,
    pub entity_id: i32,
    pub world_name: String,
    pub expires_at_ms: u64,
}

pub struct ChatState {
    pub data_folder: String,
    pub store: ChatStore,
    pub last_global_ms: HashMap<String, u64>,
    pub last_trade_ms: HashMap<String, u64>,
    pub last_message: HashMap<String, String>,
    pub last_pm_partner: HashMap<String, String>,
    pub active_bubbles: HashMap<String, ActiveBubble>,
}

impl ChatState {
    pub fn new(data_folder: String, store: ChatStore) -> Self {
        Self {
            data_folder,
            store,
            last_global_ms: HashMap::new(),
            last_trade_ms: HashMap::new(),
            last_message: HashMap::new(),
            last_pm_partner: HashMap::new(),
            active_bubbles: HashMap::new(),
        }
    }
}
