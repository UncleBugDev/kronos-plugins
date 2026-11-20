pub mod delwarp;
pub mod pwarp;
pub mod setwarp;
pub mod warp;

use pumpkin_plugin_api::{
    command::{ArgumentType, Command, CommandNode, CommandSender, ConsumedArgs, StringType},
    events::{
        BedrockFormResponseEvent, EntityDamageEvent, EventHandler, EventPriority, PlayerLeaveEvent,
        ServerTickEndEvent,
    },
    wit::pumpkin::plugin::{
        command::Arg,
        event::{
            BedrockFormResponseEventData, EntityDamageEventData, PlayerLeaveEventData,
            ServerTickEndEventData,
        },
        world::World,
    },
    Context, Player, Server,
};

use crate::menu::open_warps_catalog;
use crate::models::{normalize_key, BedrockWarpAction, CatalogTab, WarpLocation};
use crate::sounds::play_error_sound;
use crate::storage::with_state;

pub const PERM_USE: &str = "kronos-warps:use";
pub const PERM_PWARP_CREATE: &str = "kronos-warps:pwarp";
pub const PERM_VIP: &str = "kronos-warps:vip";
pub const PERM_PWARP_ELITE: &str = "kronos-warps:elite";
pub const PERM_BYPASS_WARMUP: &str = "kronos-warps:bypass-warmup";
pub const PERM_ADMIN: &str = "kronos-warps:admin";

pub fn extract_args(args: &ConsumedArgs) -> String {
    match args.get_value("args") {
        Arg::Simple(s) | Arg::Msg(s) => s,
        _ => String::new(),
    }
}

pub fn is_admin_sender(sender: &CommandSender) -> bool {
    sender.is_console() || sender.as_player().is_some_and(|p| p.has_permission(PERM_ADMIN))
}

pub fn player_warp_limit(player: &Player) -> usize {
    if player.has_permission(PERM_ADMIN) {
        25
    } else if player.has_permission(PERM_PWARP_ELITE) {
        7
    } else if player.has_permission(PERM_VIP) {
        3
    } else {
        1
    }
}

pub fn capture_location(player: &Player) -> WarpLocation {
    let pos = player.get_position();
    let world_name = player.get_world().get_name();
    WarpLocation::rounded(
        world_name,
        pos.0,
        pos.1,
        pos.2,
        player.get_yaw(),
        player.get_pitch(),
    )
}

pub fn resolve_world(server: &Server, world_name: &str) -> Option<World> {
    if let Some(world) = server.get_world_by_name(world_name) {
        return Some(world);
    }
    if world_name.eq_ignore_ascii_case("world")
        || world_name.eq_ignore_ascii_case("overworld")
        || world_name.eq_ignore_ascii_case("minecraft:overworld")
    {
        if let Some(w) = server.get_world_by_name("minecraft:overworld") {
            return Some(w);
        }
        if let Some(w) = server.get_world_by_name("world") {
            return Some(w);
        }
    }
    server.get_all_worlds().into_iter().next()
}

pub fn open_catalog_for_player(player: &Player, tab: CatalogTab) {
    let name = player.get_name();
    let is_admin = player.has_permission(PERM_ADMIN);
    let (server_warps, popular_warps, my_warps) = with_state(|st| {
        (
            st.store.server_warps(),
            st.store.popular_player_warps(15),
            st.store.player_warps_of(&name),
        )
    });
    open_warps_catalog(player, tab, &server_warps, &popular_warps, &my_warps, is_admin);
}

pub fn execute_bedrock_action(player: &Player, server: &Server, action: BedrockWarpAction) {
    match action {
        BedrockWarpAction::OpenTab(tab) => open_catalog_for_player(player, tab),
        BedrockWarpAction::TeleportTo(warp_name) => {
            warp::initiate_warp(player, server, &warp_name);
        }
    }
}

pub fn register_all(context: &Context) {
    let warp_cmd = Command::new(&["warp".into()], "Телепортация на варп (/warp <имя>)")
        .then(
            CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
                .execute(warp::WarpCmd),
        )
        .execute(warp::WarpCmd);
    context.register_command(warp_cmd, PERM_USE);

    let warps_cmd = Command::new(&["warps".into()], "Интерактивный каталог варпов (/warps)")
        .then(
            CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
                .execute(warp::WarpsCatalogCmd),
        )
        .execute(warp::WarpsCatalogCmd);
    context.register_command(warps_cmd, PERM_USE);

    let setwarp_cmd = Command::new(
        &["setwarp".into()],
        "Создать серверный варп (/setwarp <имя> [описание] [право])",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(setwarp::SetWarpCmd),
    )
    .execute(setwarp::SetWarpCmd);
    context.register_command(setwarp_cmd, PERM_ADMIN);

    let delwarp_cmd = Command::new(&["delwarp".into()], "Удалить серверный варп (/delwarp <имя>)")
        .then(
            CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
                .execute(delwarp::DelWarpCmd),
        )
        .execute(delwarp::DelWarpCmd);
    context.register_command(delwarp_cmd, PERM_ADMIN);

    let pwarp_cmd = Command::new(
        &["pwarp".into()],
        "Управление личными варпами (/pwarp create|delete|setcost|list)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(pwarp::PlayerWarpCmd),
    )
    .execute(pwarp::PlayerWarpCmd);
    context.register_command(pwarp_cmd, PERM_USE);
}

// =============================================================================
// Event Handlers (Warmup Tick, Damage Cancel, Leave Cleanup, Bedrock Forms)
// =============================================================================

struct TickWarmupHandler;
impl EventHandler<ServerTickEndEvent> for TickWarmupHandler {
    fn handle(&self, server: Server, event: ServerTickEndEventData) -> ServerTickEndEventData {
        warp::tick_warmups(&server);
        event
    }
}

struct DamageWarmupHandler;
impl EventHandler<EntityDamageEvent> for DamageWarmupHandler {
    fn handle(&self, server: Server, event: EntityDamageEventData) -> EntityDamageEventData {
        if !event.cancelled && event.damage > 0.0 {
            if let Some(player) = server
                .get_all_players()
                .into_iter()
                .find(|p| p.as_entity().get_id() as i32 == event.entity_id)
            {
                warp::cancel_warmup_on_damage(&player);
            }
        }
        event
    }
}

struct LeaveCleanupHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveCleanupHandler {
    fn handle(&self, _server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let key = normalize_key(&event.player.get_name());
        with_state(|st| {
            st.pending_warmups.remove(&key);
            st.pending_forms.retain(|_, f| f.player_key != key);
        });
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
        let pending_opt = with_state(|st| st.pending_forms.remove(&event.form_id));
        if let (Some(pending), Some(raw)) = (pending_opt, event.response_data.as_deref()) {
            if pending.player_key == normalize_key(&event.player.get_name()) {
                if let Ok(idx) = raw.trim().parse::<usize>() {
                    if let Some(action) = pending.actions.into_iter().nth(idx) {
                        execute_bedrock_action(&event.player, &server, action);
                    } else {
                        play_error_sound(&event.player);
                    }
                }
            }
        }
        event
    }
}

pub fn register_events(context: &Context) -> Result<(), String> {
    context.register_event_handler::<ServerTickEndEvent, _>(
        TickWarmupHandler,
        EventPriority::Normal,
        false,
    )?;
    context.register_event_handler::<EntityDamageEvent, _>(
        DamageWarmupHandler,
        EventPriority::Normal,
        false,
    )?;
    context.register_event_handler::<PlayerLeaveEvent, _>(
        LeaveCleanupHandler,
        EventPriority::Normal,
        false,
    )?;
    context.register_event_handler::<BedrockFormResponseEvent, _>(
        BedrockFormHandler,
        EventPriority::Normal,
        false,
    )?;
    Ok(())
}
