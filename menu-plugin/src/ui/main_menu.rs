use pumpkin_plugin_api::{gui::Gui, screens_wit::Screen, Player};

use crate::store::PlayerPrefs;
use crate::ui::{
    bool_badge, dimension_label, fill_kronos_frame, legacy, make_item, ping_colored,
    player_has_menu_compass,
};

/// Главное сундучное меню 54 слота (обновлённые слоты 20–34 под все 5 новых плагинов KRONOS).
pub fn open_java_main(player: &Player, prefs: &PlayerPrefs, balance: &str) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8✦ §6§lМЕНЮ СЕРВЕРА KRONOS §8✦"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    let name = player.get_name();
    let ping = ping_colored(player.get_ping());
    let world = player.get_world();
    let dim = dimension_label(&world.get_dimension());
    let pos = player.get_position();
    let xp_lvl = player.get_experience_level();

    // Слот 4: Профиль игрока
    gui.set_item(
        4,
        make_item(
            "nether_star",
            &format!("§6§l✦ ПРОФИЛЬ: §f{name}"),
            &[
                &format!("§7Баланс: §e{balance} §8│ §7Пинг: {ping}"),
                &format!("§7Мир: {dim} §8│ §7Опыт: §a{xp_lvl} ур."),
                &format!(
                    "§7Координаты: §f{} {} {}",
                    pos.0.floor() as i32,
                    pos.1.floor() as i32,
                    pos.2.floor() as i32
                ),
                "",
                &format!(
                    "§7Погода: {} §8│ §7Время: {}",
                    prefs.weather.colored_label(),
                    prefs.time.colored_label()
                ),
                &format!(
                    "§7HUD: {} §8│ §7Уединение: {}",
                    bool_badge(prefs.hud_compass),
                    bool_badge(prefs.hide_players)
                ),
                "",
                "§e▸ Нажмите, чтобы обновить профиль",
            ],
        ),
    );

    // Слот 20: Телепортация, Варпы и Дома (kronos-essentials + kronos-warps)
    gui.set_item(
        20,
        make_item(
            "ender_pearl",
            "§b§l⚡ Телепортация и Дома",
            &[
                "§7Быстрые переходы по миру KRONOS:",
                "  §f• §b/spawn §7— мгновенный возврат на спавн",
                "  §f• §a/rtp §7— безопасная случайная телепортация",
                "  §f• §6/warps §7— серверные и личные варпы",
                "  §f• §e/homes §7и §e/sharehome §7— ваши дома",
                "",
                "§b▸ Нажмите, чтобы открыть навигацию",
            ],
        ),
    );

    // Слот 22: Экономика и рынок (kronos-economy)
    gui.set_item(
        22,
        make_item(
            "emerald",
            "§a§l💰 Экономика и Рынок",
            &[
                &format!("§7Ваш текущий баланс: §e§l{balance}"),
                "",
                "  §f• §e/pay <ник> <сумма> §7— перевести монеты",
                "  §f• §6/baltop §7— топ-10 богачей сервера",
                "  §f• §e$ §7или §e/trade §7— торговый чат",
                "",
                "§a▸ Нажмите, чтобы открыть панель экономики",
            ],
        ),
    );

    // Слот 24: Система чатов и общение (kronos-chat)
    gui.set_item(
        24,
        make_item(
            "writable_book",
            "§e§l💬 Чаты и Общение",
            &[
                "§7Каналы общения сервера KRONOS:",
                "  §f• §fЛокал §7— 100 блоков + облачка над головой",
                "  §f• §6Глобал §7— начните сообщение с §e!",
                "  §f• §eТорг §7— символ §e$ §7или §e/trade",
                "  §f• §dЛС §7— §d/msg§7, §d/r§7, ЧС §c/ignore",
                "",
                "§e▸ Нажмите, чтобы открыть памятку чатов",
            ],
        ),
    );

    // Слот 29: Ранги и донат-возможности (kronos-ranks — 12 русских рангов)
    gui.set_item(
        29,
        make_item(
            "golden_helmet",
            "§d§l👑 Ранги и Привилегии (12)",
            &[
                "§7Актуальная линейка из 12 русских рангов:",
                "§7Игрок • §aVIP • §bПремиум • §dКреатив",
                "§eГерой • §6Титан • §cДракон • §4§lСПОНСОР",
                "§9Хелпер • §3Модератор • §cАДМИН • §4СОЗДАТЕЛЬ",
                "",
                "§d▸ Нажмите, чтобы открыть витрину (/ranks)",
            ],
        ),
    );

    // Слот 31: Личные настройки игрока
    gui.set_item(
        31,
        make_item(
            "comparator",
            "§e§l⚙ Настройки игрока",
            &[
                "§7Персональный комфорт на сервере:",
                "  §f• §eЛичная погода и время суток",
                "  §f• §bЖивой HUD-компас над хотбаром",
                "  §f• §dУпоминания, §aУединение§7, §6Авто-компас",
                "",
                "§a▸ Нажмите, чтобы открыть (/settings)",
            ],
        ),
    );

    // Слот 33: Правила и безопасность аккаунта
    gui.set_item(
        33,
        make_item(
            "totem_of_undying",
            "§6§l📜 Правила и Безопасность",
            &[
                "§7Кодекс честной игры и защита аккаунта:",
                "  §f• §6Правила выживания (/rules)",
                "  §f• §bКабинет §e/auth§7, §e/changepass§7, §c/logout",
                "",
                "§6▸ Нажмите, чтобы открыть раздел",
            ],
        ),
    );

    // Слот 48: О сервере
    gui.set_item(
        48,
        make_item(
            "lantern",
            "§b§lℹ О сервере KRONOS",
            &[
                "§7Java 1.20.1–26.3 + Bedrock 19132",
                "§7Ядро Rust Pumpkin 26.3",
                "",
                "§b▸ Нажмите для справки",
            ],
        ),
    );

    // Слот 49: Получить компас меню
    let compass_status = if player_has_menu_compass(player) {
        "§a✔ Уже находится у вас в инвентаре"
    } else {
        "§e▸ Нажмите, чтобы получить в свободный слот"
    };
    gui.set_item(
        49,
        make_item(
            "compass",
            "§6§l🧭 Получить компас меню",
            &[
                "§7Фирменный предмет быстрого доступа",
                "§7к меню KRONOS по нажатию ПКМ.",
                "",
                compass_status,
            ],
        ),
    );

    player.open_gui(gui);
}
