use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{capture_location, extract_args, is_admin_sender, PERM_ADMIN};
use crate::menu::legacy;
use crate::sounds::{play_error_sound, play_success_sound};
use crate::storage::{now_ms, save_current_store, with_state};

pub struct SetWarpCmd;

impl CommandHandler for SetWarpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /setwarp доступна только игрокам."));
            return Ok(0);
        };
        if !is_admin_sender(&sender) {
            play_error_sound(&player);
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS WARPS §8» §cНедостаточно прав (требуется {PERM_ADMIN})."
                )),
                false,
            );
            return Ok(0);
        }

        let raw = extract_args(&args);
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        if tokens.is_empty() {
            play_error_sound(&player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS WARPS §8» §eИспользование: §f/setwarp <имя> [описание] [право]"),
                false,
            );
            return Ok(0);
        }

        let name = tokens[0];
        let (description, permission) = parse_desc_and_perm(&tokens[1..]);
        let location = capture_location(&player);
        let author = player.get_name();
        let now = now_ms();

        let result = with_state(|st| {
            st.store
                .set_server_warp(name, location, description, permission, &author, now)
        });

        match result {
            Ok(warp) => {
                save_current_store();
                play_success_sound(&player);
                let perm_info = warp
                    .permission
                    .as_deref()
                    .map(|p| format!(" §8(право: §c{p}§8)"))
                    .unwrap_or_default();
                player.send_system_message(
                    legacy(&format!(
                        "§6§l✦ KRONOS WARPS §8» §aСерверный варп §f{} §aсохранён в §e{} ({:.1}, {:.1}, {:.1}){}",
                        warp.name,
                        warp.location.world,
                        warp.location.x,
                        warp.location.y,
                        warp.location.z,
                        perm_info
                    )),
                    false,
                );
            }
            Err(err) => {
                play_error_sound(&player);
                player.send_system_message(
                    legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)),
                    false,
                );
            }
        }
        Ok(0)
    }
}

/// Если последний аргумент содержит `:` (например, `kronos-warps:vip`), считаем его правом доступа.
fn parse_desc_and_perm(rest: &[&str]) -> (String, Option<String>) {
    if rest.is_empty() {
        return ("Серверный варп".to_string(), None);
    }
    if let Some(last) = rest.last() {
        if last.contains(':') && !last.starts_with(':') && !last.ends_with(':') {
            let desc = if rest.len() > 1 {
                rest[..rest.len() - 1].join(" ")
            } else {
                "Серверный варп".to_string()
            };
            return (desc, Some((*last).to_string()));
        }
    }
    (rest.join(" "), None)
}
