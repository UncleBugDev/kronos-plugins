use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{deliver_channel_message, extract_args, legacy};
use crate::models::Channel;
use crate::sounds::play_error_sound;

pub struct TradeCmd;

impl CommandHandler for TradeCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /trade доступна только игрокам."));
            return Ok(0);
        };

        let body = extract_args(&args).trim().to_string();
        if body.is_empty() {
            play_error_sound(&player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS CHAT §8» §eИспользование: §f/trade <объявление> §7(или начните сообщение с §e$§7)"),
                false,
            );
            return Ok(0);
        }

        deliver_channel_message(&player, &server, Channel::Trade, &body);
        Ok(0)
    }
}
