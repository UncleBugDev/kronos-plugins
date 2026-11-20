use pumpkin_plugin_api::{
    command::{Command, CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    wit::pumpkin::plugin::{common::Hand, gui::Gui, screens::Screen},
    Context, Player, Server,
};

use crate::commands::PERM_PLAYER;
use crate::sounds::{play_error_sound, play_success_sound};
use crate::storage::with_state;
use crate::tab_chat::legacy;

fn has_rank_perm(player: &Player, node: &str) -> bool {
    if player.has_permission(node) {
        return true;
    }
    let name = player.get_name();
    with_state(|st| {
        let user = st.users.get_or_default(&name);
        st.tree.has_permission(&user, node)
    })
}

fn require_donator_perm(sender: &CommandSender, node: &str, rank_hint: &str) -> Option<Player> {
    let Some(player) = sender.as_player() else {
        sender.send_error(legacy("§cЭта команда доступна только игрокам."));
        return None;
    };
    if !has_rank_perm(&player, node) {
        play_error_sound(&player);
        player.send_system_message(
            legacy(&format!(
                "§6§l✦ KRONOS RANKS §8» §cКоманда доступна с ранга §e{rank_hint} §8({node})."
            )),
            false,
        );
        return None;
    }
    Some(player)
}

struct FlyCmd;
impl CommandHandler for FlyCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(p) = require_donator_perm(&sender, "kronos-ranks:cmd.fly", "Креатив") else {
            return Ok(0);
        };
        let next = !p.get_abilities().allow_flying;
        p.set_allow_flight(next);
        if !next {
            p.set_flying(false);
        }
        play_success_sound(&p);
        let state = if next { "§aвключён" } else { "§cвыключен" };
        p.send_system_message(legacy(&format!("§6§l✦ KRONOS §8» §7Режим полёта {state}§7.")), false);
        Ok(0)
    }
}

struct FeedCmd;
impl CommandHandler for FeedCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(p) = require_donator_perm(&sender, "kronos-ranks:cmd.feed", "Премиум") else {
            return Ok(0);
        };
        p.set_food_level(20);
        p.set_saturation(20.0);
        p.set_exhaustion(0.0);
        play_success_sound(&p);
        p.send_system_message(legacy("§6§l✦ KRONOS §8» §aВы полностью утолили голод!"), false);
        Ok(0)
    }
}

struct HealCmd;
impl CommandHandler for HealCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(p) = require_donator_perm(&sender, "kronos-ranks:cmd.heal", "Герой") else {
            return Ok(0);
        };
        let max_hp = p.get_max_health();
        p.set_health(max_hp);
        p.set_food_level(20);
        p.set_saturation(20.0);
        play_success_sound(&p);
        p.send_system_message(legacy("§6§l✦ KRONOS §8» §aЗдоровье и голод полностью восстановлены!"), false);
        Ok(0)
    }
}

struct HatCmd;
impl CommandHandler for HatCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(p) = require_donator_perm(&sender, "kronos-ranks:cmd.hat", "Креатив") else {
            return Ok(0);
        };
        let inv = p.get_inventory();
        let hand_item = inv.get_item_in_hand(Hand::Right);
        if hand_item.is_none() {
            play_error_sound(&p);
            p.send_system_message(legacy("§6§l✦ KRONOS §8» §cВозьмите предмет в руку, чтобы надеть его на голову."), false);
            return Ok(0);
        }
        let old_helmet = inv.get_helmet();
        inv.set_helmet(hand_item);
        inv.set_item_in_hand(Hand::Right, old_helmet);
        play_success_sound(&p);
        p.send_system_message(legacy("§6§l✦ KRONOS §8» §aПредмет надет на голову!"), false);
        Ok(0)
    }
}

struct WorkbenchCmd;
impl CommandHandler for WorkbenchCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(p) = require_donator_perm(&sender, "kronos-ranks:cmd.workbench", "Премиум") else {
            return Ok(0);
        };
        let gui = Gui::new(Screen::Crafting, legacy("§6Верстак KRONOS"));
        gui.set_allow_grab_items(true);
        gui.set_allow_put_items(true);
        p.open_gui(gui);
        play_success_sound(&p);
        Ok(0)
    }
}

struct EnderChestCmd;
impl CommandHandler for EnderChestCmd {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(p) = require_donator_perm(&sender, "kronos-ranks:cmd.enderchest", "Герой") else {
            return Ok(0);
        };
        p.open_ender_chest();
        play_success_sound(&p);
        Ok(0)
    }
}

pub fn register_donator_commands(context: &Context) {
    context.register_command(Command::new(&["fly".into()], "Включить/выключить полёт (/fly)").execute(FlyCmd), PERM_PLAYER);
    context.register_command(Command::new(&["feed".into()], "Восстановить голод (/feed)").execute(FeedCmd), PERM_PLAYER);
    context.register_command(Command::new(&["heal".into()], "Восстановить здоровье и голод (/heal)").execute(HealCmd), PERM_PLAYER);
    context.register_command(Command::new(&["hat".into()], "Надеть предмет из руки на голову (/hat)").execute(HatCmd), PERM_PLAYER);
    context.register_command(Command::new(&["workbench".into(), "craft".into(), "wb".into()], "Открыть переносной верстак").execute(WorkbenchCmd), PERM_PLAYER);
    context.register_command(Command::new(&["enderchest".into(), "ec".into()], "Открыть эндер-сундук").execute(EnderChestCmd), PERM_PLAYER);
}
