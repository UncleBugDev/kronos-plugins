use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{extract_args, legacy, send_private_message};
use crate::models::normalize_key;
use crate::sounds::play_error_sound;
use crate::storage::with_state;

pub struct ReplyCmd;

impl CommandHandler for ReplyCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /r доступна только игрокам."));
            return Ok(0);
        };

        let body = extract_args(&args).trim().to_string();
        if body.is_empty() {
            play_error_sound(&player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS CHAT §8» §eИспользование: §f/r <текст>"),
                false,
            );
            return Ok(0);
        }

        let key = normalize_key(&player.get_name());
        let partner = with_state(|st| st.last_pm_partner.get(&key).cloned());
        let Some(target) = partner else {
            play_error_sound(&player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS CHAT §8» §cВам ещё никто не писал личных сообщений."),
                false,
            );
            return Ok(0);
        };

        send_private_message(&player, &server, &target, &body);
        Ok(0)
    }
}
