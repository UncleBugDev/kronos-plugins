use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Player, Server,
};

use crate::commands::{
    capture_location, extract_args, player_warp_limit, PERM_ADMIN, PERM_PWARP_CREATE,
};
use crate::menu::{java::send_warp_card, legacy, LINE_SEP};
use crate::models::{format_cost_minor, parse_cost_minor};
use crate::sounds::{play_click_sound, play_error_sound, play_success_sound};
use crate::storage::{now_ms, save_current_store, with_state};

pub struct PlayerWarpCmd;

impl CommandHandler for PlayerWarpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /pwarp доступна только игрокам."));
            return Ok(0);
        };

        let raw = extract_args(&args);
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        let sub = tokens.first().copied().unwrap_or("help").to_ascii_lowercase();

        match sub.as_str() {
            "create" => handle_create(&player, &tokens[1..]),
            "delete" | "del" | "remove" => handle_delete(&player, &tokens[1..]),
            "setcost" | "cost" => handle_setcost(&player, &tokens[1..]),
            "list" => handle_list(&player, tokens.get(1).copied()),
            _ => {
                play_click_sound(&player);
                player.send_system_message(
                    legacy("§6§l✦ PWARP §8» §e/pwarp create <имя> [цена] §8| §e/pwarp delete <имя> §8| §e/pwarp setcost <имя> <сумма> §8| §e/pwarp list [игрок]"),
                    false,
                );
            }
        }
        Ok(0)
    }
}

fn handle_create(player: &Player, args: &[&str]) {
    if !player.has_permission(PERM_PWARP_CREATE) && !player.has_permission(PERM_ADMIN) {
        play_error_sound(player);
        player.send_system_message(
            legacy("§6§l✦ KRONOS WARPS §8» §cУ вас нет права на создание личных варпов."),
            false,
        );
        return;
    }
    let Some(name) = args.first() else {
        play_error_sound(player);
        player.send_system_message(
            legacy("§6§l✦ KRONOS WARPS §8» §eИспользование: §f/pwarp create <имя> [цена_входа]"),
            false,
        );
        return;
    };

    let cost_minor = match args.get(1) {
        Some(raw_cost) => match parse_cost_minor(raw_cost) {
            Ok(v) => v,
            Err(err) => {
                play_error_sound(player);
                player.send_system_message(
                    legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)),
                    false,
                );
                return;
            }
        },
        None => 0,
    };

    let owner = player.get_name();
    let loc = capture_location(player);
    let limit = player_warp_limit(player);
    let now = now_ms();

    let result = with_state(|st| {
        st.store
            .create_player_warp(name, &owner, loc, cost_minor, limit, now)
    });

    match result {
        Ok(warp) => {
            save_current_store();
            play_success_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS WARPS §8» §aЛичный варп §f{} §aсоздан! Цена входа: §e{}",
                    warp.name,
                    format_cost_minor(warp.cost_minor)
                )),
                false,
            );
        }
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)),
                false,
            );
        }
    }
}

fn handle_delete(player: &Player, args: &[&str]) {
    let Some(name) = args.first() else {
        play_error_sound(player);
        player.send_system_message(
            legacy("§6§l✦ KRONOS WARPS §8» §eИспользование: §f/pwarp delete <имя>"),
            false,
        );
        return;
    };
    let actor = player.get_name();
    let is_admin = player.has_permission(PERM_ADMIN);

    let result = with_state(|st| st.store.delete_player_warp(name, &actor, is_admin));
    match result {
        Ok(removed) => {
            save_current_store();
            play_success_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS WARPS §8» §aЛичный варп §f{} §aудалён.",
                    removed.name
                )),
                false,
            );
        }
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)),
                false,
            );
        }
    }
}

fn handle_setcost(player: &Player, args: &[&str]) {
    if args.len() < 2 {
        play_error_sound(player);
        player.send_system_message(
            legacy("§6§l✦ KRONOS WARPS §8» §eИспользование: §f/pwarp setcost <имя> <сумма>"),
            false,
        );
        return;
    }
    let cost_minor = match parse_cost_minor(args[1]) {
        Ok(v) => v,
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)),
                false,
            );
            return;
        }
    };

    let actor = player.get_name();
    let is_admin = player.has_permission(PERM_ADMIN);
    let result = with_state(|st| {
        st.store
            .set_player_warp_cost(args[0], &actor, is_admin, cost_minor)
    });

    match result {
        Ok(warp) => {
            save_current_store();
            play_success_sound(player);
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS WARPS §8» §aЦена входа на §f{} §aобновлена: §e{}",
                    warp.name,
                    format_cost_minor(warp.cost_minor)
                )),
                false,
            );
        }
        Err(err) => {
            play_error_sound(player);
            player.send_system_message(
                legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)),
                false,
            );
        }
    }
}

fn handle_list(player: &Player, target_opt: Option<&str>) {
    play_click_sound(player);
    let target = target_opt
        .map(|s| s.to_string())
        .unwrap_or_else(|| player.get_name());
    let warps = with_state(|st| st.store.player_warps_of(&target));

    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§6§l✦ ВАРПЫ ИГРОКА §f{} §8({})",
            target,
            warps.len()
        )),
        false,
    );
    if warps.is_empty() {
        player.send_system_message(legacy(" §7У этого игрока пока нет личных варпов."), false);
    } else {
        for warp in &warps {
            send_warp_card(player, warp);
        }
    }
    player.send_system_message(legacy(LINE_SEP), false);
}
