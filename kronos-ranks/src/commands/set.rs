use pumpkin_plugin_api::{command::CommandSender, Server};

use crate::commands::is_admin_sender;
use crate::models::{format_remaining_ru, parse_duration_ms, RankTrack};
use crate::sounds::{play_error_sound, play_rank_up_sound, play_success_sound};
use crate::storage::{now_ms, save_ranks_store, save_users_store, with_state};
use crate::tab_chat::{find_online_player_ci, legacy, refresh_online_player};

/// `/rank set <игрок> <ранг> [срок: 30d|perm]` — выдача ранга (консоль авто-доната или админ).
pub fn handle_rank_set(sender: &CommandSender, server: &Server, args: &[&str]) {
    if !is_admin_sender(sender) {
        if let Some(p) = sender.as_player() { play_error_sound(&p); }
        sender.send_error(legacy("§6§l✦ KRONOS RANKS §8» §cНедостаточно прав (kronos-ranks:admin)."));
        return;
    }
    if args.len() < 2 {
        sender.send_error(legacy("§6§l✦ KRONOS RANKS §8» §eИспользование: §f/rank set <игрок> <ранг> [30d|perm]"));
        return;
    }

    let target_name = args[0];
    let rank_id = args[1];
    let duration_raw = args.get(2).copied().unwrap_or("perm");
    let duration_ms = match parse_duration_ms(duration_raw) {
        Ok(d) => d,
        Err(err) => {
            sender.send_error(legacy(&format!("§6§l✦ KRONOS RANKS §8» §c✗ {err}")));
            return;
        }
    };

    let now = now_ms();
    let expires_at = duration_ms.map(|d| now.saturating_add(d));

    let applied = with_state(|st| {
        let Some(rank) = st.tree.get(rank_id).cloned() else {
            return Err(format!("Ранг '{rank_id}' не найден."));
        };
        let (user, _) = st.users.ensure_user(target_name);
        match rank.track {
            RankTrack::Donate => {
                user.donate_rank = rank.id.clone();
                user.donate_expires_at_ms = expires_at;
            }
            RankTrack::Staff => {
                user.staff_rank = Some(rank.id.clone());
                user.staff_expires_at_ms = expires_at;
            }
        }
        Ok(rank)
    });

    match applied {
        Ok(rank) => {
            save_users_store();
            refresh_online_player(server, target_name);
            let term = format_remaining_ru(expires_at, now);

            if let Some(p) = sender.as_player() { play_success_sound(&p); }
            sender.send_message(legacy(&format!(
                "§6§l✦ KRONOS RANKS §8» §aИгроку §f{target_name} §aвыдан ранг {}§f{} §8(срок: §e{term}§8).",
                rank.prefix, rank.display_name
            )));

            if let Some(target) = find_online_player_ci(server, target_name) {
                play_rank_up_sound(&target);
                target.send_system_message(
                    legacy(&format!(
                        "§6§l✦ KRONOS RANKS §8» §aВам выдан ранг {}§f{} §8(срок: §e{term}§8)!",
                        rank.prefix, rank.display_name
                    )),
                    false,
                );
            }
        }
        Err(err) => {
            if let Some(p) = sender.as_player() { play_error_sound(&p); }
            sender.send_error(legacy(&format!("§6§l✦ KRONOS RANKS §8» §c✗ {err}")));
        }
    }
}

/// `/rank addperm|delperm <ранг|игрок> <право>`
pub fn handle_perm_edit(sender: &CommandSender, server: &Server, args: &[&str], add: bool) {
    if !is_admin_sender(sender) {
        if let Some(p) = sender.as_player() { play_error_sound(&p); }
        sender.send_error(legacy("§6§l✦ KRONOS RANKS §8» §cНедостаточно прав (kronos-ranks:admin)."));
        return;
    }
    if args.len() < 2 {
        let cmd = if add { "addperm" } else { "delperm" };
        sender.send_error(legacy(&format!("§6§l✦ KRONOS RANKS §8» §eИспользование: §f/rank {cmd} <ранг|игрок> <право>")));
        return;
    }

    let target = args[0];
    let node = args[1].trim().to_string();

    let is_rank = with_state(|st| {
        if let Some(rank) = st.tree.get_mut(target) {
            if add {
                if !rank.permissions.contains(&node) {
                    rank.permissions.push(node.clone());
                }
            } else {
                rank.permissions.retain(|p| p != &node);
            }
            return true;
        }
        let (user, _) = st.users.ensure_user(target);
        if add {
            user.denied_permissions.retain(|p| p != &node);
            if !user.extra_permissions.contains(&node) {
                user.extra_permissions.push(node.clone());
            }
        } else {
            user.extra_permissions.retain(|p| p != &node);
            if !user.denied_permissions.contains(&node) {
                user.denied_permissions.push(node.clone());
            }
        }
        false
    });

    if is_rank {
        save_ranks_store();
        for p in server.get_all_players() {
            refresh_online_player(server, &p.get_name());
        }
    } else {
        save_users_store();
        refresh_online_player(server, target);
    }

    if let Some(p) = sender.as_player() { play_success_sound(&p); }
    let verb = if add { "добавлено" } else { "удалено" };
    let scope = if is_rank { "ранга" } else { "игрока" };
    sender.send_message(legacy(&format!(
        "§6§l✦ KRONOS RANKS §8» §aПраво §e{node} §a{verb} для {scope} §f{target}§a."
    )));
}

pub fn handle_rank_info(sender: &CommandSender, target_opt: Option<&str>) {
    let target = match target_opt {
        Some(t) => t.to_string(),
        None => match sender.as_player() {
            Some(p) => p.get_name(),
            None => {
                sender.send_error(legacy("§cУкажите ник: /rank info <игрок>"));
                return;
            }
        },
    };

    let now = now_ms();
    let (donate_label, donate_term, staff_label, perm_count) = with_state(|st| {
        let user = st.users.get_or_default(&target);
        let donate_term = format_remaining_ru(user.donate_expires_at_ms, now);
        let d_name = st.tree.get(&user.donate_rank).map(|r| format!("{}{}", r.prefix, r.display_name)).unwrap_or(user.donate_rank.clone());
        let s_name = user.staff_rank.as_deref()
            .and_then(|id| st.tree.get(id))
            .map(|r| format!("{}{}", r.prefix, r.display_name))
            .unwrap_or_else(|| "§7Нет".to_string());
        let perms = st.tree.effective_permissions(&user).len();
        (d_name, donate_term, s_name, perms)
    });

    sender.send_message(legacy(&format!(
        "§6§l✦ ПРОФИЛЬ §f{target} §8| §7Донат: {donate_label} §8({donate_term}) §8| §7Стафф: {staff_label} §8| §7Прав: §b{perm_count}"
    )));
}
