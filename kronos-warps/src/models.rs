use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const WARMUP_MS: u64 = 3_000;
pub const MOVE_CANCEL_DIST_SQ: f64 = 0.35 * 0.35;
pub const COIN_SCALE: u64 = 100;

pub fn normalize_key(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

/// Валидация имени варпа: 3..=16 символов, только `a-z`, `0-9`, `_`.
pub fn validate_warp_name(raw: &str) -> Result<String, WarpError> {
    let key = normalize_key(raw);
    if key.len() < 3 || key.len() > 16 {
        return Err(WarpError::InvalidName);
    }
    if !key
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(WarpError::InvalidName);
    }
    Ok(key)
}

/// Парсинг стоимости входа в сотые доли монеты (совместимо с `kronos-economy`).
pub fn parse_cost_minor(raw: &str) -> Result<u64, WarpError> {
    let s = raw.trim().replace(',', ".");
    if s.is_empty() || s.starts_with('-') || s.starts_with('+') {
        return Err(WarpError::InvalidCost);
    }
    let mut parts = s.split('.');
    let whole_str = parts.next().unwrap_or("");
    let frac_opt = parts.next();
    if parts.next().is_some()
        || whole_str.is_empty()
        || !whole_str.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(WarpError::InvalidCost);
    }
    let whole: u64 = whole_str.parse().map_err(|_| WarpError::InvalidCost)?;
    let whole_minor = whole
        .checked_mul(COIN_SCALE)
        .ok_or(WarpError::InvalidCost)?;
    let frac_minor = match frac_opt {
        None => 0,
        Some(f) => {
            if f.is_empty() || f.len() > 2 || !f.bytes().all(|b| b.is_ascii_digit()) {
                return Err(WarpError::InvalidCost);
            }
            let parsed: u64 = f.parse().map_err(|_| WarpError::InvalidCost)?;
            if f.len() == 1 {
                parsed * 10
            } else {
                parsed
            }
        }
    };
    whole_minor
        .checked_add(frac_minor)
        .ok_or(WarpError::InvalidCost)
}

pub fn format_cost_minor(minor: u64) -> String {
    if minor == 0 {
        return "Бесплатно".to_string();
    }
    let whole = minor / COIN_SCALE;
    let frac = minor % COIN_SCALE;
    if frac == 0 {
        format!("{whole} ⛃")
    } else {
        format!("{whole}.{frac:02} ⛃")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarpError {
    InvalidName,
    InvalidCost,
    NotFound(String),
    AlreadyExists(String),
    NotOwner,
    LimitReached { limit: usize },
    NoPermission(String),
    EconomyError(String),
}

impl fmt::Display for WarpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WarpError::InvalidName => {
                write!(f, "Имя варпа должно быть от 3 до 16 символов (только a-z, 0-9, _).")
            }
            WarpError::InvalidCost => {
                write!(f, "Некорректная цена входа. Укажите 0 или положительное число (например, 50 или 12.50).")
            }
            WarpError::NotFound(name) => write!(f, "Варп '{}' не найден.", name),
            WarpError::AlreadyExists(name) => write!(f, "Варп '{}' уже существует.", name),
            WarpError::NotOwner => write!(f, "Вы можете управлять только своими варпами."),
            WarpError::LimitReached { limit } => {
                write!(f, "Достигнут лимит личных варпов ({limit}).")
            }
            WarpError::NoPermission(perm) => {
                write!(f, "Недостаточно прав для входа на этот варп (требуется {perm}).")
            }
            WarpError::EconomyError(msg) => write!(f, "{msg}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarpLocation {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub pitch: f32,
}

impl WarpLocation {
    pub fn rounded(world: String, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) -> Self {
        Self {
            world,
            x: (x * 100.0).round() / 100.0,
            y: (y * 100.0).round() / 100.0,
            z: (z * 100.0).round() / 100.0,
            yaw,
            pitch,
        }
    }

    pub fn distance_sq_to(&self, pos: (f64, f64, f64)) -> f64 {
        let dx = self.x - pos.0;
        let dy = self.y - pos.1;
        let dz = self.z - pos.2;
        dx * dx + dy * dy + dz * dz
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WarpType {
    Server,
    Player,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Warp {
    pub name: String,
    pub warp_type: WarpType,
    pub location: WarpLocation,
    #[serde(default)]
    pub description: String,
    pub owner: String,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub cost_minor: u64,
    #[serde(default)]
    pub visits: u64,
    #[serde(default)]
    pub created_at_ms: u64,
}

pub type PlayerWarp = Warp;

impl Warp {
    /// Проверяет, имеет ли игрок доступ к варпу на основе функции проверки прав.
    pub fn can_access<F>(&self, is_admin: bool, has_perm: F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        if is_admin {
            return true;
        }
        match self.permission.as_deref() {
            None | Some("") => true,
            Some(node) => has_perm(node),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WarpStore {
    #[serde(default)]
    pub warps: HashMap<String, Warp>,
}

impl WarpStore {
    pub fn set_server_warp(
        &mut self,
        raw_name: &str,
        location: WarpLocation,
        description: String,
        permission: Option<String>,
        author: &str,
        now_ms: u64,
    ) -> Result<Warp, WarpError> {
        let name = validate_warp_name(raw_name)?;
        let prev_visits = self.warps.get(&name).map(|w| w.visits).unwrap_or(0);
        let warp = Warp {
            name: name.clone(),
            warp_type: WarpType::Server,
            location,
            description,
            owner: author.to_string(),
            permission: permission.filter(|p| !p.trim().is_empty()),
            cost_minor: 0,
            visits: prev_visits,
            created_at_ms: now_ms,
        };
        self.warps.insert(name, warp.clone());
        Ok(warp)
    }

    pub fn delete_server_warp(&mut self, raw_name: &str) -> Result<Warp, WarpError> {
        let key = normalize_key(raw_name);
        match self.warps.get(&key) {
            Some(w) if w.warp_type == WarpType::Server => Ok(self.warps.remove(&key).unwrap()),
            _ => Err(WarpError::NotFound(raw_name.to_string())),
        }
    }

    pub fn create_player_warp(
        &mut self,
        raw_name: &str,
        owner: &str,
        location: WarpLocation,
        cost_minor: u64,
        max_limit: usize,
        now_ms: u64,
    ) -> Result<PlayerWarp, WarpError> {
        let name = validate_warp_name(raw_name)?;
        if self.warps.contains_key(&name) {
            return Err(WarpError::AlreadyExists(name));
        }
        let owned = self.player_warps_of(owner).len();
        if owned >= max_limit {
            return Err(WarpError::LimitReached { limit: max_limit });
        }
        let warp = Warp {
            name: name.clone(),
            warp_type: WarpType::Player,
            location,
            description: format!("Варп игрока {owner}"),
            owner: owner.to_string(),
            permission: None,
            cost_minor,
            visits: 0,
            created_at_ms: now_ms,
        };
        self.warps.insert(name, warp.clone());
        Ok(warp)
    }

    pub fn delete_player_warp(
        &mut self,
        raw_name: &str,
        actor: &str,
        is_admin: bool,
    ) -> Result<PlayerWarp, WarpError> {
        let key = normalize_key(raw_name);
        let Some(existing) = self.warps.get(&key) else {
            return Err(WarpError::NotFound(raw_name.to_string()));
        };
        if existing.warp_type != WarpType::Player {
            return Err(WarpError::NotOwner);
        }
        if !is_admin && normalize_key(&existing.owner) != normalize_key(actor) {
            return Err(WarpError::NotOwner);
        }
        Ok(self.warps.remove(&key).unwrap())
    }

    pub fn set_player_warp_cost(
        &mut self,
        raw_name: &str,
        actor: &str,
        is_admin: bool,
        cost_minor: u64,
    ) -> Result<PlayerWarp, WarpError> {
        let key = normalize_key(raw_name);
        let Some(existing) = self.warps.get_mut(&key) else {
            return Err(WarpError::NotFound(raw_name.to_string()));
        };
        if existing.warp_type != WarpType::Player {
            return Err(WarpError::NotOwner);
        }
        if !is_admin && normalize_key(&existing.owner) != normalize_key(actor) {
            return Err(WarpError::NotOwner);
        }
        existing.cost_minor = cost_minor;
        Ok(existing.clone())
    }

    pub fn record_visit(&mut self, raw_name: &str) -> Option<u64> {
        let key = normalize_key(raw_name);
        let warp = self.warps.get_mut(&key)?;
        warp.visits = warp.visits.saturating_add(1);
        Some(warp.visits)
    }

    pub fn get(&self, raw_name: &str) -> Option<&Warp> {
        self.warps.get(&normalize_key(raw_name))
    }

    pub fn server_warps(&self) -> Vec<Warp> {
        let mut list: Vec<Warp> = self
            .warps
            .values()
            .filter(|w| w.warp_type == WarpType::Server)
            .cloned()
            .collect();
        list.sort_by(|a, b| b.visits.cmp(&a.visits).then_with(|| a.name.cmp(&b.name)));
        list
    }

    pub fn popular_player_warps(&self, limit: usize) -> Vec<PlayerWarp> {
        let mut list: Vec<PlayerWarp> = self
            .warps
            .values()
            .filter(|w| w.warp_type == WarpType::Player)
            .cloned()
            .collect();
        list.sort_by(|a, b| b.visits.cmp(&a.visits).then_with(|| a.name.cmp(&b.name)));
        list.truncate(limit);
        list
    }

    pub fn player_warps_of(&self, owner: &str) -> Vec<PlayerWarp> {
        let owner_key = normalize_key(owner);
        let mut list: Vec<PlayerWarp> = self
            .warps
            .values()
            .filter(|w| w.warp_type == WarpType::Player && normalize_key(&w.owner) == owner_key)
            .cloned()
            .collect();
        list.sort_by(|a, b| b.visits.cmp(&a.visits).then_with(|| a.name.cmp(&b.name)));
        list
    }
}

#[derive(Debug, Clone)]
pub struct PendingWarmup {
    pub player_name: String,
    pub warp_name: String,
    pub origin: WarpLocation,
    pub ready_at_ms: u64,
    pub last_notified_sec: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogTab {
    Server,
    Popular,
    Mine,
}

#[derive(Debug, Clone)]
pub enum BedrockWarpAction {
    OpenTab(CatalogTab),
    TeleportTo(String),
}

#[derive(Debug, Clone)]
pub struct PendingBedrockWarpForm {
    pub player_key: String,
    pub actions: Vec<BedrockWarpAction>,
}

pub struct WarpsState {
    pub data_folder: String,
    pub store: WarpStore,
    pub pending_warmups: HashMap<String, PendingWarmup>,
    pub pending_forms: HashMap<u32, PendingBedrockWarpForm>,
}

impl WarpsState {
    pub fn new(data_folder: String, store: WarpStore) -> Self {
        Self {
            data_folder,
            store,
            pending_warmups: HashMap::new(),
            pending_forms: HashMap::new(),
        }
    }
}
