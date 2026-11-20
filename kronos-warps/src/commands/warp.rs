use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Player, Server,
};

use crate::commands::{
    capture_location, extract_args, open_catalog_for_player, resolve_world, PERM_ADMIN,
    PERM_BYPASS_WARMUP,
};
use crate::menu::legacy;
use crate::models::{
    format_cost_minor, normalize_key, CatalogTab, PendingWarmup, Warp, MOVE_CANCEL_DIST_SQ,
    WARMUP_MS,
};
use crate::sounds::{
    play_coin_sound, play_error_sound, play_teleport_sound, play_warmup_tick_sound,
};
use crate::storage::{charge_warp_entry, now_ms, save_current_store, with_state};

pub struct WarpCmd;
impl CommandHandler for WarpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /warp доступна только игрокам."));
            return Ok(0);
        };
        let raw = extract_args(&args);
        let Some(target) = raw.split_whitespace().next() else {
            open_catalog_for_player(&player, CatalogTab::Server);
            return Ok(0);
        };
        initiate_warp(&player, &server, target);
        Ok(0)
    }
}

pub struct WarpsCatalogCmd;
impl CommandHandler for WarpsCatalogCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКаталог /warps доступен только игрокам."));
            return Ok(0);
        };
        let tab = match extract_args(&args).trim().to_ascii_lowercase().as_str() {
            "popular" | "players" | "top" => CatalogTab::Popular,
            "mine" | "my" => CatalogTab::Mine,
            _ => CatalogTab::Server,
        };
        open_catalog_for_player(&player, tab);
        Ok(0)
    }
}

pub fn initiate_warp(player: &Player, server: &Server, warp_name: &str) {
    let warp_opt = with_state(|st| st.store.get(warp_name).cloned());
    let Some(warp) = warp_opt else {
        play_error_sound(player);
        player.send_system_message(
            legacy(&format!("§6§l✦ KRONOS WARPS §8» §cВарп §f{} §cне найден.", warp_name)),
            false,
        );
        return;
    };

    let is_admin = player.has_permission(PERM_ADMIN);
    if !warp.can_access(is_admin, |perm| player.has_permission(perm)) {
        play_error_sound(player);
        let req = warp.permission.as_deref().unwrap_or(PERM_ADMIN);
        player.send_system_message(
            legacy(&format!(
                "§6§l✦ KRONOS WARPS §8» §cНедостаточно прав для входа (требуется §f{}§c).",
                req
            )),
            false,
        );
        return;
    }

    if is_admin || player.has_permission(PERM_BYPASS_WARMUP) {
        complete_warp_teleport(player, server, &warp);
        return;
    }

    let origin = capture_location(player);
    let now = now_ms();
    let p_name = player.get_name();
    let p_key = normalize_key(&p_name);

    with_state(|st| {
        st.pending_warmups.insert(
            p_key,
            PendingWarmup {
                player_name: p_name,
                warp_name: warp.name.clone(),
                origin,
                ready_at_ms: now.saturating_add(WARMUP_MS),
                last_notified_sec: 3,
            },
        );
    });

    play_warmup_tick_sound(player, 3);
    player.send_system_message(
        legacy(&format!(
            "§6§l✦ KRONOS WARPS §8» §eТелепортация на §f{} §eчерез §63 сек. §7Не двигайтесь и не получайте урон!",
            warp.name
        )),
        false,
    );
}

pub fn tick_warmups(server: &Server) {
    let pending_list: Vec<(String, PendingWarmup)> = with_state(|st| {
        st.pending_warmups
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    });
    if pending_list.is_empty() {
        return;
    }

    let now = now_ms();
    let online = server.get_all_players();

    for (p_key, pending) in pending_list {
        let Some(player) = online
            .iter()
            .find(|p| normalize_key(&p.get_name()) == p_key)
        else {
            with_state(|st| {
                st.pending_warmups.remove(&p_key);
            });
            continue;
        };

        let pos = player.get_position();
        if pending.origin.distance_sq_to(pos) > MOVE_CANCEL_DIST_SQ {
            with_state(|st| {
                st.pending_warmups.remove(&p_key);
            });
            play_error_sound(player);
            player.show_actionbar(legacy("§c✗ Телепортация отменена: вы сдвинулись с места!"));
            continue;
        }

        if now >= pending.ready_at_ms {
            let warp_opt = with_state(|st| {
                st.pending_warmups.remove(&p_key);
                st.store.get(&pending.warp_name).cloned()
            });
            if let Some(warp) = warp_opt {
                complete_warp_teleport(player, server, &warp);
            }
            continue;
        }

        let secs_left = (pending.ready_at_ms - now).div_ceil(1000);
        if secs_left < pending.last_notified_sec {
            with_state(|st| {
                if let Some(entry) = st.pending_warmups.get_mut(&p_key) {
                    entry.last_notified_sec = secs_left;
                }
            });
            play_warmup_tick_sound(player, secs_left);
            player.show_actionbar(legacy(&format!(
                "§6✦ Телепортация на §f{} §6через §e{}с...",
                pending.warp_name, secs_left
            )));
        }
    }
}

pub fn cancel_warmup_on_damage(player: &Player) {
    let key = normalize_key(&player.get_name());
    let removed = with_state(|st| st.pending_warmups.remove(&key).is_some());
    if removed {
        play_error_sound(player);
        player.show_actionbar(legacy("§c✗ Телепортация отменена: получен урон!"));
    }
}

fn complete_warp_teleport(player: &Player, server: &Server, warp: &Warp) {
    let Some(world) = resolve_world(server, &warp.location.world) else {
        play_error_sound(player);
        player.send_system_message(
            legacy("§6§l✦ KRONOS WARPS §8» §cЦелевой мир недоступен."),
            false,
        );
        return;
    };

    if warp.cost_minor > 0 && normalize_key(&player.get_name()) != normalize_key(&warp.owner) {
        let data_folder = with_state(|st| st.data_folder.clone());
        if let Err(err) = charge_warp_entry(&data_folder, &player.get_name(), &warp.owner, warp.cost_minor) {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)),
                false,
            );
            return;
        }
        play_coin_sound(player);
    }

    let visits = with_state(|st| st.store.record_visit(&warp.name).unwrap_or(warp.visits + 1));
    save_current_store();

    player.teleport(
        (warp.location.x, warp.location.y, warp.location.z),
        Some(warp.location.yaw),
        Some(warp.location.pitch),
        world,
    );
    play_teleport_sound(player);

    let paid_note = if warp.cost_minor > 0 && normalize_key(&player.get_name()) != normalize_key(&warp.owner) {
        format!(" §7(оплачено §e{}§7)", format_cost_minor(warp.cost_minor))
    } else {
        String::new()
    };
    player.show_actionbar(legacy(&format!(
        "§a✦ Вы на варпе §f{} §8| §b{} визитов",
        warp.name, visits
    )));
    player.send_system_message(
        legacy(&format!(
            "§6§l✦ KRONOS WARPS §8» §aВы телепортированы на §f{}{}§a.",
            warp.name, paid_note
        )),
        false,
    );
}
