use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub fn normalize_key(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

/// Парсинг срока действия ранга: `"perm"` / `"forever"` -> `Ok(None)`,
/// `"30d"`, `"90d"`, `"12h"`, `"45m"`, `"60s"` -> `Ok(Some(duration_ms))`.
pub fn parse_duration_ms(raw: &str) -> Result<Option<u64>, String> {
    let s = normalize_key(raw);
    if s.is_empty() || matches!(s.as_str(), "perm" | "permanent" | "forever" | "inf") {
        return Ok(None);
    }
    let (num_part, unit) = s.split_at(s.len().saturating_sub(1));
    let value: u64 = num_part
        .parse()
        .map_err(|_| format!("Некорректный срок '{raw}'. Пример: 30d, 90d, 12h, perm"))?;
    if value == 0 {
        return Err("Срок действия должен быть больше 0.".to_string());
    }
    let mult_ms: u64 = match unit {
        "s" => 1_000,
        "m" => 60 * 1_000,
        "h" => 3_600 * 1_000,
        "d" => 86_400 * 1_000,
        _ => return Err(format!("Неизвестная единица срока '{unit}'. Используйте s, m, h, d или perm.")),
    };
    value
        .checked_mul(mult_ms)
        .map(Some)
        .ok_or_else(|| "Срок действия слишком велик.".to_string())
}

pub fn format_remaining_ru(expires_at_ms: Option<u64>, now_ms: u64) -> String {
    let Some(exp) = expires_at_ms else {
        return "Навсегда".to_string();
    };
    if exp <= now_ms {
        return "Истёк".to_string();
    }
    let secs = (exp - now_ms) / 1000;
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}д {hours}ч")
    } else if hours > 0 {
        format!("{hours}ч {mins}м")
    } else {
        format!("{}м", mins.max(1))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RankTrack {
    Donate,
    Staff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rank {
    pub id: String,
    pub display_name: String,
    pub track: RankTrack,
    pub weight: u32,
    pub prefix: String,
    pub name_color: String,
    #[serde(default)]
    pub inherits: Option<String>,
    #[serde(default)]
    pub max_homes: u32,
    #[serde(default)]
    pub multiplier_x10: u32,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRankData {
    pub player_key: String,
    pub player_name: String,
    pub donate_rank: String,
    #[serde(default)]
    pub donate_expires_at_ms: Option<u64>,
    #[serde(default)]
    pub staff_rank: Option<String>,
    #[serde(default)]
    pub staff_expires_at_ms: Option<u64>,
    #[serde(default)]
    pub extra_permissions: Vec<String>,
    #[serde(default)]
    pub denied_permissions: Vec<String>,
}

impl PlayerRankData {
    pub fn new(player_name: &str) -> Self {
        Self {
            player_key: normalize_key(player_name),
            player_name: player_name.trim().to_string(),
            donate_rank: "player".to_string(),
            donate_expires_at_ms: None,
            staff_rank: None,
            staff_expires_at_ms: None,
            extra_permissions: Vec::new(),
            denied_permissions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionTree {
    pub ranks: HashMap<String, Rank>,
}

impl Default for PermissionTree {
    fn default() -> Self {
        default_permission_tree()
    }
}

impl PermissionTree {
    pub fn get(&self, id: &str) -> Option<&Rank> {
        self.ranks.get(&normalize_key(id))
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Rank> {
        self.ranks.get_mut(&normalize_key(id))
    }

    /// Возвращает все права ранга с учётом рекурсивного наследования (с защитой от циклов).
    pub fn inherited_permissions(&self, rank_id: &str) -> HashSet<String> {
        let mut out = HashSet::new();
        let mut visited = HashSet::new();
        let mut current = Some(normalize_key(rank_id));

        while let Some(id) = current {
            if !visited.insert(id.clone()) {
                break;
            }
            if let Some(rank) = self.ranks.get(&id) {
                for perm in &rank.permissions {
                    out.insert(perm.clone());
                }
                current = rank.inherits.as_deref().map(normalize_key);
            } else {
                break;
            }
        }
        out
    }

    /// Полный набор эффективных прав игрока (Донат + Стафф + extra - denied).
    pub fn effective_permissions(&self, user: &PlayerRankData) -> HashSet<String> {
        let mut perms = self.inherited_permissions(&user.donate_rank);
        if let Some(ref staff_id) = user.staff_rank {
            perms.extend(self.inherited_permissions(staff_id));
        }
        for p in &user.extra_permissions {
            perms.insert(p.clone());
        }
        for d in &user.denied_permissions {
            perms.remove(d);
        }
        perms
    }

    pub fn has_permission(&self, user: &PlayerRankData, node: &str) -> bool {
        if user.denied_permissions.iter().any(|d| d == node) {
            return false;
        }
        let perms = self.effective_permissions(user);
        perms.contains("*") || perms.contains(node)
    }

    /// Выбирает главный отображаемый ранг по максимальному весу (Dual-Track).
    pub fn primary_rank<'a>(&'a self, user: &PlayerRankData) -> &'a Rank {
        let donate = self.get(&user.donate_rank);
        let staff = user.staff_rank.as_deref().and_then(|s| self.get(s));
        match (donate, staff) {
            (Some(d), Some(s)) => if s.weight >= d.weight { s } else { d },
            (None, Some(s)) => s,
            (Some(d), None) => d,
            (None, None) => self.ranks.get("player").expect("default player rank must exist"),
        }
    }

    pub fn is_staff(&self, user: &PlayerRankData) -> bool {
        user.staff_rank
            .as_deref()
            .and_then(|id| self.get(id))
            .is_some_and(|r| r.track == RankTrack::Staff)
    }

    pub fn donate_ranks_sorted(&self) -> Vec<Rank> {
        let mut list: Vec<Rank> = self
            .ranks
            .values()
            .filter(|r| r.track == RankTrack::Donate)
            .cloned()
            .collect();
        list.sort_by_key(|r| r.weight);
        list
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsersStore {
    #[serde(default)]
    pub users: HashMap<String, PlayerRankData>,
}

impl UsersStore {
    pub fn ensure_user(&mut self, player_name: &str) -> (&mut PlayerRankData, bool) {
        let key = normalize_key(player_name);
        let existed = self.users.contains_key(&key);
        let entry = self
            .users
            .entry(key)
            .or_insert_with(|| PlayerRankData::new(player_name));
        if !player_name.trim().is_empty() {
            entry.player_name = player_name.trim().to_string();
        }
        (entry, !existed)
    }

    pub fn get_or_default(&self, player_name: &str) -> PlayerRankData {
        self.users
            .get(&normalize_key(player_name))
            .cloned()
            .unwrap_or_else(|| PlayerRankData::new(player_name))
    }

    /// Проверяет и автоматически снимает истёкшие временные ранги.
    /// Возвращает список `(player_name, expired_rank_id)`.
    pub fn expire_ranks(&mut self, now_ms: u64) -> Vec<(String, String)> {
        let mut expired = Vec::new();
        for user in self.users.values_mut() {
            if let Some(exp) = user.donate_expires_at_ms {
                if exp <= now_ms && user.donate_rank != "player" {
                    let old = user.donate_rank.clone();
                    user.donate_rank = "player".to_string();
                    user.donate_expires_at_ms = None;
                    expired.push((user.player_name.clone(), old));
                }
            }
            if let Some(exp) = user.staff_expires_at_ms {
                if exp <= now_ms {
                    if let Some(old) = user.staff_rank.take() {
                        user.staff_expires_at_ms = None;
                        expired.push((user.player_name.clone(), old));
                    }
                }
            }
        }
        expired
    }
}

pub fn default_permission_tree() -> PermissionTree {
    let items = vec![
        Rank {
            id: "player".into(), display_name: "Игрок".into(), track: RankTrack::Donate,
            weight: 0, prefix: "§7[Игрок] ".into(), name_color: "§f".into(), inherits: None,
            max_homes: 1, multiplier_x10: 10, summary: "Базовый ранг: 1 дом".into(),
            permissions: vec!["kronos-ranks:player".into(), "kronos-essentials:homes.1".into()],
        },
        Rank {
            id: "vip".into(), display_name: "VIP".into(), track: RankTrack::Donate,
            weight: 10, prefix: "§a[VIP] ".into(), name_color: "§a".into(), inherits: Some("player".into()),
            max_homes: 3, multiplier_x10: 12, summary: "1.2x множитель, 3 дома, RTP VIP, чат &a/&b".into(),
            permissions: vec![
                "kronos-essentials:homes.3".into(), "kronos-essentials:rtp.vip".into(),
                "kronos-ranks:chat.color.vip".into(), "kronos-warps:vip".into(),
            ],
        },
        Rank {
            id: "premium".into(), display_name: "Премиум".into(), track: RankTrack::Donate,
            weight: 25, prefix: "§b[Премиум] ".into(), name_color: "§b".into(), inherits: Some("vip".into()),
            max_homes: 5, multiplier_x10: 13, summary: "5 домов, /feed, /workbench (/craft)".into(),
            permissions: vec![
                "kronos-essentials:homes.5".into(), "kronos-ranks:cmd.feed".into(),
                "kronos-ranks:cmd.workbench".into(),
            ],
        },
        Rank {
            id: "creative".into(), display_name: "Креатив".into(), track: RankTrack::Donate,
            weight: 50, prefix: "§d[Креатив] ".into(), name_color: "§d".into(), inherits: Some("premium".into()),
            max_homes: 7, multiplier_x10: 15, summary: "7 домов, /fly, /hat".into(),
            permissions: vec![
                "kronos-essentials:homes.7".into(), "kronos-ranks:cmd.fly".into(),
                "kronos-ranks:cmd.hat".into(),
            ],
        },
        Rank {
            id: "hero".into(), display_name: "Герой".into(), track: RankTrack::Donate,
            weight: 80, prefix: "§e[Герой] ".into(), name_color: "§e".into(), inherits: Some("creative".into()),
            max_homes: 10, multiplier_x10: 17, summary: "10 домов, /heal, /enderchest (/ec)".into(),
            permissions: vec![
                "kronos-essentials:homes.10".into(), "kronos-ranks:cmd.heal".into(),
                "kronos-ranks:cmd.enderchest".into(),
            ],
        },
        Rank {
            id: "titan".into(), display_name: "Титан".into(), track: RankTrack::Donate,
            weight: 120, prefix: "§6[Титан] ".into(), name_color: "§6".into(), inherits: Some("hero".into()),
            max_homes: 15, multiplier_x10: 20, summary: "15 домов, /near, /repair, элитные варпы".into(),
            permissions: vec![
                "kronos-essentials:homes.15".into(), "kronos-ranks:cmd.near".into(),
                "kronos-ranks:cmd.repair".into(), "kronos-warps:pwarp.elite".into(),
                "kronos-warps:elite".into(),
            ],
        },
        Rank {
            id: "dragon".into(), display_name: "Дракон".into(), track: RankTrack::Donate,
            weight: 200, prefix: "§c[Дракон] ".into(), name_color: "§c".into(), inherits: Some("titan".into()),
            max_homes: 25, multiplier_x10: 25, summary: "25 домов, обход задержки /warp и /home".into(),
            permissions: vec![
                "kronos-essentials:homes.25".into(), "kronos-warps:bypass-warmup".into(),
                "kronos-essentials:bypass-warmup".into(),
            ],
        },
        Rank {
            id: "sponsor".into(), display_name: "Спонсор".into(), track: RankTrack::Donate,
            weight: 500, prefix: "§4§l[СПОНСОР] ".into(), name_color: "§f".into(), inherits: Some("dragon".into()),
            max_homes: 50, multiplier_x10: 30, summary: "50 домов, все команды, все цвета и стили чата".into(),
            permissions: vec![
                "kronos-essentials:homes.50".into(), "kronos-ranks:chat.color.all".into(),
                "kronos-ranks:chat.style.all".into(),
            ],
        },
        Rank {
            id: "helper".into(), display_name: "Хелпер".into(), track: RankTrack::Staff,
            weight: 600, prefix: "§9[Хелпер] ".into(), name_color: "§9".into(), inherits: None,
            max_homes: 10, multiplier_x10: 15, summary: "Стафф-чат /sc, базовая модерация".into(),
            permissions: vec!["kronos-ranks:staff".into(), "kronos-ranks:sc".into()],
        },
        Rank {
            id: "moderator".into(), display_name: "Модератор".into(), track: RankTrack::Staff,
            weight: 750, prefix: "§3[Модератор] ".into(), name_color: "§3".into(), inherits: Some("helper".into()),
            max_homes: 15, multiplier_x10: 20, summary: "Расширенная модерация и проверки".into(),
            permissions: vec!["kronos-ranks:cmd.fly".into(), "kronos-warps:bypass-warmup".into()],
        },
        Rank {
            id: "admin".into(), display_name: "Админ".into(), track: RankTrack::Staff,
            weight: 900, prefix: "§c§l[АДМИН] ".into(), name_color: "§c".into(), inherits: Some("moderator".into()),
            max_homes: 50, multiplier_x10: 30, summary: "Администрирование рангов и прав".into(),
            permissions: vec!["kronos-ranks:admin".into(), "kronos-ranks:chat.color.all".into()],
        },
        Rank {
            id: "owner".into(), display_name: "Создатель".into(), track: RankTrack::Staff,
            weight: 1000, prefix: "§4§l[СОЗДАТЕЛЬ] ".into(), name_color: "§4".into(), inherits: Some("admin".into()),
            max_homes: 100, multiplier_x10: 50, summary: "Полный доступ * ко всем модулям сервера".into(),
            permissions: vec!["*".into()],
        },
    ];

    let mut ranks = HashMap::new();
    for rank in items {
        ranks.insert(rank.id.clone(), rank);
    }
    PermissionTree { ranks }
}

#[derive(Debug, Clone)]
pub enum BedrockRanksAction {
    ShowRankDetails(String),
    RefreshCatalog,
}

#[derive(Debug, Clone)]
pub struct PendingBedrockForm {
    pub player_key: String,
    pub actions: Vec<BedrockRanksAction>,
}

pub struct RanksState {
    pub data_folder: String,
    pub tree: PermissionTree,
    pub users: UsersStore,
    pub tick_counter: u64,
    pub pending_forms: HashMap<u32, PendingBedrockForm>,
}

impl RanksState {
    pub fn new(data_folder: String, tree: PermissionTree, users: UsersStore) -> Self {
        Self {
            data_folder,
            tree,
            users,
            tick_counter: 0,
            pending_forms: HashMap::new(),
        }
    }
}
