use pumpkin_plugin_api::{
    command::{CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Server,
};

use crate::commands::{extract_args, is_admin_sender, PERM_STAFF};
use crate::sounds::{play_chime_sound, play_error_sound};
use crate::storage::with_state;
use crate::tab_chat::{format_staff_chat_line, legacy};

pub struct StaffChatCmd;

impl CommandHandler for StaffChatCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let message = extract_args(&args).trim().to_string();
        if message.is_empty() {
            sender.send_error(legacy("§c§l[SC] §eИспользование: §f/sc <сообщение>"));
            return Ok(0);
        }

        let (allowed, formatted) = if let Some(player) = sender.as_player() {
            let name = player.get_name();
            let native_staff = player.has_permission(PERM_STAFF) || is_admin_sender(&sender);
            with_state(|st| {
                let user = st.users.get_or_default(&name);
                let ok = native_staff
                    || st.tree.is_staff(&user)
                    || st.tree.has_permission(&user, "kronos-ranks:sc");
                (ok, format_staff_chat_line(&st.tree, &user, &message))
            })
        } else {
            (true, format!("§c§l[SC] §4§l[КОНСОЛЬ] §8» §e{message}"))
        };

        if !allowed {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy("§c§l[SC] §cСтафф-чат доступен с должности Хелпер."));
            return Ok(0);
        }

        let online = server.get_all_players();
        let staff_names: Vec<String> = with_state(|st| {
            online
                .iter()
                .filter(|p| {
                    let u = st.users.get_or_default(&p.get_name());
                    st.tree.is_staff(&u) || st.tree.has_permission(&u, "kronos-ranks:sc")
                })
                .map(|p| p.get_name())
                .collect()
        });

        for p in &online {
            if staff_names.iter().any(|n| n == &p.get_name()) || p.has_permission(PERM_STAFF) {
                p.send_system_message(legacy(&formatted), false);
                play_chime_sound(p);
            }
        }
        if sender.is_console() {
            sender.send_message(legacy(&formatted));
        }
        Ok(0)
    }
}
