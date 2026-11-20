use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{extract_args, legacy, send_private_message};
use crate::sounds::play_error_sound;

pub struct MsgCmd;

impl CommandHandler for MsgCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /msg доступна только игрокам."));
            return Ok(0);
        };

        let raw = extract_args(&args);
        let mut split = raw.trim().splitn(2, char::is_whitespace);
        let target = split.next().unwrap_or("").trim();
        let body = split.next().unwrap_or("").trim();

        if target.is_empty() || body.is_empty() {
            play_error_sound(&player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS CHAT §8» §eИспользование: §f/msg <ник> <текст>"),
                false,
            );
            return Ok(0);
        }

        send_private_message(&player, &server, target, body);
        Ok(0)
    }
}
