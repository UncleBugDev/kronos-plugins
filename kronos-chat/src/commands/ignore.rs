use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{extract_args, legacy};
use crate::models::normalize_key;
use crate::sounds::{play_error_sound, play_success_sound};
use crate::storage::{save_current_store, with_state};

pub struct IgnoreCmd;

impl CommandHandler for IgnoreCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy("§cКоманда /ignore доступна только игрокам."));
            return Ok(0);
        };

        let raw = extract_args(&args);
        let Some(target) = raw.split_whitespace().next() else {
            let p_key = normalize_key(&player.get_name());
            let list: Vec<String> = with_state(|st| {
                st.store
                    .ignored
                    .get(&p_key)
                    .map(|set| set.iter().cloned().collect())
                    .unwrap_or_default()
            });
            if list.is_empty() {
                player.send_system_message(
                    legacy("§6§l✦ KRONOS CHAT §8» §7Ваш чёрный список пуст. Использование: §f/ignore <ник>"),
                    false,
                );
            } else {
                player.send_system_message(
                    legacy(&format!("§6§l✦ KRONOS CHAT §8» §7В чёрном списке: §e{}", list.join(", "))),
                    false,
                );
            }
            return Ok(0);
        };

        let owner = player.get_name();
        if normalize_key(&owner) == normalize_key(target) {
            play_error_sound(&player);
            player.send_system_message(legacy("§6§l✦ KRONOS CHAT §8» §cНельзя добавить в игнор самого себя."), false);
            return Ok(0);
        }

        let added = with_state(|st| st.store.toggle_ignore(&owner, target));
        save_current_store();
        play_success_sound(&player);

        let msg = if added {
            format!("§6§l✦ KRONOS CHAT §8» §eИгрок §f{target} §eдобавлен в чёрный список.")
        } else {
            format!("§6§l✦ KRONOS CHAT §8» §aИгрок §f{target} §aудалён из чёрного списка.")
        };
        player.send_system_message(legacy(&msg), false);
        Ok(0)
    }
}
