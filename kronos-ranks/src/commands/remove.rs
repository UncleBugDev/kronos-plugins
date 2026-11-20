use pumpkin_plugin_api::{command::CommandSender, Server};

use crate::commands::is_admin_sender;
use crate::models::{normalize_key, RankTrack};
use crate::sounds::{play_error_sound, play_success_sound};
use crate::storage::{save_users_store, with_state};
use crate::tab_chat::{find_online_player_ci, legacy, refresh_online_player};

/// `/rank remove <игрок> [ранг|donate|staff]` — снятие ранга.
pub fn handle_rank_remove(sender: &CommandSender, server: &Server, args: &[&str]) {
    if !is_admin_sender(sender) {
        if let Some(p) = sender.as_player() {
            play_error_sound(&p);
        }
        sender.send_error(legacy("§6§l✦ KRONOS RANKS §8» §cНедостаточно прав (kronos-ranks:admin)."));
        return;
    }
    let Some(target_name) = args.first().copied() else {
        sender.send_error(legacy("§6§l✦ KRONOS RANKS §8» §eИспользование: §f/rank remove <игрок> [ранг|donate|staff]"));
        return;
    };
    let selector = args.get(1).copied().map(normalize_key);

    let removed_desc = with_state(|st| {
        let (user, _) = st.users.ensure_user(target_name);
        match selector.as_deref() {
            None | Some("all") => {
                user.donate_rank = "player".to_string();
                user.donate_expires_at_ms = None;
                user.staff_rank = None;
                user.staff_expires_at_ms = None;
                Ok("все ранги (сброс до Игрока)".to_string())
            }
            Some("donate") => {
                user.donate_rank = "player".to_string();
                user.donate_expires_at_ms = None;
                Ok("донат-ранг".to_string())
            }
            Some("staff") => {
                user.staff_rank = None;
                user.staff_expires_at_ms = None;
                Ok("стафф-ранг".to_string())
            }
            Some(specific) => {
                if let Some(rank) = st.tree.get(specific) {
                    match rank.track {
                        RankTrack::Donate if user.donate_rank == rank.id => {
                            user.donate_rank = "player".to_string();
                            user.donate_expires_at_ms = None;
                            Ok(format!("ранг {}", rank.display_name))
                        }
                        RankTrack::Staff if user.staff_rank.as_deref() == Some(rank.id.as_str()) => {
                            user.staff_rank = None;
                            user.staff_expires_at_ms = None;
                            Ok(format!("стафф-ранг {}", rank.display_name))
                        }
                        _ => Err(format!("У игрока {target_name} нет ранга '{}'.", rank.id)),
                    }
                } else {
                    Err(format!("Неизвестный ранг '{specific}'."))
                }
            }
        }
    });

    match removed_desc {
        Ok(desc) => {
            save_users_store();
            refresh_online_player(server, target_name);
            if let Some(p) = sender.as_player() {
                play_success_sound(&p);
            }
            sender.send_message(legacy(&format!(
                "§6§l✦ KRONOS RANKS §8» §aУ игрока §f{target_name} §aснят: §e{desc}§a."
            )));
            if let Some(target) = find_online_player_ci(server, target_name) {
                target.send_system_message(
                    legacy(&format!("§6§l✦ KRONOS RANKS §8» §eВаш {desc} был снят администратором.")),
                    false,
                );
            }
        }
        Err(err) => {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(&format!("§6§l✦ KRONOS RANKS §8» §c✗ {err}")));
        }
    }
}
