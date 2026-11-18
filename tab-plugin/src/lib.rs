use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use pumpkin_plugin_api::{
    Context, Player, Plugin, PluginMetadata, Server,
    command::{ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType},
    commands::CommandHandler,
    events::{EventHandler, EventPriority, PlayerJoinEvent},
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    scheduler::SchedulerExt,
    text::TextComponent,
    wit::pumpkin::plugin::{command::Arg, event::PlayerJoinEventData},
};
use serde::{Deserialize, Serialize};

const PERM_PLAYER: &str = "tab-plugin:view";
const PERM_ADMIN: &str = "tab-plugin:admin";

const SHIMMER_FRAMES: [&str; 8] = [
    "§f§l✦ §6§lK§e§lR§e§lO§e§lN§e§lO§e§lS §7• §fВыживание 1.20.1+ §f§l✦",
    "§6§l✦ §f§lK§6§lR§e§lO§e§lN§e§lO§e§lS §7• §fВыживание 1.20.1+ §6§l✦",
    "§e§l✦ §6§lK§f§lR§6§lO§e§lN§e§lO§e§lS §7• §fВыживание 1.20.1+ §e§l✦",
    "§6§l✦ §e§lK§6§lR§f§lO§6§lN§e§lO§e§lS §7• §fВыживание 1.20.1+ §6§l✦",
    "§f§l✦ §e§lK§e§lR§6§lO§f§lN§6§lO§e§lS §7• §fВыживание 1.20.1+ §f§l✦",
    "§6§l✦ §e§lK§e§lR§e§lO§6§lN§f§lO§6§lS §7• §fВыживание 1.20.1+ §6§l✦",
    "§e§l✦ §e§lK§e§lR§e§lO§e§lN§6§lO§f§lS §7• §fВыживание 1.20.1+ §e§l✦",
    "§6§l✦ §6§lK§6§lR§6§lO§6§lN§6§lO§6§lS §7• §fВыживание 1.20.1+ §6§l✦",
];

const TIPS: [&str; 5] = [
    "§7Подсказка: §6/menu §8— §fглавное меню, настройки игрока и правила KRONOS",
    "§7Подсказка: §b/auth §8— §fличный кабинет, смена пароля и выход",
    "§7Подсказка: §a/kit §8— §fстартовое снаряжение выжившего",
    "§7Подсказка: §e/spawn §8— §fбыстрый телепорт на точку спавна",
    "§7Подсказка: §6/ranks §8— §fпосмотреть привилегии и ранги KRONOS",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rank {
    Owner,
    Admin,
    Moder,
    Kronos,
    Legend,
    Vip,
    Player,
}

impl Rank {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "owner" => Some(Self::Owner),
            "admin" => Some(Self::Admin),
            "moder" | "mod" => Some(Self::Moder),
            "kronos" => Some(Self::Kronos),
            "legend" => Some(Self::Legend),
            "vip" => Some(Self::Vip),
            "player" | "default" => Some(Self::Player),
            _ => None,
        }
    }

    fn as_key(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Moder => "moder",
            Self::Kronos => "kronos",
            Self::Legend => "legend",
            Self::Vip => "vip",
            Self::Player => "player",
        }
    }

    fn badge(self) -> &'static str {
        match self {
            Self::Owner => "§4§l[👑 ВЛАДЕЛЕЦ]",
            Self::Admin => "§c§l[⚡ АДМИН]",
            Self::Moder => "§9§l[🛡 МОДЕР]",
            Self::Kronos => "§6§l[✦ KRONOS]",
            Self::Legend => "§d§l[★ ЛЕГЕНДА]",
            Self::Vip => "§a§l[♦ VIP]",
            Self::Player => "§7[Игрок]",
        }
    }

    fn order(self) -> i32 {
        match self {
            Self::Owner => 100,
            Self::Admin => 90,
            Self::Moder => 80,
            Self::Kronos => 70,
            Self::Legend => 60,
            Self::Vip => 50,
            Self::Player => 10,
        }
    }

    fn chat_display(self) -> &'static str {
        match self {
            Self::Owner => "§4§l👑 ВЛАДЕЛЕЦ",
            Self::Admin => "§c§l⚡ АДМИН",
            Self::Moder => "§9§l🛡 МОДЕР",
            Self::Kronos => "§6§l✦ KRONOS",
            Self::Legend => "§d§l★ ЛЕГЕНДА",
            Self::Vip => "§a§l♦ VIP",
            Self::Player => "§7Игрок",
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct RanksDb {
    ranks: HashMap<String, String>,
}

struct TabState {
    data_folder: String,
    db: RanksDb,
}

static STATE: OnceLock<Mutex<TabState>> = OnceLock::new();
static TICK_FRAME: AtomicU64 = AtomicU64::new(0);

fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

fn init_state(data_folder: String) {
    let _ = fs::create_dir_all(&data_folder);
    let path = format!("{data_folder}/ranks.json");
    let db = match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<RanksDb>(&raw).unwrap_or_default(),
        Err(_) => RanksDb::default(),
    };
    let _ = STATE.set(Mutex::new(TabState { data_folder, db }));
}

fn with_state<R>(f: impl FnOnce(&mut TabState) -> R) -> R {
    let mutex = STATE.get_or_init(|| {
        Mutex::new(TabState {
            data_folder: ".".to_string(),
            db: RanksDb::default(),
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

impl TabState {
    fn save(&self) {
        let _ = fs::create_dir_all(&self.data_folder);
        let path = format!("{}/ranks.json", self.data_folder);
        if let Ok(json) = serde_json::to_string_pretty(&self.db) {
            let _ = fs::write(path, json);
        }
    }
}

fn resolve_player_rank(player: &Player) -> Rank {
    let key = player.get_name().to_lowercase();
    let explicit = with_state(|st| {
        st.db
            .ranks
            .get(&key)
            .and_then(|r| Rank::from_str(r))
    });
    if let Some(rank) = explicit {
        return rank;
    }
    match player.get_permission_level() {
        PermissionLevel::Four => Rank::Owner,
        PermissionLevel::Three => Rank::Admin,
        PermissionLevel::Two => Rank::Moder,
        _ => Rank::Player,
    }
}

fn format_clock(ticks_of_day: u64) -> String {
    // 0 ticks = 06:00 in Minecraft; 1000 ticks = 1 hour
    let total_mc_minutes = ((ticks_of_day % 24000) * 60) / 1000;
    let total_minutes = (360 + total_mc_minutes) % (24 * 60);
    let hours = total_minutes / 60;
    let mins = total_minutes % 60;
    format!("{hours:02}:{mins:02}")
}

fn dimension_labels(dim: &str) -> (&'static str, &'static str) {
    match dim {
        "minecraft:the_nether" => ("§c🔥 Нижний мир", "§c🔥"),
        "minecraft:the_end" => ("§d🌌 Край", "§d🌌"),
        _ => ("§a☀ Верхний мир", "§a☀"),
    }
}

fn ping_colored(ping: u32) -> String {
    if ping < 60 {
        format!("§a{ping}")
    } else if ping < 150 {
        format!("§e{ping}")
    } else {
        format!("§c{ping}")
    }
}

fn is_staff_viewer(player: &Player, rank: Rank) -> bool {
    matches!(rank, Rank::Owner | Rank::Admin | Rank::Moder)
        || matches!(
            player.get_permission_level(),
            PermissionLevel::Two | PermissionLevel::Three | PermissionLevel::Four
        )
        || player.has_permission(PERM_ADMIN)
}

fn update_single_player_tab(player: &Player, online_count: u32, tps: f64, mspt: f64, frame: u64) {
    let name = player.get_name();
    let ping = player.get_ping();
    let ping_str = ping_colored(ping);
    let rank = resolve_player_rank(player);

    let world = player.get_world();
    let dim = world.get_dimension();
    let (dim_full, dim_short) = dimension_labels(&dim);
    let clock = format_clock(world.get_time_of_day());
    let pos = player.get_position();
    let bx = pos.0.floor() as i32;
    let by = pos.1.floor() as i32;
    let bz = pos.2.floor() as i32;

    let locked = player.is_movement_locked();

    if locked {
        player.set_tab_list_name(Some(legacy(&format!("§c[🔒 Вход] §7{name}"))));
        player.set_tab_list_order(1);
    } else {
        player.set_tab_list_name(Some(legacy(&format!(
            "{} §f{name} {dim_short} §8[{ping_str}ms§8]",
            rank.badge()
        ))));
        player.set_tab_list_order(rank.order());
    }

    let shimmer = SHIMMER_FRAMES[(frame as usize) % SHIMMER_FRAMES.len()];
    let tip = TIPS[((frame as usize) / 10) % TIPS.len()];

    let header_str = format!(
        "\n{shimmer}\n\n§7Игрок: {} §f{name}  §8│  §7Пинг: {ping_str}ms\n§7Мир: {dim_full}  §8│  §7Время: §e🕒 {clock}  §8│  §7XYZ: §b{bx} {by} {bz}\n",
        rank.badge()
    );
    let footer_str = if is_staff_viewer(player, rank) {
        let tps_color = if tps >= 19.0 {
            "§a"
        } else if tps >= 15.0 {
            "§e"
        } else {
            "§c"
        };
        format!(
            "\n§7Онлайн: §a§l{online_count}§8/1000  §8│  §7TPS: {tps_color}{tps:.1} §8(§7{mspt:.1} mspt§8)\n{tip}\n"
        )
    } else {
        format!(
            "\n§7Онлайн: §a§l{online_count}§8/1000  §8│  §7Меню сервера: §6/menu §8(§7ПКМ Компасом§8)\n{tip}\n"
        )
    };

    player.set_tab_list_header_footer(legacy(&header_str), legacy(&footer_str));
}

fn update_all_tabs(server: &Server, frame: u64) {
    let players = server.get_all_players();
    if players.is_empty() {
        return;
    }
    let online_count = players.len() as u32;
    let tps = server.get_tps().min(20.0);
    let mspt = server.get_mspt();

    for p in players {
        update_single_player_tab(&p, online_count, tps, mspt, frame);
    }
}

fn tab_refresh_tick(server: Server) {
    let frame = TICK_FRAME.fetch_add(1, Ordering::Relaxed);
    update_all_tabs(&server, frame);
}

struct JoinTabHandler;
impl EventHandler<PlayerJoinEvent> for JoinTabHandler {
    fn handle(&self, server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let frame = TICK_FRAME.load(Ordering::Relaxed);
        let online_count = server.get_player_count().max(1);
        let tps = server.get_tps().min(20.0);
        let mspt = server.get_mspt();
        update_single_player_tab(&event.player, online_count, tps, mspt, frame);
        event
    }
}

struct RankCmd;
impl CommandHandler for RankCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let raw = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.len() < 2 {
            sender.send_error(legacy(
                "§eИспользование: §b/rank <ник> <owner|admin|moder|kronos|legend|vip|player>",
            ));
            return Ok(1);
        }

        let target_name = parts[0];
        let Some(rank) = Rank::from_str(parts[1]) else {
            sender.send_error(legacy(
                "§c✖ Неизвестный ранг. Доступны: §eowner, admin, moder, kronos, legend, vip, player",
            ));
            return Ok(1);
        };

        let key = target_name.to_lowercase();
        with_state(|st| {
            st.db.ranks.insert(key, rank.as_key().to_string());
            st.save();
        });

        let frame = TICK_FRAME.load(Ordering::Relaxed);
        update_all_tabs(&server, frame);

        sender.send_message(legacy(&format!(
            "§6§l✦ KRONOS §8» §aИгроку §f{target_name} §aвыдан ранг {}§a!",
            rank.chat_display()
        )));

        if let Some(target_player) = server.get_player_by_name(target_name) {
            target_player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS §8» §aВаш ранг обновлён на {}§a!",
                    rank.chat_display()
                )),
                false,
            );
        }
        Ok(0)
    }
}

struct RanksListCmd;
impl CommandHandler for RanksListCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        sender.send_system_message(legacy("§8§m────────────────────────────────────────"));
        sender.send_system_message(legacy("§6§l✦ РАНГИ СЕРВЕРА KRONOS:"));
        for r in [
            Rank::Owner,
            Rank::Admin,
            Rank::Moder,
            Rank::Kronos,
            Rank::Legend,
            Rank::Vip,
            Rank::Player,
        ] {
            sender.send_system_message(legacy(&format!(
                "  {} §8— §7код: §f{}",
                r.badge(),
                r.as_key()
            )));
        }
        sender.send_system_message(legacy("§8§m────────────────────────────────────────"));
        Ok(0)
    }
}

pub struct TabPlugin;

impl Plugin for TabPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "tab-plugin".into(),
            version: "0.1.0".into(),
            authors: vec!["UncleBug".into()],
            description: "Animated KRONOS Tab-list header/footer, live stats, and sorted player ranks".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_state(context.get_data_folder());

        context.register_permission(&Permission {
            node: PERM_PLAYER.to_string(),
            description: "View KRONOS server rank list".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        context.register_permission(&Permission {
            node: PERM_ADMIN.to_string(),
            description: "Manage player ranks on KRONOS".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: vec![],
        })?;

        let rank_cmd = Command::new(
            &["rank".into()],
            "Выдать ранг игроку (/rank <ник> <ранг>)",
        )
        .then(
            CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
                .execute(RankCmd),
        );
        context.register_command(rank_cmd, PERM_ADMIN);

        let ranks_cmd = Command::new(
            &["ranks".into()],
            "Список всех рангов сервера KRONOS",
        )
        .execute(RanksListCmd);
        context.register_command(ranks_cmd, PERM_PLAYER);

        context.register_event_handler::<PlayerJoinEvent, _>(
            JoinTabHandler,
            EventPriority::Normal,
            false,
        )?;

        context.schedule_repeating_task(10, 10, tab_refresh_tick);

        tracing::info!("KRONOS Tab & Ranks Plugin v0.1.0 initialized!");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        with_state(|st| st.save());
        tracing::info!("Tab Plugin unloaded.");
        Ok(())
    }
}

register_plugin!(TabPlugin);
