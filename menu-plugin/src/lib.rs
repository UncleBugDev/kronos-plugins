pub mod store;
pub mod ui;

use pumpkin_plugin_api::{
    Context, Player, Plugin, PluginMetadata, Server,
    command::{Command, CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    common::Hand,
    events::{
        BedrockFormResponseEvent, EventHandler, EventPriority, InteractAction,
        InventoryClickEvent, InventoryCloseEvent, PlayerChatEvent, PlayerInteractEvent,
        PlayerLeaveEvent, PlayerRespawnEvent, ServerLoadEvent,
    },
    forms::FormResponse,
    permission::{Permission, PermissionDefault},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    scheduler::SchedulerExt,
    uuid::Uuid,
    wit::pumpkin::plugin::{
        event::{
            BedrockFormResponseEventData, InventoryClickEventData, InventoryCloseEventData,
            PlayerChatEventData, PlayerInteractEventData, PlayerLeaveEventData,
            PlayerRespawnEventData, ServerLoadEventData,
        },
        sounds::SoundCategory,
    },
};

use store::{
    BedrockFormKind, MenuPage, PlayerPrefs, TimePref, WeatherPref, init_state,
    read_player_balance_display, save_current_db, unix_ms_now, with_state,
};
use ui::{
    CompassGiveOutcome, apply_time_pref, apply_visibility_pref, apply_weather_pref,
    bedrock::open_bedrock_menu_page,
    chat_guide::{handle_chat_guide_click, handle_economy_click},
    give_menu_compass_non_destructive, handle_safe_spawn_teleport, is_menu_compass, legacy,
    nav::handle_nav_click,
    open_java_menu_page,
    ranks::handle_ranks_click,
    same_uuid, send_rule_card_to_chat, send_security_cabinet_guide, send_server_info_to_chat,
    settings::handle_settings_click,
    show_hud_compass_actionbar,
};

const PERM_USE: &str = "menu-plugin:use";
const INTERACT_DEBOUNCE_MS: u128 = 150;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "menu-plugin".into(),
            version: "0.2.0".into(),
            authors: vec!["UncleBug".into(), "KRONOS Team".into()],
            description: "KRONOS Server Menu Compass, Navigation Hub, Economy, Chat Guide, 12 Ranks & Player Settings".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_state(context.get_data_folder());

        context.register_permission(&Permission {
            node: PERM_USE.to_string(),
            description: "Access to KRONOS server menu, settings, and rules".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        register_commands(&context);

        context.register_event_handler::<ServerLoadEvent, _>(ServerLoadHandler, EventPriority::Normal, false)?;
        context.register_event_handler::<PlayerRespawnEvent, _>(RespawnHandler, EventPriority::Normal, false)?;
        context.register_event_handler::<PlayerLeaveEvent, _>(LeaveHandler, EventPriority::Normal, false)?;
        context.register_event_handler::<PlayerInteractEvent, _>(InteractHandler, EventPriority::Normal, true)?;
        context.register_event_handler::<InventoryCloseEvent, _>(InvCloseHandler, EventPriority::Normal, false)?;
        context.register_event_handler::<InventoryClickEvent, _>(InvClickHandler, EventPriority::Normal, true)?;
        context.register_event_handler::<PlayerChatEvent, _>(ChatMentionHandler, EventPriority::Low, true)?;
        context.register_event_handler::<BedrockFormResponseEvent, _>(BedrockFormHandler, EventPriority::Normal, true)?;

        context.schedule_delayed_task(1, menu_tick);
        context.schedule_repeating_task(20, 20, menu_tick);

        tracing::info!("KRONOS Menu Plugin v0.2.0 loaded (modular UI + 5 KRONOS plugins integration)!");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_current_db();
        with_state(|st| {
            st.open_pages.clear();
            st.bedrock_forms.clear();
            st.delivered_this_session.clear();
            st.last_interact_ms.clear();
        });
        tracing::info!("KRONOS Menu Plugin unloaded cleanly.");
        Ok(())
    }
}

register_plugin!(MenuPlugin);

fn register_commands(context: &Context) {
    context.register_command(
        Command::new(&["menu".into(), "m".into()], "Открыть главное меню KRONOS (/menu)")
            .execute(OpenPageCmd(MenuPage::Main)),
        PERM_USE,
    );
    context.register_command(
        Command::new(&["settings".into()], "Открыть персональные настройки (/settings)")
            .execute(OpenPageCmd(MenuPage::Settings)),
        PERM_USE,
    );
    context.register_command(
        Command::new(&["rules".into()], "Открыть правила сервера KRONOS (/rules)")
            .execute(OpenPageCmd(MenuPage::Rules)),
        PERM_USE,
    );
}

fn open_page_for_player(player: &Player, page: MenuPage) {
    if player.is_movement_locked() {
        return;
    }

    let uuid = player.get_id().to_string();
    let name = player.get_name();
    let key = name.to_lowercase();
    let (prefs, data_folder) = with_state(|st| (st.prefs_for(&key), st.data_folder.clone()));
    let balance = read_player_balance_display(&data_folder, &name);

    if player.as_bedrock().is_some() {
        if let Some((fid, kind)) = open_bedrock_menu_page(player, page, &prefs, &balance) {
            with_state(|st| {
                st.bedrock_forms.insert(uuid, (fid, kind));
            });
        }
    } else {
        open_java_menu_page(player, page, &prefs, &balance);
        if !player.is_movement_locked() {
            with_state(|st| {
                st.open_pages.insert(uuid, page);
            });
        }
    }
}

fn handle_manual_compass_claim(player: &Player, refresh_main: bool) {
    match give_menu_compass_non_destructive(player) {
        CompassGiveOutcome::Locked => {}
        CompassGiveOutcome::PlacedInSlot(_) => {
            player.play_custom_sound("minecraft:entity.experience_orb.pickup", SoundCategory::Ui, 0.85, 1.25);
            player.send_system_message(legacy("§a✔ Компас меню KRONOS добавлен в ваш инвентарь!"), false);
            if refresh_main {
                open_page_for_player(player, MenuPage::Main);
            }
        }
        CompassGiveOutcome::AlreadyPresent => {
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.8, 1.2);
            player.send_system_message(legacy("§e🧭 Компас меню KRONOS уже есть в вашем инвентаре!"), false);
        }
        CompassGiveOutcome::InventoryFull => {
            player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Ui, 0.85, 1.0);
            player.send_system_message(legacy("§c✖ В инвентаре нет свободного слота!"), false);
        }
    }
}

fn menu_tick(server: Server) {
    let players = server.get_all_players();
    if players.is_empty() {
        return;
    }

    let snapshots: Vec<(usize, Uuid, String, String, bool)> = players
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let id = p.get_id();
            (idx, id, id.to_string(), p.get_name().to_lowercase(), p.is_movement_locked())
        })
        .collect();

    let (plans, solitude_viewers) = with_state(|st| {
        let mut plans: Vec<(usize, Uuid, bool, PlayerPrefs)> = Vec::new();
        let mut viewers: Vec<Uuid> = Vec::new();
        for (idx, uuid_raw, uuid, key, locked) in snapshots {
            if locked {
                st.open_pages.remove(&uuid);
                st.delivered_this_session.remove(&uuid);
                continue;
            }
            let newly_auth = st.delivered_this_session.insert(uuid);
            let prefs = st.prefs_for(&key);
            if prefs.hide_players {
                viewers.push(uuid_raw);
            }
            plans.push((idx, uuid_raw, newly_auth, prefs));
        }
        (plans, viewers)
    });

    for (idx, uuid_raw, newly_auth, prefs) in plans {
        let player = &players[idx];
        if newly_auth {
            apply_weather_pref(player, prefs.weather);
            apply_time_pref(player, prefs.time);
            if prefs.hide_players {
                apply_visibility_pref(&server, player, true);
            }
            for viewer_uuid in &solitude_viewers {
                if !same_uuid(*viewer_uuid, uuid_raw)
                    && let (Some(viewer), Some(target)) = (
                        server.get_player_by_uuid(*viewer_uuid),
                        server.get_player_by_uuid(uuid_raw),
                    )
                {
                    viewer.hide_player(target);
                }
            }
            if prefs.auto_compass {
                let _ = give_menu_compass_non_destructive(player);
            }
        }
        if prefs.hud_compass {
            show_hud_compass_actionbar(player);
        }
    }
}

struct OpenPageCmd(MenuPage);
impl CommandHandler for OpenPageCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("Команда доступна только игрокам."));
            return Ok(1);
        };
        if player.is_movement_locked() {
            player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Ui, 0.85, 1.0);
            player.show_actionbar(legacy("§c🔒 Сначала авторизуйтесь! Введите пароль или §e/auth"));
            return Ok(1);
        }
        player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15);
        open_page_for_player(&player, self.0);
        Ok(0)
    }
}

struct ServerLoadHandler;
impl EventHandler<ServerLoadEvent> for ServerLoadHandler {
    fn handle(&self, server: Server, event: ServerLoadEventData) -> ServerLoadEventData {
        server.schedule_delayed_task(1, menu_tick);
        event
    }
}

struct RespawnHandler;
impl EventHandler<PlayerRespawnEvent> for RespawnHandler {
    fn handle(&self, server: Server, event: PlayerRespawnEventData) -> PlayerRespawnEventData {
        let uuid_raw = event.player.get_id();
        server.schedule_delayed_task(10, move |srv| {
            let Some(p) = srv.get_player_by_uuid(uuid_raw) else { return; };
            if p.is_movement_locked() { return; }
            let prefs = with_state(|st| st.prefs_for(&p.get_name().to_lowercase()));
            apply_weather_pref(&p, prefs.weather);
            apply_time_pref(&p, prefs.time);
            if prefs.auto_compass {
                let _ = give_menu_compass_non_destructive(&p);
            }
        });
        event
    }
}

struct LeaveHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(&self, _server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let uuid = event.player.get_id().to_string();
        with_state(|st| {
            st.open_pages.remove(&uuid);
            st.bedrock_forms.remove(&uuid);
            st.delivered_this_session.remove(&uuid);
            st.last_interact_ms.remove(&uuid);
        });
        event
    }
}

struct InteractHandler;
impl EventHandler<PlayerInteractEvent> for InteractHandler {
    fn handle(&self, _server: Server, mut event: PlayerInteractEventData) -> PlayerInteractEventData {
        if event.cancelled || event.player.is_movement_locked() {
            return event;
        }
        if !matches!(event.action, InteractAction::RightClickAir | InteractAction::RightClickBlock) {
            return event;
        }
        let in_main = event.player.get_item_in_hand(Hand::Right).is_some_and(|s| is_menu_compass(&s));
        let in_off = !in_main && event.player.get_item_in_hand(Hand::Left).is_some_and(|s| is_menu_compass(&s));
        if !in_main && !in_off {
            return event;
        }

        event.cancelled = true;
        let uuid = event.player.get_id().to_string();
        let now_ms = unix_ms_now();
        let should_open = with_state(|st| {
            if let Some(prev) = st.last_interact_ms.get(&uuid)
                && now_ms.saturating_sub(*prev) < INTERACT_DEBOUNCE_MS
            {
                return false;
            }
            st.last_interact_ms.insert(uuid, now_ms);
            true
        });

        if should_open {
            event.player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15);
            open_page_for_player(&event.player, MenuPage::Main);
        }
        event
    }
}

struct InvCloseHandler;
impl EventHandler<InventoryCloseEvent> for InvCloseHandler {
    fn handle(&self, _server: Server, event: InventoryCloseEventData) -> InventoryCloseEventData {
        let uuid = event.player.get_id().to_string();
        with_state(|st| { st.open_pages.remove(&uuid); });
        event
    }
}

struct InvClickHandler;
impl EventHandler<InventoryClickEvent> for InvClickHandler {
    fn handle(&self, server: Server, mut event: InventoryClickEventData) -> InventoryClickEventData {
        let player = &event.player;
        if player.is_movement_locked() {
            return event;
        }
        let uuid = player.get_id().to_string();
        let Some(page) = with_state(|st| st.open_pages.get(&uuid).copied()) else {
            return event;
        };

        event.cancelled = true;
        if event.raw_slot < 0 || event.raw_slot >= 54 || event.raw_slot != event.slot {
            return event;
        }

        let next_page = match page {
            MenuPage::Main => handle_main_click(player, event.slot),
            MenuPage::Navigation => handle_nav_click(player, event.slot),
            MenuPage::Economy => {
                let folder = with_state(|st| st.data_folder.clone());
                let bal = read_player_balance_display(&folder, &player.get_name());
                handle_economy_click(player, event.slot, &bal)
            }
            MenuPage::ChatGuide => handle_chat_guide_click(player, event.slot),
            MenuPage::RanksInfo => handle_ranks_click(player, event.slot),
            MenuPage::Settings => handle_settings_click(&server, player, event.slot),
            MenuPage::Rules => handle_rules_click(player, event.slot),
            MenuPage::ServerInfo => handle_server_info_click(player, event.slot),
        };

        if let Some(target_page) = next_page {
            open_page_for_player(player, target_page);
        }
        event
    }
}

fn handle_main_click(player: &Player, slot: i16) -> Option<MenuPage> {
    match slot {
        4 => {
            player.play_custom_sound("minecraft:entity.experience_orb.pickup", SoundCategory::Ui, 0.75, 1.35);
            Some(MenuPage::Main)
        }
        20 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15); Some(MenuPage::Navigation) }
        22 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15); Some(MenuPage::Economy) }
        24 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15); Some(MenuPage::ChatGuide) }
        29 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15); Some(MenuPage::RanksInfo) }
        31 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15); Some(MenuPage::Settings) }
        33 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15); Some(MenuPage::Rules) }
        48 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15); Some(MenuPage::ServerInfo) }
        49 => { handle_manual_compass_claim(player, true); None }
        _ => None,
    }
}

fn handle_rules_click(player: &Player, slot: i16) -> Option<MenuPage> {
    match slot {
        19 => { send_rule_card_to_chat(player, 0); None }
        21 => { send_rule_card_to_chat(player, 1); None }
        23 => { send_rule_card_to_chat(player, 2); None }
        29 => { send_rule_card_to_chat(player, 3); None }
        31 => { send_security_cabinet_guide(player); None }
        33 => { send_rule_card_to_chat(player, 5); None }
        49 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0); Some(MenuPage::Main) }
        _ => None,
    }
}

fn handle_server_info_click(player: &Player, slot: i16) -> Option<MenuPage> {
    match slot {
        13 | 20 | 22 | 24 => { send_server_info_to_chat(player); None }
        49 => { player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0); Some(MenuPage::Main) }
        _ => None,
    }
}

struct ChatMentionHandler;
impl EventHandler<PlayerChatEvent> for ChatMentionHandler {
    fn handle(&self, server: Server, event: PlayerChatEventData) -> PlayerChatEventData {
        if event.cancelled || event.player.is_movement_locked() {
            return event;
        }
        let trimmed = event.message.trim();
        if trimmed.is_empty() || trimmed.starts_with('/') {
            return event;
        }
        let msg_lower = trimmed.to_lowercase();
        let online = server.get_all_players();

        let candidates: Vec<(usize, String)> = online
            .iter()
            .enumerate()
            .filter_map(|(idx, r)| {
                if r.is_movement_locked() { return None; }
                let rk = r.get_name().to_lowercase();
                if !rk.is_empty() && msg_lower.contains(&rk) { Some((idx, rk)) } else { None }
            })
            .collect();

        if candidates.is_empty() {
            return event;
        }

        let enabled = with_state(|st| {
            candidates
                .into_iter()
                .filter_map(|(idx, rk)| st.prefs_for(&rk).chat_mentions.then_some(idx))
                .collect::<Vec<usize>>()
        });
        for idx in enabled {
            online[idx].play_custom_sound("minecraft:block.note_block.chime", SoundCategory::Players, 0.9, 1.4);
        }
        event
    }
}

struct BedrockFormHandler;
impl EventHandler<BedrockFormResponseEvent> for BedrockFormHandler {
    fn handle(&self, server: Server, event: BedrockFormResponseEventData) -> BedrockFormResponseEventData {
        let player = &event.player;
        if player.is_movement_locked() {
            return event;
        }
        let uuid = player.get_id().to_string();
        let matched = with_state(|st| {
            if let Some(&(fid, kind)) = st.bedrock_forms.get(&uuid)
                && fid == event.form_id
            {
                st.bedrock_forms.remove(&uuid);
                return Some(kind);
            }
            None
        });
        let Some(kind) = matched else { return event; };

        match (kind, FormResponse::parse(event.response_data.clone())) {
            (BedrockFormKind::Main, FormResponse::Simple(btn)) => match btn {
                0 => open_page_for_player(player, MenuPage::Navigation),
                1 => open_page_for_player(player, MenuPage::Economy),
                2 => open_page_for_player(player, MenuPage::ChatGuide),
                3 => open_page_for_player(player, MenuPage::RanksInfo),
                4 => open_page_for_player(player, MenuPage::Settings),
                5 => open_page_for_player(player, MenuPage::Rules),
                6 => open_page_for_player(player, MenuPage::ServerInfo),
                7 => handle_manual_compass_claim(player, false),
                _ => {}
            },
            (BedrockFormKind::Navigation, FormResponse::Simple(0)) => handle_safe_spawn_teleport(player),
            (BedrockFormKind::Navigation, FormResponse::Simple(1)) => open_page_for_player(player, MenuPage::Main),
            (BedrockFormKind::Rules, FormResponse::Simple(0)) => send_security_cabinet_guide(player),
            (BedrockFormKind::Rules, FormResponse::Simple(1)) => open_page_for_player(player, MenuPage::Main),
            (BedrockFormKind::Settings, FormResponse::Custom(vals)) if vals.len() >= 6 => {
                let key = player.get_name().to_lowercase();
                let updated = with_state(|st| {
                    st.update_prefs(&key, |p| {
                        p.weather = WeatherPref::from_index(vals[0].as_u64().unwrap_or(0) as u32);
                        p.time = TimePref::from_index(vals[1].as_u64().unwrap_or(0) as u32);
                        p.hud_compass = vals[2].as_bool().unwrap_or(false);
                        p.chat_mentions = vals[3].as_bool().unwrap_or(true);
                        p.hide_players = vals[4].as_bool().unwrap_or(false);
                        p.auto_compass = vals[5].as_bool().unwrap_or(true);
                    })
                });
                save_current_db();
                apply_weather_pref(player, updated.weather);
                apply_time_pref(player, updated.time);
                apply_visibility_pref(&server, player, updated.hide_players);
                if updated.hud_compass { show_hud_compass_actionbar(player); } else { player.show_actionbar(legacy("")); }
                if updated.auto_compass { let _ = give_menu_compass_non_destructive(player); }
                player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.85, 1.35);
                player.send_system_message(legacy("§a✔ Персональные настройки KRONOS сохранены!"), false);
            }
            (BedrockFormKind::Economy | BedrockFormKind::ChatGuide | BedrockFormKind::RanksInfo | BedrockFormKind::ServerInfo, FormResponse::Simple(0)) => {
                open_page_for_player(player, MenuPage::Main);
            }
            _ => {}
        }
        event
    }
}
#[cfg(test)] mod tests;
