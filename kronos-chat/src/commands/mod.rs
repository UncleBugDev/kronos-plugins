pub mod ignore;
pub mod msg;
pub mod reply;
pub mod spy;
pub mod trade;

use pumpkin_plugin_api::{
    command::{ArgumentType, Command, CommandNode, ConsumedArgs, StringType},
    events::{EventHandler, EventPriority, PlayerChatEvent, PlayerLeaveEvent, ServerTickEndEvent},
    text::TextComponent,
    wit::pumpkin::plugin::{
        command::Arg,
        event::{PlayerChatEventData, PlayerLeaveEventData, ServerTickEndEventData},
    },
    Context, Player, Server,
};

use crate::bubble::{remove_player_bubble, spawn_chat_bubble, tick_bubbles};
use crate::filter::{
    apply_anti_caps, apply_color_codes, contains_advertisement, is_duplicate_flood,
    remaining_cooldown_secs, sanitize_input,
};
use crate::models::{is_within_local_radius, normalize_key, Channel};
use crate::sounds::{play_error_sound, play_pm_sound, play_trade_sound};
use crate::storage::{now_ms, with_state};

pub const PERM_USE: &str = "kronos-chat:use";
pub const PERM_CHAT_COLOR: &str = "kronos-chat:color";
pub const PERM_CHAT_COLOR_ALL: &str = "kronos-chat:color.all";
pub const PERM_BYPASS_COOLDOWN: &str = "kronos-chat:bypass-cooldown";
pub const PERM_BYPASS_FILTER: &str = "kronos-chat:bypass-filter";
pub const PERM_SOCIALSPY: &str = "kronos-chat:socialspy";

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

pub fn extract_args(args: &ConsumedArgs) -> String {
    match args.get_value("args") {
        Arg::Simple(s) | Arg::Msg(s) => s,
        _ => String::new(),
    }
}

pub fn find_online_player_ci(server: &Server, target: &str) -> Option<Player> {
    let key = normalize_key(target);
    server
        .get_all_players()
        .into_iter()
        .find(|p| normalize_key(&p.get_name()) == key)
}

fn player_can_color(player: &Player) -> bool {
    player.has_permission(PERM_CHAT_COLOR)
        || player.has_permission(PERM_CHAT_COLOR_ALL)
        || player.has_permission("kronos-ranks:chat.color")
        || player.has_permission("kronos-ranks:chat.color.all")
}

/// Основной конвейер доставки сообщения в канал (Локальный / Глобальный / Торговый).
pub fn deliver_channel_message(
    player: &Player,
    server: &Server,
    channel: Channel,
    raw_body: &str,
) {
    let clean = sanitize_input(raw_body);
    if clean.is_empty() {
        play_error_sound(player);
        player.send_system_message(legacy("§6§l✦ KRONOS CHAT §8» §cСообщение не может быть пустым."), false);
        return;
    }

    let sender_name = player.get_name();
    let sender_key = normalize_key(&sender_name);
    let bypass_cd = player.has_permission(PERM_BYPASS_COOLDOWN);
    let bypass_filter = player.has_permission(PERM_BYPASS_FILTER);
    let now = now_ms();

    // 1. Анти-реклама
    if !bypass_filter && contains_advertisement(&clean) {
        play_error_sound(player);
        player.send_system_message(
            legacy("§6§l✦ KRONOS CHAT §8» §c✗ Реклама сторонних серверов, IP и ссылок запрещена!"),
            false,
        );
        return;
    }

    // 2. Анти-капс
    let processed = if bypass_filter { clean } else { apply_anti_caps(&clean) };

    // 3. Проверка анти-флуда и кулдауна под одним Mutex
    let check: Result<f64, String> = with_state(|st| {
        if !bypass_filter && is_duplicate_flood(st.last_message.get(&sender_key).map(String::as_str), &processed) {
            return Err("Не отправляйте одинаковые сообщения подряд!".to_string());
        }
        if !bypass_cd {
            match channel {
                Channel::Global => {
                    if let Some(secs) = remaining_cooldown_secs(
                        st.last_global_ms.get(&sender_key).copied(),
                        now,
                        st.store.config.global_cooldown_ms,
                    ) {
                        return Err(format!("Подождите §e{secs}с §cперед следующим сообщением в глобальный чат."));
                    }
                    st.last_global_ms.insert(sender_key.clone(), now);
                }
                Channel::Trade => {
                    if let Some(secs) = remaining_cooldown_secs(
                        st.last_trade_ms.get(&sender_key).copied(),
                        now,
                        st.store.config.trade_cooldown_ms,
                    ) {
                        return Err(format!("Подождите §e{secs}с §cперед следующим торговым объявлением."));
                    }
                    st.last_trade_ms.insert(sender_key.clone(), now);
                }
                Channel::Local => {}
            }
        }
        st.last_message.insert(sender_key.clone(), processed.clone());
        Ok(st.store.config.local_radius)
    });

    let local_radius = match check {
        Ok(r) => r,
        Err(reason) => {
            play_error_sound(player);
            player.send_system_message(legacy(&format!("§6§l✦ KRONOS CHAT §8» §c✗ {reason}")), false);
            return;
        }
    };

    let styled_text = apply_color_codes(&processed, player_can_color(player));
    let formatted_line = channel.format_line("", &sender_name, &styled_text);
    let online = server.get_all_players();

    let ignored_by: Vec<String> = with_state(|st| {
        online
            .iter()
            .filter(|p| st.store.is_ignoring(&p.get_name(), &sender_name))
            .map(|p| normalize_key(&p.get_name()))
            .collect()
    });

    match channel {
        Channel::Local => {
            let s_world = player.get_world().get_name();
            let s_pos = player.get_position();
            let mut heard_others = 0usize;

            for listener in &online {
                let l_key = normalize_key(&listener.get_name());
                let is_self = l_key == sender_key;
                if !is_self && ignored_by.contains(&l_key) {
                    continue;
                }
                if is_self
                    || is_within_local_radius(
                        &s_world,
                        s_pos,
                        &listener.get_world().get_name(),
                        listener.get_position(),
                        local_radius,
                    )
                {
                    listener.send_system_message(legacy(&formatted_line), false);
                    if !is_self {
                        heard_others += 1;
                    }
                }
            }

            if heard_others == 0 {
                player.send_system_message(legacy("§7(Вас никто не услышал)"), false);
            }

            // Облачко текста над головой только для локального чата
            spawn_chat_bubble(player, server, &processed);
        }
        Channel::Global | Channel::Trade => {
            for listener in &online {
                let l_key = normalize_key(&listener.get_name());
                if l_key != sender_key && ignored_by.contains(&l_key) {
                    continue;
                }
                listener.send_system_message(legacy(&formatted_line), false);
                if channel == Channel::Trade {
                    play_trade_sound(listener);
                }
            }
        }
    }
}

/// Доставляет личное сообщение с проверкой ЧС, звуком колокольчика и рассылкой в SocialSpy.
pub fn send_private_message(sender: &Player, server: &Server, target_name: &str, raw_text: &str) {
    let clean = sanitize_input(raw_text);
    if clean.is_empty() {
        play_error_sound(sender);
        sender.send_system_message(legacy("§6§l✦ KRONOS CHAT §8» §cВведите текст сообщения."), false);
        return;
    }

    let sender_name = sender.get_name();
    let Some(target) = find_online_player_ci(server, target_name) else {
        play_error_sound(sender);
        sender.send_system_message(
            legacy(&format!("§6§l✦ KRONOS CHAT §8» §cИгрок §f{target_name} §cне в сети.")),
            false,
        );
        return;
    };

    let resolved_target = target.get_name();
    if normalize_key(&sender_name) == normalize_key(&resolved_target) {
        play_error_sound(sender);
        sender.send_system_message(legacy("§6§l✦ KRONOS CHAT §8» §cНельзя писать ЛС самому себе."), false);
        return;
    }

    let (is_ignored, spy_keys) = with_state(|st| {
        let ignored = st.store.is_ignoring(&resolved_target, &sender_name);
        if !ignored {
            st.last_pm_partner
                .insert(normalize_key(&sender_name), resolved_target.clone());
            st.last_pm_partner
                .insert(normalize_key(&resolved_target), sender_name.clone());
        }
        (ignored, st.store.social_spy.clone())
    });

    if is_ignored {
        play_error_sound(sender);
        sender.send_system_message(
            legacy(&format!("§6§l✦ KRONOS CHAT §8» §cИгрок §f{resolved_target} §cдобавил вас в чёрный список.")),
            false,
        );
        return;
    }

    let styled = apply_color_codes(&clean, player_can_color(sender));
    sender.send_system_message(
        legacy(&format!("§8[§dЯ §8-> §d{resolved_target}§8] §f{styled}")),
        false,
    );
    target.send_system_message(
        legacy(&format!("§8[§d{sender_name} §8-> §dЯ§8] §f{styled}")),
        false,
    );
    play_pm_sound(&target);

    if !spy_keys.is_empty() {
        let spy_line = format!("§8[§cSPY§8] §7{sender_name} §8-> §7{resolved_target}§8: §f{styled}");
        let s_key = normalize_key(&sender_name);
        let t_key = normalize_key(&resolved_target);
        for p in server.get_all_players() {
            let pk = normalize_key(&p.get_name());
            if pk != s_key && pk != t_key && spy_keys.contains(&pk) && p.has_permission(PERM_SOCIALSPY) {
                p.send_system_message(legacy(&spy_line), false);
            }
        }
    }
}

pub fn register_all(context: &Context) {
    let msg_cmd = Command::new(&["msg".into(), "tell".into(), "w".into()], "Личное сообщение (/msg <ник> <текст>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(msg::MsgCmd))
        .execute(msg::MsgCmd);
    context.register_command(msg_cmd, PERM_USE);

    let reply_cmd = Command::new(&["r".into(), "reply".into()], "Ответить на последнее ЛС (/r <текст>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(reply::ReplyCmd))
        .execute(reply::ReplyCmd);
    context.register_command(reply_cmd, PERM_USE);

    let trade_cmd = Command::new(&["trade".into(), "ad".into()], "Торговый чат (/trade <текст>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(trade::TradeCmd))
        .execute(trade::TradeCmd);
    context.register_command(trade_cmd, PERM_USE);

    let ignore_cmd = Command::new(&["ignore".into()], "Чёрный список игроков (/ignore <ник>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(ignore::IgnoreCmd))
        .execute(ignore::IgnoreCmd);
    context.register_command(ignore_cmd, PERM_USE);

    let spy_cmd = Command::new(&["socialspy".into()], "Слежка за ЛС для администрации (/socialspy)")
        .execute(spy::SocialSpyCmd);
    context.register_command(spy_cmd, PERM_SOCIALSPY);
}

struct ChatEventHandler;
impl EventHandler<PlayerChatEvent> for ChatEventHandler {
    fn handle(&self, server: Server, mut event: PlayerChatEventData) -> PlayerChatEventData {
        if event.cancelled {
            return event;
        }
        event.cancelled = true;
        let (channel, body) = Channel::parse_message(&event.message);
        deliver_channel_message(&event.player, &server, channel, body);
        event
    }
}

struct TickBubbleHandler;
impl EventHandler<ServerTickEndEvent> for TickBubbleHandler {
    fn handle(&self, server: Server, event: ServerTickEndEventData) -> ServerTickEndEventData {
        tick_bubbles(&server);
        event
    }
}

struct LeaveCleanupHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveCleanupHandler {
    fn handle(&self, server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let key = normalize_key(&event.player.get_name());
        remove_player_bubble(&server, &key);
        event
    }
}

pub fn register_events(context: &Context) -> Result<(), String> {
    context.register_event_handler::<PlayerChatEvent, _>(ChatEventHandler, EventPriority::Normal, false)?;
    context.register_event_handler::<ServerTickEndEvent, _>(TickBubbleHandler, EventPriority::Normal, false)?;
    context.register_event_handler::<PlayerLeaveEvent, _>(LeaveCleanupHandler, EventPriority::Normal, false)?;
    Ok(())
}
