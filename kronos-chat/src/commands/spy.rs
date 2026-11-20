use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{legacy, PERM_SOCIALSPY};
use crate::sounds::{play_error_sound, play_success_sound};
use crate::storage::{save_current_store, with_state};

pub struct SocialSpyCmd;

impl CommandHandler for SocialSpyCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /socialspy доступна только игрокам."));
            return Ok(0);
        };
        if !player.has_permission(PERM_SOCIALSPY) {
            play_error_sound(&player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS CHAT §8» §cНедостаточно прав (kronos-chat:socialspy)."),
                false,
            );
            return Ok(0);
        }

        let name = player.get_name();
        let enabled = with_state(|st| st.store.toggle_social_spy(&name));
        save_current_store();
        play_success_sound(&player);

        let status = if enabled { "§aвключён" } else { "§cвыключен" };
        player.send_system_message(
            legacy(&format!("§6§l✦ KRONOS CHAT §8» §7Режим SocialSpy {status}§7.")),
            false,
        );
        Ok(0)
    }
}
