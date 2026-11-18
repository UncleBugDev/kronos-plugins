mod store;
mod ui;

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
    BedrockFormKind, MenuPage, PlayerPrefs, TimePref, WeatherPref, init_state, unix_ms_now,
    with_state,
};
use ui::{
    CompassGiveOutcome, apply_time_pref, apply_visibility_pref, apply_weather_pref,
    give_menu_compass_non_destructive, handle_safe_spawn_teleport, is_menu_compass, legacy,
    open_bedrock_menu_page, open_java_menu_page, same_uuid, send_rank_card_to_chat,
    send_rule_card_to_chat, send_security_cabinet_guide, send_server_info_to_chat,
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
            version: "0.1.0".into(),
            authors: vec!["UncleBug".into()],
            description: "KRONOS Server Menu Compass, Player Settings, Rules & Navigation Hub".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        let data_folder = context.get_data_folder();
        init_state(data_folder);

        context.register_permission(&Permission {
            node: PERM_USE.to_string(),
            description: "Access to KRONOS server menu, settings, and rules".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        register_commands(&context);

        context.register_event_handler::<ServerLoadEvent, _>(
            ServerLoadHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<PlayerRespawnEvent, _>(
            RespawnHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<PlayerLeaveEvent, _>(
            LeaveHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<PlayerInteractEvent, _>(
            InteractHandler,
            EventPriority::Normal,
            true,
        )?;
        context.register_event_handler::<InventoryCloseEvent, _>(
            InvCloseHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<InventoryClickEvent, _>(
            InvClickHandler,
            EventPriority::Normal,
            true,
        )?;
        context.register_event_handler::<PlayerChatEvent, _>(
            ChatMentionHandler,
            EventPriority::Low,
            true,
        )?;
        context.register_event_handler::<BedrockFormResponseEvent, _>(
            BedrockFormHandler,
            EventPriority::Normal,
            true,
        )?;

        // Immediate scan for already-online players on hot-reload + repeating 20-tick sync
        context.schedule_delayed_task(1, menu_tick);
        context.schedule_repeating_task(20, 20, menu_tick);

        tracing::info!("KRONOS Menu Plugin v0.1.0 loaded!");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        with_state(|st| {
            st.save_db();
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
    let menu_cmd = Command::new(
        &["menu".into(), "m".into()],
        "Открыть главное меню сервера KRONOS (/menu)",
    )
    .execute(OpenPageCmd(MenuPage::Main));
    context.register_command(menu_cmd, PERM_USE);

    let settings_cmd = Command::new(
        &["settings".into()],
        "Открыть персональные настройки игрока (/settings)",
    )
    .execute(OpenPageCmd(MenuPage::Settings));
    context.register_command(settings_cmd, PERM_USE);

    let rules_cmd = Command::new(
        &["rules".into()],
        "Открыть правила сервера KRONOS (/rules)",
    )
    .execute(OpenPageCmd(MenuPage::Rules));
    context.register_command(rules_cmd, PERM_USE);
}

/// Opens a menu page for an authenticated Java or Bedrock player.
/// Strictly obeys the synchronous `InventoryCloseEvent` race rule:
/// `open_gui` is called OUTSIDE `with_state`, and `open_pages` is recorded AFTER `open_gui` returns.
fn open_page_for_player(player: &Player, page: MenuPage) {
    if player.is_movement_locked() {
        return;
    }

    let uuid = player.get_id().to_string();
    let key = player.get_name().to_lowercase();
    let prefs = with_state(|st| st.prefs_for(&key));

    if player.as_bedrock().is_some() {
        if let Some((fid, kind)) = open_bedrock_menu_page(player, page, &prefs) {
            with_state(|st| {
                st.bedrock_forms.insert(uuid, (fid, kind));
            });
        }
    } else {
        open_java_menu_page(player, page, &prefs);
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
        CompassGiveOutcome::PlacedInSlot(_slot) => {
            player.play_custom_sound(
                "minecraft:entity.experience_orb.pickup",
                SoundCategory::Ui,
                0.85,
                1.25,
            );
            player.send_system_message(
                legacy("§a✔ Компас меню KRONOS добавлен в ваш инвентарь!"),
                false,
            );
            if refresh_main {
                open_page_for_player(player, MenuPage::Main);
            }
        }
        CompassGiveOutcome::AlreadyPresent => {
            player.play_custom_sound(
                "minecraft:block.note_block.pling",
                SoundCategory::Ui,
                0.8,
                1.2,
            );
            player.send_system_message(
                legacy("§e🧭 Компас меню KRONOS уже есть в вашем инвентаре!"),
                false,
            );
        }
        CompassGiveOutcome::InventoryFull => {
            player.play_custom_sound(
                "minecraft:entity.villager.no",
                SoundCategory::Ui,
                0.85,
                1.0,
            );
            player.send_system_message(
                legacy("§c✖ В инвентаре нет свободного слота! Освободите хотя бы 1 слот."),
                false,
            );
        }
    }
}

fn menu_tick(server: Server) {
    let players = server.get_all_players();
    if players.is_empty() {
        return;
    }

    let mut snapshots: Vec<(usize, Uuid, String, String, bool)> = Vec::with_capacity(players.len());
    for (idx, p) in players.iter().enumerate() {
        let uuid_raw = p.get_id();
        let uuid = uuid_raw.to_string();
        let key = p.get_name().to_lowercase();
        let locked = p.is_movement_locked();
        snapshots.push((idx, uuid_raw, uuid, key, locked));
    }

    let (plans, solitude_viewers) = with_state(|st| {
        let mut plans: Vec<(usize, Uuid, bool, PlayerPrefs)> = Vec::new();
        let mut viewers: Vec<Uuid> = Vec::new();

        for (idx, uuid_raw, uuid, key, locked) in snapshots {
            if locked {
                st.open_pages.remove(&uuid);
                st.delivered_this_session.remove(&uuid);
                continue;
            }
            let newly_authenticated = st.delivered_this_session.insert(uuid);
            let prefs = st.prefs_for(&key);
            if prefs.hide_players {
                viewers.push(uuid_raw);
            }
            plans.push((idx, uuid_raw, newly_authenticated, prefs));
        }

        (plans, viewers)
    });

    for (idx, uuid_raw, newly_authenticated, prefs) in plans {
        let player = &players[idx];
        if newly_authenticated {
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
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("Команда доступна только игрокам."));
            return Ok(1);
        };
        if player.is_movement_locked() {
            player.play_custom_sound(
                "minecraft:entity.villager.no",
                SoundCategory::Ui,
                0.85,
                1.0,
            );
            player.show_actionbar(legacy(
                "§c🔒 Сначала авторизуйтесь! Введите пароль в чат или §e/auth",
            ));
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
            let Some(p) = srv.get_player_by_uuid(uuid_raw) else {
                return;
            };
            if p.is_movement_locked() {
                return;
            }
            let key = p.get_name().to_lowercase();
            let prefs = with_state(|st| st.prefs_for(&key));
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
        if !matches!(
            event.action,
            InteractAction::RightClickAir | InteractAction::RightClickBlock
        ) {
            return event;
        }

        // In Pumpkin WIT: Hand::Right = main hand (selected slot), Hand::Left = off-hand
        let in_main = event
            .player
            .get_item_in_hand(Hand::Right)
            .is_some_and(|s| is_menu_compass(&s));
        let in_off = !in_main
            && event
                .player
                .get_item_in_hand(Hand::Left)
                .is_some_and(|s| is_menu_compass(&s));

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
            event.player.play_custom_sound(
                "minecraft:ui.button.click",
                SoundCategory::Ui,
                0.8,
                1.15,
            );
            open_page_for_player(&event.player, MenuPage::Main);
        }

        event
    }
}

struct InvCloseHandler;
impl EventHandler<InventoryCloseEvent> for InvCloseHandler {
    fn handle(&self, _server: Server, event: InventoryCloseEventData) -> InventoryCloseEventData {
        let uuid = event.player.get_id().to_string();
        with_state(|st| {
            st.open_pages.remove(&uuid);
        });
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
        let active_page = with_state(|st| st.open_pages.get(&uuid).copied());
        let Some(page) = active_page else {
            return event;
        };

        event.cancelled = true;

        if event.raw_slot < 0 || event.raw_slot >= 54 || event.raw_slot != event.slot {
            return event;
        }

        match page {
            MenuPage::Main => handle_main_click(player, event.slot),
            MenuPage::Settings => handle_settings_click(&server, player, event.slot),
            MenuPage::Rules => handle_rules_click(player, event.slot),
            MenuPage::RanksInfo => handle_ranks_click(player, event.slot),
            MenuPage::ServerInfo => handle_server_info_click(player, event.slot),
        }

        event
    }
}

fn handle_main_click(player: &Player, slot: i16) {
    match slot {
        4 => {
            player.play_custom_sound(
                "minecraft:entity.experience_orb.pickup",
                SoundCategory::Ui,
                0.75,
                1.35,
            );
            open_page_for_player(player, MenuPage::Main);
        }
        20 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15);
            open_page_for_player(player, MenuPage::Settings);
        }
        22 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15);
            open_page_for_player(player, MenuPage::Rules);
        }
        24 => {
            handle_safe_spawn_teleport(player);
        }
        29 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15);
            open_page_for_player(player, MenuPage::RanksInfo);
        }
        31 => {
            send_security_cabinet_guide(player);
        }
        33 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.15);
            open_page_for_player(player, MenuPage::ServerInfo);
        }
        49 => {
            handle_manual_compass_claim(player, true);
        }
        _ => {}
    }
}

fn handle_settings_click(server: &Server, player: &Player, slot: i16) {
    if slot == 49 {
        player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
        open_page_for_player(player, MenuPage::Main);
        return;
    }

    let key = player.get_name().to_lowercase();

    match slot {
        19 | 28 => {
            let updated = with_state(|st| {
                st.update_prefs(&key, |p| {
                    p.weather = p.weather.cycle();
                })
            });
            apply_weather_pref(player, updated.weather);
            player.play_custom_sound(
                "minecraft:block.note_block.pling",
                SoundCategory::Ui,
                0.8,
                1.3,
            );
            open_page_for_player(player, MenuPage::Settings);
        }
        20 | 29 => {
            let updated = with_state(|st| {
                st.update_prefs(&key, |p| {
                    p.time = p.time.cycle();
                })
            });
            apply_time_pref(player, updated.time);
            player.play_custom_sound(
                "minecraft:block.note_block.pling",
                SoundCategory::Ui,
                0.8,
                1.3,
            );
            open_page_for_player(player, MenuPage::Settings);
        }
        21 | 30 => {
            let updated = with_state(|st| {
                st.update_prefs(&key, |p| {
                    p.hud_compass = !p.hud_compass;
                })
            });
            if updated.hud_compass {
                show_hud_compass_actionbar(player);
            } else {
                player.show_actionbar(legacy(""));
            }
            let pitch = if updated.hud_compass { 1.4 } else { 0.95 };
            player.play_custom_sound(
                "minecraft:block.note_block.pling",
                SoundCategory::Ui,
                0.8,
                pitch,
            );
            open_page_for_player(player, MenuPage::Settings);
        }
        23 | 32 => {
            let updated = with_state(|st| {
                st.update_prefs(&key, |p| {
                    p.chat_mentions = !p.chat_mentions;
                })
            });
            let pitch = if updated.chat_mentions { 1.4 } else { 0.95 };
            player.play_custom_sound(
                "minecraft:block.note_block.pling",
                SoundCategory::Ui,
                0.8,
                pitch,
            );
            open_page_for_player(player, MenuPage::Settings);
        }
        24 | 33 => {
            let updated = with_state(|st| {
                st.update_prefs(&key, |p| {
                    p.hide_players = !p.hide_players;
                })
            });
            apply_visibility_pref(server, player, updated.hide_players);
            let pitch = if updated.hide_players { 1.4 } else { 0.95 };
            player.play_custom_sound(
                "minecraft:block.note_block.pling",
                SoundCategory::Ui,
                0.8,
                pitch,
            );
            open_page_for_player(player, MenuPage::Settings);
        }
        25 | 34 => {
            let updated = with_state(|st| {
                st.update_prefs(&key, |p| {
                    p.auto_compass = !p.auto_compass;
                })
            });
            if updated.auto_compass {
                let _ = give_menu_compass_non_destructive(player);
            }
            let pitch = if updated.auto_compass { 1.4 } else { 0.95 };
            player.play_custom_sound(
                "minecraft:block.note_block.pling",
                SoundCategory::Ui,
                0.8,
                pitch,
            );
            open_page_for_player(player, MenuPage::Settings);
        }
        _ => {}
    }
}

fn handle_rules_click(player: &Player, slot: i16) {
    match slot {
        19 => send_rule_card_to_chat(player, 0),
        21 => send_rule_card_to_chat(player, 1),
        23 => send_rule_card_to_chat(player, 2),
        29 => send_rule_card_to_chat(player, 3),
        31 => send_rule_card_to_chat(player, 4),
        33 => send_rule_card_to_chat(player, 5),
        49 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
            open_page_for_player(player, MenuPage::Main);
        }
        _ => {}
    }
}

fn handle_ranks_click(player: &Player, slot: i16) {
    match slot {
        11 => send_rank_card_to_chat(player, 0),
        13 => send_rank_card_to_chat(player, 1),
        15 => send_rank_card_to_chat(player, 2),
        29 => send_rank_card_to_chat(player, 3),
        30 => send_rank_card_to_chat(player, 4),
        32 => send_rank_card_to_chat(player, 5),
        33 => send_rank_card_to_chat(player, 6),
        49 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
            open_page_for_player(player, MenuPage::Main);
        }
        _ => {}
    }
}

fn handle_server_info_click(player: &Player, slot: i16) {
    match slot {
        13 | 20 | 22 | 24 | 31 => send_server_info_to_chat(player),
        49 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
            open_page_for_player(player, MenuPage::Main);
        }
        _ => {}
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

        let mut candidates: Vec<(usize, String)> = Vec::new();
        for (idx, recipient) in online.iter().enumerate() {
            if recipient.is_movement_locked() {
                continue;
            }
            let r_key = recipient.get_name().to_lowercase();
            if !r_key.is_empty() && msg_lower.contains(&r_key) {
                candidates.push((idx, r_key));
            }
        }

        if candidates.is_empty() {
            return event;
        }

        let enabled_indices = with_state(|st| {
            candidates
                .into_iter()
                .filter_map(|(idx, r_key)| {
                    if st.prefs_for(&r_key).chat_mentions {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect::<Vec<usize>>()
        });

        for idx in enabled_indices {
            online[idx].play_custom_sound(
                "minecraft:block.note_block.chime",
                SoundCategory::Players,
                0.9,
                1.4,
            );
        }

        event
    }
}

struct BedrockFormHandler;
impl EventHandler<BedrockFormResponseEvent> for BedrockFormHandler {
    fn handle(
        &self,
        server: Server,
        event: BedrockFormResponseEventData,
    ) -> BedrockFormResponseEventData {
        let player = &event.player;
        if player.is_movement_locked() {
            return event;
        }

        let uuid = player.get_id().to_string();
        let matched_kind = with_state(|st| {
            if let Some(&(fid, kind)) = st.bedrock_forms.get(&uuid)
                && fid == event.form_id
            {
                st.bedrock_forms.remove(&uuid);
                return Some(kind);
            }
            None
        });

        let Some(kind) = matched_kind else {
            return event;
        };

        let parsed = FormResponse::parse(event.response_data.clone());
        match (kind, parsed) {
            (BedrockFormKind::Main, FormResponse::Simple(btn)) => match btn {
                0 => open_page_for_player(player, MenuPage::Settings),
                1 => open_page_for_player(player, MenuPage::Rules),
                2 => handle_safe_spawn_teleport(player),
                3 => open_page_for_player(player, MenuPage::RanksInfo),
                4 => send_security_cabinet_guide(player),
                5 => open_page_for_player(player, MenuPage::ServerInfo),
                6 => handle_manual_compass_claim(player, false),
                _ => {}
            },
            (BedrockFormKind::Settings, FormResponse::Custom(vals)) => {
                if vals.len() >= 6 {
                    let weather_idx = vals[0].as_u64().unwrap_or(0) as u32;
                    let time_idx = vals[1].as_u64().unwrap_or(0) as u32;
                    let hud = vals[2].as_bool().unwrap_or(false);
                    let mentions = vals[3].as_bool().unwrap_or(true);
                    let hide = vals[4].as_bool().unwrap_or(false);
                    let auto_c = vals[5].as_bool().unwrap_or(true);

                    let key = player.get_name().to_lowercase();
                    let updated = with_state(|st| {
                        st.update_prefs(&key, |p| {
                            p.weather = WeatherPref::from_index(weather_idx);
                            p.time = TimePref::from_index(time_idx);
                            p.hud_compass = hud;
                            p.chat_mentions = mentions;
                            p.hide_players = hide;
                            p.auto_compass = auto_c;
                        })
                    });

                    apply_weather_pref(player, updated.weather);
                    apply_time_pref(player, updated.time);
                    apply_visibility_pref(&server, player, updated.hide_players);
                    if updated.hud_compass {
                        show_hud_compass_actionbar(player);
                    } else {
                        player.show_actionbar(legacy(""));
                    }
                    if updated.auto_compass {
                        let _ = give_menu_compass_non_destructive(player);
                    }

                    player.play_custom_sound(
                        "minecraft:block.note_block.pling",
                        SoundCategory::Ui,
                        0.85,
                        1.35,
                    );
                    player.send_system_message(
                        legacy("§a✔ Персональные настройки KRONOS сохранены!"),
                        false,
                    );
                }
            }
            (
                BedrockFormKind::Rules | BedrockFormKind::RanksInfo | BedrockFormKind::ServerInfo,
                FormResponse::Simple(0),
            ) => {
                open_page_for_player(player, MenuPage::Main);
            }
            _ => {}
        }

        event
    }
}
