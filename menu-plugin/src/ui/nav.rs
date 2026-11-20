use pumpkin_plugin_api::{
    gui::Gui, screens_wit::Screen, wit::pumpkin::plugin::sounds::SoundCategory, Player,
};

use crate::store::MenuPage;
use crate::ui::{fill_kronos_frame, handle_safe_spawn_teleport, legacy, make_item};

/// Страница телепортов, варпов и домов (`kronos-essentials` + `kronos-warps`).
pub fn open_java_nav(player: &Player) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8⚡ §b§lТЕЛЕПОРТАЦИЯ, ВАРПЫ И ДОМА §8✦"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "ender_pearl",
            "§b§l⚡ НАВИГАТОР KRONOS",
            &["§7Выберите точку телепортации или откройте каталог."],
        ),
    );

    // Слот 20: /spawn
    gui.set_item(
        20,
        make_item(
            "beacon",
            "§b§l⚡ Спавн (/spawn)",
            &[
                "§7Мгновенный безопасный возврат на спавн мира.",
                "",
                "§b▸ Нажмите для мгновенной телепортации",
            ],
        ),
    );

    // Слот 22: /rtp
    gui.set_item(
        22,
        make_item(
            "firework_rocket",
            "§a§l🌍 Случайный телепорт (/rtp)",
            &[
                "§7Безопасный телепорт в случайную точку мира",
                "§7(плагин §ekronos-essentials§7, защита от лавы/воды).",
                "",
                "§a▸ Нажмите, чтобы запустить /rtp",
            ],
        ),
    );

    // Слот 24: /warps
    gui.set_item(
        24,
        make_item(
            "lodestone",
            "§6§l✦ Каталог варпов (/warps)",
            &[
                "§7Серверные и популярные пользовательские варпы",
                "§7(плагин §ekronos-warps§7: /warp, /pwarp create).",
                "",
                "§6▸ Нажмите, чтобы открыть каталог /warps",
            ],
        ),
    );

    // Слот 31: /homes & /sharehome
    gui.set_item(
        31,
        make_item(
            "red_bed",
            "§e§l⌂ Дома и общие дома (/homes)",
            &[
                "§7Управление личными и общими точками дома:",
                "  §f• §e/sethome <имя> §7и §e/home <имя>",
                "  §f• §a/sharehome <имя> <ник> §7— доступ для друзей",
                "",
                "§e▸ Нажмите, чтобы вывести кнопки в чат",
            ],
        ),
    );

    gui.set_item(49, make_item("arrow", "§e§l« Назад в главное меню", &["§7Вернуться в /menu"]));
    player.open_gui(gui);
}

pub fn handle_nav_click(player: &Player, slot: i16) -> Option<MenuPage> {
    match slot {
        20 => {
            handle_safe_spawn_teleport(player);
            None
        }
        22 => {
            send_quick_command_hint(
                player,
                "§a§l🌍 Безопасный RTP",
                "/rtp",
                "§aНажмите, чтобы выполнить /rtp",
            );
            None
        }
        24 => {
            send_quick_command_hint(
                player,
                "§6§l✦ Каталог варпов KRONOS",
                "/warps",
                "§6Нажмите, чтобы открыть /warps",
            );
            None
        }
        31 => {
            player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
            let row = legacy("§6§l✦ ДОМА §8» ")
                .add_child(legacy("§e§l[/homes] ").click_run_command("/homes").hover_show_text(legacy("§eОткрыть список ваших домов")))
                .add_child(legacy("§a§l[/sethome] ").click_suggest_command("/sethome ").hover_show_text(legacy("§aСоздать точку дома")))
                .add_child(legacy("§b§l[/sharehome]").click_suggest_command("/sharehome ").hover_show_text(legacy("§bПоделиться домом с другом")));
            player.send_system_message(row, false);
            None
        }
        49 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
            Some(MenuPage::Main)
        }
        _ => None,
    }
}

fn send_quick_command_hint(player: &Player, title: &str, cmd: &str, hover: &str) {
    player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
    let msg = legacy(&format!("{title} §8» "))
        .add_child(legacy(&format!("§a§l[Выполнить {cmd}]")).click_run_command(cmd).hover_show_text(legacy(hover)));
    player.send_system_message(msg, false);
}
