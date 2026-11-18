mod sha256;
mod store;
mod ui;

use pumpkin_plugin_api::{
    Context, Plugin, PluginMetadata, Player, Server,
    command::{ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType},
    commands::CommandHandler,
    events::{
        BedrockFormResponseEvent, BlockBreakEvent, BlockPlaceEvent, EventHandler, EventPriority,
        InventoryClickEvent, InventoryCloseEvent, PlayerChatEvent, PlayerCommandPreprocessEvent,
        PlayerDropItemEvent, PlayerInteractEvent, PlayerJoinEvent, PlayerLeaveEvent,
        PlayerMoveEvent,
    },
    permission::{Permission, PermissionDefault},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    scheduler::SchedulerExt,
    wit::pumpkin::plugin::{
        command::Arg,
        event::{
            BedrockFormResponseEventData, BlockBreakEventData, BlockPlaceEventData,
            InventoryClickEventData, InventoryCloseEventData, PlayerChatEventData,
            PlayerCommandPreprocessEventData, PlayerDropItemEventData, PlayerInteractEventData,
            PlayerJoinEventData, PlayerLeaveEventData, PlayerMoveEventData,
        },
        sounds::SoundCategory,
    },
};

use store::{AUTH_TIMEOUT_SECS, PlayerSession, init_state, normalize_ip, unix_secs_now, with_state};
use ui::{
    apply_tab_branding, attach_boss_bar, create_auth_boss_bar, detach_boss_bar, give_starter_kit,
    kick_player, legacy, open_account_cabinet_gui, open_auth_gui, open_bedrock_auth_form,
    trigger_welcome_effects,
};

const PERM_PLAYER: &str = "auth-plugin:player";

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "auth-plugin".into(),
            version: "0.2.0".into(),
            authors: vec!["UncleBug".into()],
            description: "Interactive Auth, PIN GUI, Welcome FX & Account Security".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        let data_folder = context.get_data_folder();
        init_state(data_folder);

        context.register_permission(&Permission {
            node: PERM_PLAYER.to_string(),
            description: "Default player access to auth and survival commands".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        register_commands(&context);

        context.register_event_handler::<PlayerJoinEvent, _>(JoinHandler, EventPriority::Highest, true)?;
        context.register_event_handler::<PlayerLeaveEvent, _>(LeaveHandler, EventPriority::Highest, true)?;
        context.register_event_handler::<PlayerMoveEvent, _>(MoveHandler, EventPriority::Highest, true)?;
        context.register_event_handler::<PlayerChatEvent, _>(ChatHandler, EventPriority::Highest, true)?;
        context.register_event_handler::<PlayerCommandPreprocessEvent, _>(
            CmdPreprocessHandler,
            EventPriority::Highest,
            true,
        )?;
        context.register_event_handler::<BlockBreakEvent, _>(BreakHandler, EventPriority::Highest, true)?;
        context.register_event_handler::<BlockPlaceEvent, _>(PlaceHandler, EventPriority::Highest, true)?;
        context.register_event_handler::<PlayerInteractEvent, _>(
            InteractHandler,
            EventPriority::Highest,
            true,
        )?;
        context.register_event_handler::<PlayerDropItemEvent, _>(
            DropHandler,
            EventPriority::Highest,
            true,
        )?;
        context.register_event_handler::<InventoryClickEvent, _>(
            InvClickHandler,
            EventPriority::Highest,
            true,
        )?;
        context.register_event_handler::<InventoryCloseEvent, _>(
            InvCloseHandler,
            EventPriority::Highest,
            true,
        )?;
        context.register_event_handler::<BedrockFormResponseEvent, _>(
            BedrockFormHandler,
            EventPriority::Highest,
            true,
        )?;

        context.schedule_repeating_task(20, 20, auth_countdown_tick);

        tracing::info!("Auth & Welcome Plugin v0.2.0 initialized!");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        let old_sessions = with_state(|st| {
            st.save_db();
            std::mem::take(&mut st.sessions)
        });
        drop(old_sessions);
        tracing::info!("Auth Plugin unloaded cleanly.");
        Ok(())
    }
}

register_plugin!(AuthPlugin);

fn register_commands(context: &Context) {
    let reg_cmd = Command::new(
        &["register".into(), "reg".into()],
        "Регистрация аккаунта (/register <пароль> [повтор])",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(RegisterCmd),
    )
    .execute(OpenAuthGuiCmd);
    context.register_command(reg_cmd, PERM_PLAYER);

    let login_cmd = Command::new(
        &["login".into(), "l".into()],
        "Вход в аккаунт (/login <пароль>)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(LoginCmd),
    )
    .execute(OpenAuthGuiCmd);
    context.register_command(login_cmd, PERM_PLAYER);

    let changepass_cmd = Command::new(
        &["changepass".into(), "cp".into()],
        "Смена пароля (/changepass <старый> <новый>)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(ChangePassCmd),
    );
    context.register_command(changepass_cmd, PERM_PLAYER);

    let auth_cmd = Command::new(
        &["auth".into(), "account".into(), "pin".into()],
        "Открыть окно входа / Личный кабинет",
    )
    .execute(OpenAuthGuiCmd);
    context.register_command(auth_cmd, PERM_PLAYER);

    let logout_cmd = Command::new(&["logout".into()], "Выйти из текущей сессии").execute(LogoutCmd);
    context.register_command(logout_cmd, PERM_PLAYER);

    let kit_cmd = Command::new(&["kit".into()], "Получить стартовый набор выжившего").execute(KitCmd);
    context.register_command(kit_cmd, PERM_PLAYER);

    let spawn_cmd = Command::new(&["spawn".into()], "Телепортироваться на точку спавна").execute(SpawnCmd);
    context.register_command(spawn_cmd, PERM_PLAYER);
}

fn is_authenticated_player(player: &Player) -> bool {
    let uuid = player.get_id().to_string();
    with_state(|st| st.sessions.get(&uuid).is_some_and(|s| s.authenticated))
}

fn lock_player_sandbox(player: &Player, seconds_left: u32) {
    player.set_movement_locked(true);
    player.set_invulnerable(true);
    player.set_freeze_ticks(140);
    let name = player.get_name();
    player.set_display_name(legacy(&format!("§c[🔒] §7{name}")));
    player.show_actionbar(legacy(&format!(
        "§e🔐 Введите пароль в чат или в окне §b/auth §8(§c{seconds_left}с§8)"
    )));
}

fn unlock_player_sandbox(player: &Player) {
    player.set_movement_locked(false);
    player.set_invulnerable(false);
    player.set_freeze_ticks(0);
    let name = player.get_name();
    player.set_display_name(legacy(&format!("§a[✔ Игрок] §f{name}")));
}

fn finish_auth(server: &Server, player: &Player, is_new_reg: bool, auto_ip: bool) {
    let uuid_raw = player.get_id();
    let uuid = uuid_raw.to_string();
    let old_bar = with_state(|st| {
        if let Some(s) = st.sessions.get_mut(&uuid) {
            s.authenticated = true;
            s.pin_buffer.clear();
            s.wrong_attempts = 0;
            s.boss_bar.take()
        } else {
            None
        }
    });

    if let Some(bar) = old_bar {
        detach_boss_bar(server, uuid_raw, &bar);
    }

    unlock_player_sandbox(player);

    if is_new_reg {
        let key = player.get_name().to_lowercase();
        let should_give = with_state(|st| {
            if let Some(acc) = st.db.accounts.get_mut(&key) {
                if !acc.kit_claimed {
                    acc.kit_claimed = true;
                    st.save_db();
                    return true;
                }
            }
            false
        });
        if should_give {
            give_starter_kit(player);
        }
    }

    if auto_ip {
        player.play_custom_sound(
            "minecraft:entity.experience_orb.pickup",
            SoundCategory::Players,
            0.8,
            1.3,
        );
        player.show_title(legacy("§b§l✦ АВТО-ВХОД ✦"));
        player.show_subtitle(legacy("§7Сессия подтверждена по вашему IP (24ч)"));
        player.show_actionbar(legacy("§a✔ Автоматическая авторизация по IP"));
        ui::send_clickable_dashboard(player);
    } else {
        trigger_welcome_effects(player, is_new_reg);
    }
}

fn show_cabinet_gui(player: &Player) {
    let uuid = player.get_id().to_string();
    let key = player.get_name().to_lowercase();
    let acc = with_state(|st| st.db.accounts.get(&key).cloned());
    // Must call open_gui outside with_state and mark cabinet_open AFTER open_gui:
    // pumpkin-core's open_handled_screen_direct closes any active screen first and
    // synchronously fires InventoryCloseEvent (which resets cabinet_open = false).
    open_account_cabinet_gui(player, acc.as_ref());
    with_state(|st| {
        if let Some(s) = st.sessions.get_mut(&uuid) {
            if s.authenticated {
                s.cabinet_open = true;
            }
        }
    });
}

fn process_password_submission(
    server: &Server,
    player: &Player,
    password: &str,
    is_register_cmd: Option<bool>,
) -> bool {
    let uuid = player.get_id().to_string();
    let name = player.get_name();
    let key = name.to_lowercase();
    let ip = normalize_ip(&player.get_ip());

    if password.len() < 4 || password.len() > 32 {
        player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Players, 0.9, 1.0);
        player.send_system_message(
            legacy("§c✖ Пароль должен содержать от 4 до 32 символов!"),
            false,
        );
        return false;
    }

    let registered = with_state(|st| st.is_registered(&key));

    match is_register_cmd {
        Some(true) if registered => {
            player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Players, 0.9, 1.0);
            player.send_system_message(
                legacy("§c✖ Этот ник уже зарегистрирован! Введите: §e/login <пароль> §cили прямо в чат."),
                false,
            );
            return false;
        }
        Some(false) if !registered => {
            player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Players, 0.9, 1.0);
            player.send_system_message(
                legacy("§c✖ Вы ещё не зарегистрированы! Введите: §a/register <пароль> §cили прямо в чат."),
                false,
            );
            return false;
        }
        _ => {}
    }

    if !registered {
        with_state(|st| {
            st.register_account(&name, &ip, password);
        });
        finish_auth(server, player, true, false);
        true
    } else {
        let verified = with_state(|st| st.verify_and_touch_login(&key, &ip, password).is_some());
        if verified {
            finish_auth(server, player, false, false);
            true
        } else {
            let attempts = with_state(|st| {
                if let Some(s) = st.sessions.get_mut(&uuid) {
                    s.wrong_attempts = s.wrong_attempts.saturating_add(1);
                    s.pin_buffer.clear();
                    s.wrong_attempts
                } else {
                    1
                }
            });
            player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Players, 0.9, 0.8);
            player.send_hurt_animation(0.0);
            player.send_system_message(
                legacy(&format!("§c✖ Неверный пароль! Попытка §f{attempts}/5")),
                false,
            );
            if attempts >= 5 {
                kick_player(
                    player,
                    "§c§l✦ KRONOS SECURITY ✦\n\n§fПревышено число неверных попыток входа (5/5).",
                );
            }
            false
        }
    }
}

fn auth_countdown_tick(server: Server) {
    let players = server.get_all_players();
    let online_count = players.len() as u32;

    for player in players {
        let uuid = player.get_id().to_string();
        let (authenticated, rem_after, bar_opt) = with_state(|st| {
            let Some(sess) = st.sessions.get_mut(&uuid) else {
                return (true, 0, None);
            };
            if sess.authenticated {
                return (true, 0, None);
            }
            sess.remaining_secs = sess.remaining_secs.saturating_sub(1);
            (false, sess.remaining_secs, sess.boss_bar.take())
        });

        apply_tab_branding(&player, online_count);

        if authenticated {
            continue;
        }

        if let Some(bar) = bar_opt {
            bar.set_title(legacy(&format!(
                "§6§l✦ KRONOS SECURITY §8• §eОсталось §c§l{rem_after} §eсек для входа"
            )));
            bar.set_health((rem_after as f32 / AUTH_TIMEOUT_SECS as f32).clamp(0.0, 1.0));
            let leftover_bar = with_state(|st| {
                if let Some(sess) = st.sessions.get_mut(&uuid) {
                    if !sess.authenticated {
                        return sess.boss_bar.replace(bar);
                    }
                }
                Some(bar)
            });
            drop(leftover_bar);
        }

        if rem_after == 0 {
            kick_player(
                &player,
                "§6§l✦ KRONOS SECURITY ✦\n\n§eВремя на авторизацию истекло (60 сек).\n§7Перезайдите и введите пароль в окне или в чат.",
            );
        } else {
            player.set_freeze_ticks(140);
            player.show_actionbar(legacy(&format!(
                "§e🔐 Введите пароль в чат или в окне §b/auth §8(§c{rem_after}с§8)"
            )));
        }
    }
}

struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let player = &event.player;
        let uuid_raw = player.get_id();
        let uuid = uuid_raw.to_string();
        let name = player.get_name();
        let key = name.to_lowercase();
        let ip = normalize_ip(&player.get_ip());
        let now = unix_secs_now();

        let (registered, valid_session) = with_state(|st| {
            let reg = st.is_registered(&key);
            let valid = st.has_valid_ip_session(&key, &ip, now);
            (reg, valid)
        });

        let online_count = server.get_player_count().max(1);
        apply_tab_branding(player, online_count);

        if valid_session {
            let replaced_session = with_state(|st| {
                if let Some(acc) = st.db.accounts.get_mut(&key) {
                    acc.last_ip = ip;
                    acc.last_login_ts = now;
                    acc.login_count = acc.login_count.saturating_add(1);
                    st.save_db();
                }
                st.sessions.insert(
                    uuid,
                    PlayerSession {
                        key,
                        authenticated: true,
                        pin_buffer: String::new(),
                        cabinet_open: false,
                        remaining_secs: AUTH_TIMEOUT_SECS,
                        wrong_attempts: 0,
                        boss_bar: None,
                        bedrock_form_id: None,
                    },
                )
            });
            if let Some(mut old_sess) = replaced_session {
                if let Some(ob) = old_sess.boss_bar.take() {
                    detach_boss_bar(&server, uuid_raw, &ob);
                }
            }
            unlock_player_sandbox(player);
            server.schedule_delayed_task(15, move |srv| {
                if let Some(p) = srv.get_player_by_uuid(uuid_raw) {
                    finish_auth(&srv, &p, false, true);
                }
            });
            return event;
        }

        let bar = create_auth_boss_bar(AUTH_TIMEOUT_SECS);
        attach_boss_bar(&server, uuid_raw, &bar);
        let replaced_session = with_state(|st| {
            st.sessions.insert(
                uuid.clone(),
                PlayerSession {
                    key: key.clone(),
                    authenticated: false,
                    pin_buffer: String::new(),
                    cabinet_open: false,
                    remaining_secs: AUTH_TIMEOUT_SECS,
                    wrong_attempts: 0,
                    boss_bar: Some(bar),
                    bedrock_form_id: None,
                },
            )
        });
        if let Some(mut old_sess) = replaced_session {
            if let Some(ob) = old_sess.boss_bar.take() {
                detach_boss_bar(&server, uuid_raw, &ob);
            }
        }

        lock_player_sandbox(player, AUTH_TIMEOUT_SECS);

        player.send_system_message(legacy("§8§m────────────────────────────────────────"), false);
        if registered {
            player.send_system_message(
                legacy("§6§l✦ KRONOS §8» §fС возвращением! Подтвердите аккаунт:"),
                false,
            );
            player.send_system_message(
                legacy("  §7• Введите пароль §aпрямо в чат §7(другие не увидят)"),
                false,
            );
            player.send_system_message(
                legacy("  §7• Или введите в графическом PIN-окне!"),
                false,
            );
        } else {
            player.send_system_message(
                legacy("§6§l✦ KRONOS §8» §fДобро пожаловать! Защитите новый аккаунт:"),
                false,
            );
            player.send_system_message(
                legacy("  §7• Введите пароль §aпрямо в чат §7или §b/register <пароль>"),
                false,
            );
            player.send_system_message(
                legacy("  §7• Либо задайте PIN в окне, которое откроется сейчас!"),
                false,
            );
        }

        let open_btn = legacy("§8» §b§l[🔐 Нажмите, чтобы открыть окно входа]")
            .click_run_command("/auth")
            .hover_show_text(legacy("§7Открыть PIN-клавиатуру и меню приветствия"));
        player.send_system_message(open_btn, false);
        player.send_system_message(legacy("§8§m────────────────────────────────────────"), false);

        // Open GUI 15 ticks after join so client finishes initial terrain load cleanly
        let uuid_str = uuid;
        server.schedule_delayed_task(15, move |srv| {
            let Some(p) = srv.get_player_by_uuid(uuid_raw) else {
                return;
            };
            let (auth, pin, reg) = with_state(|st| {
                if let Some(s) = st.sessions.get(&uuid_str) {
                    (s.authenticated, s.pin_buffer.clone(), st.is_registered(&s.key))
                } else {
                    (true, String::new(), false)
                }
            });
            if auth {
                return;
            }
            if p.as_bedrock().is_some() {
                let fid = open_bedrock_auth_form(&p, reg);
                with_state(|st| {
                    if let Some(s) = st.sessions.get_mut(&uuid_str) {
                        s.bedrock_form_id = fid;
                    }
                });
            } else {
                open_auth_gui(&p, &pin, reg);
            }
        });

        event
    }
}

struct LeaveHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(&self, server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let uuid_raw = event.player.get_id();
        let uuid = uuid_raw.to_string();
        let bar = with_state(|st| {
            st.sessions.remove(&uuid).and_then(|mut s| s.boss_bar.take())
        });
        if let Some(b) = bar {
            detach_boss_bar(&server, uuid_raw, &b);
        }
        event
    }
}

struct MoveHandler;
impl EventHandler<PlayerMoveEvent> for MoveHandler {
    fn handle(&self, _server: Server, mut event: PlayerMoveEventData) -> PlayerMoveEventData {
        if !is_authenticated_player(&event.player) {
            let dx = (event.from_position.0 - event.to_position.0).abs();
            let dz = (event.from_position.2 - event.to_position.2).abs();
            if dx > 0.01 || dz > 0.01 {
                event.cancelled = true;
            }
        }
        event
    }
}

struct ChatHandler;
impl EventHandler<PlayerChatEvent> for ChatHandler {
    fn handle(&self, server: Server, mut event: PlayerChatEventData) -> PlayerChatEventData {
        if !is_authenticated_player(&event.player) {
            event.cancelled = true;
            let trimmed = event.message.trim().to_string();
            if !trimmed.is_empty() && !trimmed.starts_with('/') {
                process_password_submission(&server, &event.player, &trimmed, None);
            }
        }
        event
    }
}

struct CmdPreprocessHandler;
impl EventHandler<PlayerCommandPreprocessEvent> for CmdPreprocessHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: PlayerCommandPreprocessEventData,
    ) -> PlayerCommandPreprocessEventData {
        if !is_authenticated_player(&event.player) {
            let cmd_clean = event
                .command
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();
            let allowed = matches!(
                cmd_clean.as_str(),
                "register" | "reg" | "login" | "l" | "auth" | "account" | "pin"
            );
            if !allowed {
                event.cancelled = true;
                event
                    .player
                    .play_custom_sound("minecraft:entity.villager.no", SoundCategory::Players, 0.8, 1.0);
                event.player.show_actionbar(legacy(
                    "§c🔒 Сначала авторизуйтесь! Пароль в чат или §e/auth",
                ));
            }
        }
        event
    }
}

struct BreakHandler;
impl EventHandler<BlockBreakEvent> for BreakHandler {
    fn handle(&self, _server: Server, mut event: BlockBreakEventData) -> BlockBreakEventData {
        if let Some(ref p) = event.player {
            if !is_authenticated_player(p) {
                event.cancelled = true;
            }
        }
        event
    }
}

struct PlaceHandler;
impl EventHandler<BlockPlaceEvent> for PlaceHandler {
    fn handle(&self, _server: Server, mut event: BlockPlaceEventData) -> BlockPlaceEventData {
        if !is_authenticated_player(&event.player) {
            event.cancelled = true;
        }
        event
    }
}

struct InteractHandler;
impl EventHandler<PlayerInteractEvent> for InteractHandler {
    fn handle(&self, _server: Server, mut event: PlayerInteractEventData) -> PlayerInteractEventData {
        if !is_authenticated_player(&event.player) {
            event.cancelled = true;
        }
        event
    }
}

struct DropHandler;
impl EventHandler<PlayerDropItemEvent> for DropHandler {
    fn handle(&self, _server: Server, mut event: PlayerDropItemEventData) -> PlayerDropItemEventData {
        if !is_authenticated_player(&event.player) {
            event.cancelled = true;
        }
        event
    }
}

struct InvCloseHandler;
impl EventHandler<InventoryCloseEvent> for InvCloseHandler {
    fn handle(&self, _server: Server, event: InventoryCloseEventData) -> InventoryCloseEventData {
        let uuid = event.player.get_id().to_string();
        with_state(|st| {
            if let Some(s) = st.sessions.get_mut(&uuid) {
                s.cabinet_open = false;
            }
        });
        event
    }
}

// Note on container closing: WIT v0.1 does not expose Player::close_handled_screen().
// Sending raw CCloseContainer via JavaPlayer::send_packet bypasses Player::on_handled_screen_closed()
// in pumpkin-core, leaving current_screen_handler stuck on sync_id = 1..=100 (since Notchian clients
// do not reply with SCloseContainer on server-initiated close), which causes Player::on_slot_click
// to reject subsequent player inventory (sync_id = 0) clicks. Instead, we transition open GUIs cleanly
// via open_gui (which calls open_handled_screen_direct -> close_handled_screen in pumpkin-core) and
// let players dismiss screens via ESC so server/client sync_id never desyncs.
struct InvClickHandler;
impl EventHandler<InventoryClickEvent> for InvClickHandler {
    fn handle(&self, server: Server, mut event: InventoryClickEventData) -> InventoryClickEventData {
        let player = &event.player;
        let uuid = player.get_id().to_string();
        let (authenticated, cabinet_open) = with_state(|st| {
            if let Some(s) = st.sessions.get(&uuid) {
                (s.authenticated, s.cabinet_open)
            } else {
                (true, false)
            }
        });

        if authenticated {
            if cabinet_open {
                event.cancelled = true;
                match event.slot {
                    20 => {
                        if handle_kit_claim(player) {
                            show_cabinet_gui(player);
                        }
                    }
                    22 => handle_spawn_teleport(player),
                    24 => perform_logout(&server, player),
                    _ => {}
                }
            }
            return event;
        }

        event.cancelled = true;

        let digit = match event.slot {
            21 => Some('1'),
            22 => Some('2'),
            23 => Some('3'),
            30 => Some('4'),
            31 => Some('5'),
            32 => Some('6'),
            39 => Some('7'),
            40 => Some('8'),
            41 => Some('9'),
            49 => Some('0'),
            _ => None,
        };

        let player_key = player.get_name().to_lowercase();

        if let Some(d) = digit {
            let (pin, reg) = with_state(|st| {
                let mut pin = String::new();
                if let Some(s) = st.sessions.get_mut(&uuid) {
                    if s.pin_buffer.len() < 12 {
                        s.pin_buffer.push(d);
                    }
                    pin = s.pin_buffer.clone();
                }
                (pin, st.is_registered(&player_key))
            });
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.7, 1.5);
            open_auth_gui(player, &pin, reg);
            return event;
        }

        match event.slot {
            48 => {
                let reg = with_state(|st| {
                    if let Some(s) = st.sessions.get_mut(&uuid) {
                        s.pin_buffer.clear();
                    }
                    st.is_registered(&player_key)
                });
                player.play_custom_sound("minecraft:ui.toast.out", SoundCategory::Ui, 0.7, 1.0);
                open_auth_gui(player, "", reg);
            }
            50 => {
                let pin = with_state(|st| {
                    st.sessions
                        .get(&uuid)
                        .map(|s| s.pin_buffer.clone())
                        .unwrap_or_default()
                });
                if pin.len() < 4 {
                    player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Ui, 0.8, 1.0);
                    player.send_system_message(
                        legacy("§c✖ PIN-код должен содержать минимум 4 цифры!"),
                        false,
                    );
                } else if process_password_submission(&server, player, &pin, None) {
                    show_cabinet_gui(player);
                }
            }
            28 => {
                player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.7, 1.2);
                player.send_system_message(
                    legacy("§e💬 Закройте окно (ESC) и просто напишите пароль в чат — другие игроки его не увидят!"),
                    false,
                );
            }
            34 => {
                player.play_custom_sound("minecraft:block.bell.use", SoundCategory::Ui, 0.7, 1.2);
                player.send_system_message(
                    legacy("§d§l📖 ПАМЯТКА: §7Выживание 1.20.1+, авто-вход на 24ч, стартовый /kit, быстрый /spawn, защита и PIN-вход!"),
                    false,
                );
            }
            _ => {}
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
        if is_authenticated_player(&event.player) {
            return event;
        }
        let uuid = event.player.get_id().to_string();
        let expected_id = with_state(|st| st.sessions.get(&uuid).and_then(|s| s.bedrock_form_id));
        if expected_id != Some(event.form_id) {
            return event;
        }
        if let Some(json) = event.response_data.as_deref() {
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
                if let Some(pass) = arr.iter().rev().find_map(|v| v.as_str()) {
                    let trimmed = pass.trim();
                    if !trimmed.is_empty() {
                        process_password_submission(&server, &event.player, trimmed, None);
                    }
                }
            }
        }
        event
    }
}

fn extract_args(args: &ConsumedArgs, key: &str) -> String {
    match args.get_value(key) {
        Arg::Simple(s) | Arg::Msg(s) => s,
        _ => String::new(),
    }
}

struct RegisterCmd;
impl CommandHandler for RegisterCmd {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("Команда доступна только игрокам."));
            return Ok(1);
        };
        let raw = extract_args(&args, "args");
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.is_empty() {
            let key = player.get_name().to_lowercase();
            let reg = with_state(|st| st.is_registered(&key));
            open_auth_gui(&player, "", reg);
            return Ok(0);
        }
        if parts.len() >= 2 && parts[0] != parts[1] {
            player.send_system_message(legacy("§c✖ Пароли не совпадают!"), false);
            return Ok(1);
        }
        process_password_submission(&server, &player, parts[0], Some(true));
        Ok(0)
    }
}

struct LoginCmd;
impl CommandHandler for LoginCmd {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("Команда доступна только игрокам."));
            return Ok(1);
        };
        let raw = extract_args(&args, "args");
        let pass = raw.split_whitespace().next().unwrap_or("");
        if pass.is_empty() {
            let key = player.get_name().to_lowercase();
            let reg = with_state(|st| st.is_registered(&key));
            open_auth_gui(&player, "", reg);
            return Ok(0);
        }
        process_password_submission(&server, &player, pass, Some(false));
        Ok(0)
    }
}

struct OpenAuthGuiCmd;
impl CommandHandler for OpenAuthGuiCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            return Ok(1);
        };
        let uuid = player.get_id().to_string();
        let key = player.get_name().to_lowercase();
        let (auth, pin, reg) = with_state(|st| {
            let reg = st.is_registered(&key);
            if let Some(s) = st.sessions.get(&uuid) {
                (s.authenticated, s.pin_buffer.clone(), reg)
            } else {
                (false, String::new(), reg)
            }
        });

        if auth {
            show_cabinet_gui(&player);
        } else if player.as_bedrock().is_some() {
            let fid = open_bedrock_auth_form(&player, reg);
            with_state(|st| {
                if let Some(s) = st.sessions.get_mut(&uuid) {
                    s.bedrock_form_id = fid;
                }
            });
        } else {
            open_auth_gui(&player, &pin, reg);
        }
        Ok(0)
    }
}

struct ChangePassCmd;
impl CommandHandler for ChangePassCmd {
    fn handle(&self, sender: CommandSender, _server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            return Ok(1);
        };
        if !is_authenticated_player(&player) {
            player.send_system_message(legacy("§c✖ Сначала войдите в аккаунт!"), false);
            return Ok(1);
        }
        let raw = extract_args(&args, "args");
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.len() < 2 {
            player.send_system_message(legacy("§eИспользование: §b/changepass <старый> <новый>"), false);
            return Ok(1);
        }
        if parts[1].len() < 4 || parts[1].len() > 32 {
            player.send_system_message(legacy("§c✖ Новый пароль должен быть от 4 до 32 символов."), false);
            return Ok(1);
        }
        let key = player.get_name().to_lowercase();
        let res = with_state(|st| st.change_password(&key, parts[0], parts[1]));
        match res {
            Ok(()) => {
                player.play_custom_sound("minecraft:entity.player.levelup", SoundCategory::Players, 0.8, 1.3);
                player.send_system_message(legacy("§a✔ Пароль успешно изменён!"), false);
            }
            Err(err) => {
                player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Players, 0.8, 1.0);
                player.send_system_message(legacy(&format!("§c✖ {err}")), false);
            }
        }
        Ok(0)
    }
}

fn perform_logout(server: &Server, player: &Player) {
    let uuid_raw = player.get_id();
    let uuid = uuid_raw.to_string();
    let key = player.get_name().to_lowercase();
    let bar = create_auth_boss_bar(AUTH_TIMEOUT_SECS);
    attach_boss_bar(server, uuid_raw, &bar);

    let (old_bar, unused_new_bar) = with_state(|st| {
        if let Some(acc) = st.db.accounts.get_mut(&key) {
            acc.last_ip.clear();
            acc.last_login_ts = 0;
            st.save_db();
        }
        if let Some(s) = st.sessions.get_mut(&uuid) {
            s.authenticated = false;
            s.cabinet_open = false;
            s.pin_buffer.clear();
            s.remaining_secs = AUTH_TIMEOUT_SECS;
            let old = s.boss_bar.replace(bar);
            (old, None)
        } else {
            (None, Some(bar))
        }
    });

    if let Some(ob) = old_bar {
        detach_boss_bar(server, uuid_raw, &ob);
    }
    if let Some(ub) = unused_new_bar {
        detach_boss_bar(server, uuid_raw, &ub);
    }

    lock_player_sandbox(player, AUTH_TIMEOUT_SECS);
    player.send_system_message(
        legacy("§e🔐 Вы вышли из сессии. Введите пароль в чат или откройте §b/auth§e."),
        false,
    );
    open_auth_gui(player, "", true);
}

struct LogoutCmd;
impl CommandHandler for LogoutCmd {
    fn handle(&self, sender: CommandSender, server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            return Ok(1);
        };
        perform_logout(&server, &player);
        Ok(0)
    }
}

fn handle_kit_claim(player: &Player) -> bool {
    let key = player.get_name().to_lowercase();
    let can_claim = with_state(|st| {
        if let Some(acc) = st.db.accounts.get_mut(&key) {
            if acc.kit_claimed {
                return false;
            }
            acc.kit_claimed = true;
            st.save_db();
            return true;
        }
        false
    });

    if !can_claim {
        player.play_custom_sound("minecraft:entity.villager.no", SoundCategory::Players, 0.8, 1.0);
        player.send_system_message(legacy("§e🎁 Стартовый набор уже был получен ранее!"), false);
        return false;
    }

    let count = give_starter_kit(player);
    player.play_custom_sound("minecraft:entity.player.levelup", SoundCategory::Players, 0.9, 1.2);
    player.send_system_message(
        legacy(&format!("§a🎁 Выдан стартовый набор выжившего ({count} предм.)!")),
        false,
    );
    true
}

struct KitCmd;
impl CommandHandler for KitCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            return Ok(1);
        };
        if !is_authenticated_player(&player) {
            player.send_system_message(legacy("§c✖ Сначала авторизуйтесь!"), false);
            return Ok(1);
        }
        handle_kit_claim(&player);
        Ok(0)
    }
}

fn handle_spawn_teleport(player: &Player) {
    let world = player.get_world();
    let spawn = world.get_spawn_location();
    let target = (
        f64::from(spawn.pos.x) + 0.5,
        f64::from(spawn.pos.y) + 0.5,
        f64::from(spawn.pos.z) + 0.5,
    );
    player.teleport(target, Some(spawn.yaw), Some(spawn.pitch), world);
    player.play_custom_sound("minecraft:entity.enderman.teleport", SoundCategory::Players, 0.8, 1.1);
    player.show_actionbar(legacy("§b🧭 Телепортация на спавн выполнена!"));
}

struct SpawnCmd;
impl CommandHandler for SpawnCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            return Ok(1);
        };
        if !is_authenticated_player(&player) {
            player.send_system_message(legacy("§c✖ Сначала авторизуйтесь!"), false);
            return Ok(1);
        }
        handle_spawn_teleport(&player);
        Ok(0)
    }
}
