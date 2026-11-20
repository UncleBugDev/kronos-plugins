use pumpkin_plugin_api::{
    gui::Gui, screens_wit::Screen, wit::pumpkin::plugin::sounds::SoundCategory, Player,
};

use crate::store::MenuPage;
use crate::ui::{fill_kronos_frame, legacy, make_item};

/// Страница памятки по системе чатов (`kronos-chat`).
pub fn open_java_chat_guide(player: &Player) {
    let gui = Gui::new(Screen::Generic9x6, legacy("§8💬 §e§lЧАТЫ И ОБЩЕНИЕ KRONOS §8✦"));
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "writable_book",
            "§e§l💬 СИСТЕМА ЧАТОВ KRONOS",
            &["§7Анти-спам, защита от инъекций § и облачка над головой."],
        ),
    );

    gui.set_item(
        20,
        make_item(
            "oak_sign",
            "§f§l[Л] Локальный чат",
            &[
                "§7Работает по умолчанию (без спецсимволов).",
                "§7• Радиус слышимости: §e100 блоков §7(в том же мире).",
                "§7• Над головой появляется облачко §e«Текст» §7на 4.5с.",
                "§7• Если рядом никого нет — подсказка в чате.",
            ],
        ),
    );

    gui.set_item(
        22,
        make_item(
            "bell",
            "§6§l[Г] Глобальный чат (!)",
            &[
                "§7Поставьте §e! §7в начале сообщения: §f!Привет всем",
                "§7• Слышат все игроки на сервере.",
                "§7• Кулдаун: §e3 секунды§7.",
            ],
        ),
    );

    gui.set_item(
        24,
        make_item(
            "gold_ingot",
            "§e§l[ТОРГ] Торговый чат ($ / /trade)",
            &[
                "§7Поставьте §e$ §7в начале или используйте §e/trade <текст>",
                "§7• Слышат все игроки со звуком монетки.",
                "§7• Кулдаун против флуда: §e30 секунд§7.",
            ],
        ),
    );

    gui.set_item(
        30,
        make_item(
            "feather",
            "§d§lЛичные сообщения (/msg, /r)",
            &[
                "§7• §d/msg <ник> <текст> §7(/tell, /w) — написать в ЛС",
                "§7• §d/r <текст> §7— быстрый ответ собеседнику",
                "§7• Звук колокольчика при получении сообщения.",
            ],
        ),
    );

    gui.set_item(
        32,
        make_item(
            "barrier",
            "§c§lЧёрный список (/ignore)",
            &[
                "§7• §c/ignore <ник> §7— скрыть сообщения и ЛС игрока",
                "§7• §c/ignore §7— показать ваш чёрный список",
                "",
                "§e▸ Нажмите, чтобы вывести памятку в чат",
            ],
        ),
    );

    gui.set_item(49, make_item("arrow", "§e§l« Назад в главное меню", &["§7Вернуться в /menu"]));
    player.open_gui(gui);
}

/// Страница экономики и рынка (`kronos-economy`).
pub fn open_java_economy(player: &Player, balance: &str) {
    let gui = Gui::new(Screen::Generic9x6, legacy("§8💰 §a§lЭКОНОМИКА И РЫНОК KRONOS §8✦"));
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "emerald_block",
            &format!("§a§l💰 ВАШ БАЛАНС: §e§l{balance}"),
            &["§7Валютная система сервера KRONOS (kronos-economy)."],
        ),
    );

    gui.set_item(
        20,
        make_item(
            "emerald",
            "§e§lБаланс (/bal)",
            &[
                &format!("§7Текущий баланс: §e{balance}"),
                "",
                "§a▸ Нажмите, чтобы открыть /bal",
            ],
        ),
    );

    gui.set_item(
        22,
        make_item(
            "gold_ingot",
            "§a§lПеревод монет (/pay)",
            &[
                "§7Команда: §e/pay <ник> <сумма>",
                "§7Безопасные переводы без потерь и дюпа.",
                "",
                "§a▸ Нажмите, чтобы подставить /pay",
            ],
        ),
    );

    gui.set_item(
        24,
        make_item(
            "nether_star",
            "§6§lТоп-10 богачей (/baltop)",
            &[
                "§7Рейтинг самых богатых игроков сервера",
                "§7с hover-статистикой и долей экономики.",
                "",
                "§6▸ Нажмите, чтобы открыть /baltop",
            ],
        ),
    );

    gui.set_item(
        31,
        make_item(
            "chest",
            "§e§lТорговый чат ($ / /trade)",
            &[
                "§7Размещайте объявления о покупке/продаже:",
                "§e$Продам алмазы §7или §e/trade <текст>",
                "",
                "§e▸ Нажмите, чтобы подставить /trade",
            ],
        ),
    );

    gui.set_item(49, make_item("arrow", "§e§l« Назад в главное меню", &["§7Вернуться в /menu"]));
    player.open_gui(gui);
}

pub fn handle_chat_guide_click(player: &Player, slot: i16) -> Option<MenuPage> {
    if slot == 49 {
        player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
        return Some(MenuPage::Main);
    }
    if matches!(slot, 20 | 22 | 24 | 30 | 32) {
        send_chat_guide_to_chat(player);
    }
    None
}

pub fn handle_economy_click(player: &Player, slot: i16, balance: &str) -> Option<MenuPage> {
    match slot {
        49 => {
            player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
            Some(MenuPage::Main)
        }
        20 | 22 | 24 | 31 => {
            send_economy_guide_to_chat(player, balance);
            None
        }
        _ => None,
    }
}

pub fn send_chat_guide_to_chat(player: &Player) {
    player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
    player.send_system_message(legacy("§e§l💬 ПАМЯТКА ПО ЧАТАМ KRONOS"), false);
    player.send_system_message(legacy("  §f• Локал: §7просто пишите в чат (100 блоков + облачко над головой)"), false);
    player.send_system_message(legacy("  §6• Глобал: §e!<текст> §7(весь сервер, КД 3с)"), false);
    player.send_system_message(legacy("  §e• Торг: §e$<текст> §7или §e/trade <текст> §7(КД 30с + звук монетки)"), false);
    let actions = legacy("  §8» ")
        .add_child(legacy("§d§l[/msg] ").click_suggest_command("/msg ").hover_show_text(legacy("§dНаписать личное сообщение")))
        .add_child(legacy("§e§l[/trade] ").click_suggest_command("/trade ").hover_show_text(legacy("§eОтправить торговое объявление")))
        .add_child(legacy("§c§l[/ignore]").click_suggest_command("/ignore ").hover_show_text(legacy("§cДобавить игрока в ЧС")));
    player.send_system_message(actions, false);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
}

pub fn send_economy_guide_to_chat(player: &Player, balance: &str) {
    player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
    player.send_system_message(legacy(&format!("§a§l💰 ЭКОНОМИКА KRONOS §8│ §7Ваш баланс: §e{balance}")), false);
    let actions = legacy("  §8» ")
        .add_child(legacy("§e§l[/bal] ").click_run_command("/bal").hover_show_text(legacy("§eОткрыть карточку баланса")))
        .add_child(legacy("§a§l[/pay] ").click_suggest_command("/pay ").hover_show_text(legacy("§aПеревести монеты: /pay <ник> <сумма>")))
        .add_child(legacy("§6§l[/baltop] ").click_run_command("/baltop").hover_show_text(legacy("§6Открыть Топ-10 богачей")))
        .add_child(legacy("§e§l[/trade]").click_suggest_command("/trade ").hover_show_text(legacy("§eТорговый чат ($ или /trade)")));
    player.send_system_message(actions, false);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
}
