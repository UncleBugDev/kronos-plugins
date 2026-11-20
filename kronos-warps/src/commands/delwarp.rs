use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{extract_args, is_admin_sender, PERM_ADMIN};
use crate::menu::legacy;
use crate::sounds::{play_error_sound, play_success_sound};
use crate::storage::{save_current_store, with_state};

pub struct DelWarpCmd;

impl CommandHandler for DelWarpCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if !is_admin_sender(&sender) {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(&format!(
                "§6§l✦ KRONOS WARPS §8» §cНедостаточно прав (требуется {PERM_ADMIN})."
            )));
            return Ok(0);
        }

        let raw = extract_args(&args);
        let Some(name) = raw.split_whitespace().next() else {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(
                "§6§l✦ KRONOS WARPS §8» §eИспользование: §f/delwarp <имя>",
            ));
            return Ok(0);
        };

        let result = with_state(|st| st.store.delete_server_warp(name));
        match result {
            Ok(removed) => {
                save_current_store();
                if let Some(p) = sender.as_player() {
                    play_success_sound(&p);
                }
                sender.send_message(legacy(&format!(
                    "§6§l✦ KRONOS WARPS §8» §aСерверный варп §f{} §aудалён.",
                    removed.name
                )));
            }
            Err(err) => {
                if let Some(p) = sender.as_player() {
                    play_error_sound(&p);
                }
                sender.send_error(legacy(&format!("§6§l✦ KRONOS WARPS §8» §c✗ {}", err)));
            }
        }
        Ok(0)
    }
}
