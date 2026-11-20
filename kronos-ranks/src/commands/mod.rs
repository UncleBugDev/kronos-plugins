pub mod donator_cmds;
pub mod remove;
pub mod set;
pub mod staffchat;

use pumpkin_plugin_api::{
    command::{ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType},
    commands::CommandHandler,
    events::{
        BedrockFormResponseEvent, EventHandler, EventPriority, PlayerChatEvent, PlayerJoinEvent,
        PlayerLeaveEvent, ServerTickEndEvent,
    },
    wit::pumpkin::plugin::{
        command::Arg,
        event::{
            BedrockFormResponseEventData, PlayerChatEventData, PlayerJoinEventData,
            PlayerLeaveEventData, ServerTickEndEventData,
        },
    },
    Context, Player, Server,
};

use crate::menu::{java::send_rank_detail_card, open_ranks_catalog};
use crate::models::{normalize_key, BedrockRanksAction};
use crate::sounds::play_error_sound;
use crate::storage::{now_ms, save_users_store, with_state};
use crate::tab_chat::{
    find_online_player_ci, format_chat_message, legacy, refresh_online_player, refresh_player_tab,
    sync_player_permissions,
};

pub const PERM_PLAYER: &str = "kronos-ranks:player";
pub const PERM_STAFF: &str = "kronos-ranks:staff";
pub const PERM_ADMIN: &str = "kronos-ranks:admin";

pub fn extract_args(args: &ConsumedArgs) -> String {
    match args.get_value("args") {
        Arg::Simple(s) | Arg::Msg(s) => s,
        _ => String::new(),
    }
}

pub fn is_admin_sender(sender: &CommandSender) -> bool {
    if sender.is_console() {
        return true;
    }
    let Some(p) = sender.as_player() else {
        return false;
    };
    if p.has_permission(PERM_ADMIN) {
        return true;
    }
    let name = p.get_name();
    with_state(|st| {
        let user = st.users.get_or_default(&name);
        st.tree.has_permission(&user, PERM_ADMIN)
    })
}

struct RanksCatalogCmd;
impl CommandHandler for RanksCatalogCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКаталог /ranks доступен только игрокам."));
            return Ok(0);
        };
        let name = player.get_name();
        let (tree, user) = with_state(|st| (st.tree.clone(), st.users.get_or_default(&name)));
        open_ranks_catalog(&player, &tree, &user, now_ms());
        Ok(0)
    }
}

struct RankRouterCmd;
impl CommandHandler for RankRouterCmd {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let raw = extract_args(&args);
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        let sub = tokens.first().copied().unwrap_or("info").to_ascii_lowercase();

        match sub.as_str() {
            "set" => set::handle_rank_set(&sender, &server, &tokens[1..]),
            "remove" | "del" => remove::handle_rank_remove(&sender, &server, &tokens[1..]),
            "addperm" => set::handle_perm_edit(&sender, &server, &tokens[1..], true),
            "delperm" => set::handle_perm_edit(&sender, &server, &tokens[1..], false),
            "info" => set::handle_rank_info(&sender, tokens.get(1).copied()),
            _ => sender.send_message(legacy(
                "§6§l✦ KRONOS RANKS §8» §e/rank set <игрок> <ранг> [срок] §8| §e/rank remove <игрок> [ранг] §8| §e/rank addperm|delperm <ранг|игрок> <право>",
            )),
        }
        Ok(0)
    }
}

pub fn register_all(context: &Context) {
    let ranks_cmd = Command::new(&["ranks".into(), "donate".into()], "Каталог донат-рангов KRONOS (/ranks)")
        .execute(RanksCatalogCmd);
    context.register_command(ranks_cmd, PERM_PLAYER);

    let rank_cmd = Command::new(&["rank".into()], "Управление рангами и правами (/rank)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(RankRouterCmd))
        .execute(RankRouterCmd);
    context.register_command(rank_cmd, PERM_PLAYER);

    let sc_cmd = Command::new(&["sc".into(), "staffchat".into()], "Стафф-чат (/sc <текст>)")
        .then(CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(staffchat::StaffChatCmd))
        .execute(staffchat::StaffChatCmd);
    context.register_command(sc_cmd, PERM_PLAYER);

    donator_cmds::register_donator_commands(context);
}

// =============================================================================
// Event Handlers (Join, Leave, Chat, Tick Expiration, Bedrock Forms)
// =============================================================================

struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let name = event.player.get_name();
        let now = now_ms();
        let (tree, user, changed) = with_state(|st| {
            let expired = st.users.expire_ranks(now);
            let (_, created) = st.users.ensure_user(&name);
            let u = st.users.get_or_default(&name);
            (st.tree.clone(), u, created || !expired.is_empty())
        });
        if changed {
            save_users_store();
        }
        sync_player_permissions(&event.player, &tree, &user);
        refresh_player_tab(
            &event.player,
            &tree,
            &user,
            server.get_player_count().max(1),
            server.get_max_players(),
        );
        event
    }
}

struct LeaveHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(&self, _server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let key = normalize_key(&event.player.get_name());
        with_state(|st| st.pending_forms.retain(|_, f| f.player_key != key));
        event
    }
}

struct ChatHandler;
impl EventHandler<PlayerChatEvent> for ChatHandler {
    fn handle(&self, server: Server, mut event: PlayerChatEventData) -> PlayerChatEventData {
        if event.cancelled {
            return event;
        }
        let name = event.player.get_name();
        let formatted = with_state(|st| {
            let user = st.users.get_or_default(&name);
            format_chat_message(&st.tree, &user, &event.message)
        });
        event.cancelled = true;
        if event.recipients.is_empty() {
            for p in server.get_all_players() {
                p.send_system_message(legacy(&formatted), false);
            }
        } else {
            for p in &event.recipients {
                p.send_system_message(legacy(&formatted), false);
            }
        }
        event
    }
}

struct TickExpirationHandler;
impl EventHandler<ServerTickEndEvent> for TickExpirationHandler {
    fn handle(&self, server: Server, event: ServerTickEndEventData) -> ServerTickEndEventData {
        let now = now_ms();
        let expired = with_state(|st| {
            st.tick_counter = st.tick_counter.wrapping_add(1);
            if st.tick_counter % 100 == 0 {
                st.users.expire_ranks(now)
            } else {
                Vec::new()
            }
        });
        if !expired.is_empty() {
            save_users_store();
            for (player_name, old_rank) in expired {
                refresh_online_player(&server, &player_name);
                if let Some(p) = find_online_player_ci(&server, &player_name) {
                    p.send_system_message(
                        legacy(&format!(
                            "§6§l✦ KRONOS RANKS §8» §eСрок действия ранга §f{} §eистёк.",
                            old_rank
                        )),
                        false,
                    );
                }
            }
        }
        event
    }
}

struct BedrockFormHandler;
impl EventHandler<BedrockFormResponseEvent> for BedrockFormHandler {
    fn handle(&self, _server: Server, event: BedrockFormResponseEventData) -> BedrockFormResponseEventData {
        let pending_opt = with_state(|st| st.pending_forms.remove(&event.form_id));
        if let (Some(pending), Some(raw)) = (pending_opt, event.response_data.as_deref()) {
            if pending.player_key == normalize_key(&event.player.get_name()) {
                if let Ok(idx) = raw.trim().parse::<usize>() {
                    if let Some(action) = pending.actions.into_iter().nth(idx) {
                        handle_bedrock_action(&event.player, action);
                    } else {
                        play_error_sound(&event.player);
                    }
                }
            }
        }
        event
    }
}

fn handle_bedrock_action(player: &Player, action: BedrockRanksAction) {
    let name = player.get_name();
    let (tree, user) = with_state(|st| (st.tree.clone(), st.users.get_or_default(&name)));
    match action {
        BedrockRanksAction::ShowRankDetails(rank_id) => {
            if let Some(rank) = tree.get(&rank_id) {
                send_rank_detail_card(player, &tree, rank);
            }
        }
        BedrockRanksAction::RefreshCatalog => open_ranks_catalog(player, &tree, &user, now_ms()),
    }
}

pub fn register_events(context: &Context) -> Result<(), String> {
    context.register_event_handler::<PlayerJoinEvent, _>(JoinHandler, EventPriority::Normal, false)?;
    context.register_event_handler::<PlayerLeaveEvent, _>(LeaveHandler, EventPriority::Normal, false)?;
    context.register_event_handler::<PlayerChatEvent, _>(ChatHandler, EventPriority::Normal, false)?;
    context.register_event_handler::<ServerTickEndEvent, _>(TickExpirationHandler, EventPriority::Normal, false)?;
    context.register_event_handler::<BedrockFormResponseEvent, _>(BedrockFormHandler, EventPriority::Normal, false)?;
    Ok(())
}
