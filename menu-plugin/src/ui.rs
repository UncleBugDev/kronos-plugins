use pumpkin_plugin_api::{
    ItemStackExt, Player,
    forms::{CustomFormBuilder, SimpleFormBuilder},
    gui::Gui,
    item_stack::ItemStack,
    particles::Particle,
    persistent_data::byte_tree,
    player::PlayerWeather,
    screens_wit::Screen,
    server::Server,
    text::TextComponent,
    uuid::Uuid,
    wit::pumpkin::plugin::sounds::SoundCategory,
};

use crate::store::{BedrockFormKind, MenuPage, PlayerPrefs, TimePref, WeatherPref};

pub const MENU_COMPASS_NAMESPACE: &str = "kronos";
pub const MENU_COMPASS_KEY: &str = "menu_compass";
pub const MENU_COMPASS_MARKER: &str = "Меню KRONOS";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompassGiveOutcome {
    Locked,
    AlreadyPresent,
    PlacedInSlot(u8),
    InventoryFull,
}

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

pub fn make_item(id: &str, name: &str, lore: &[&str]) -> ItemStack {
    let stack = ItemStack::of(id, 1);
    stack.set_custom_name(Some(legacy(name).italic(false)));
    if !lore.is_empty() {
        let lore_components: Vec<TextComponent> =
            lore.iter().map(|line| legacy(line).italic(false)).collect();
        stack.set_lore(lore_components);
    }
    stack
}

pub fn create_menu_compass() -> ItemStack {
    let stack = make_item(
        "compass",
        "§6§l✦ Меню KRONOS §7(ПКМ)",
        &[
            "§7Главный навигатор сервера §6§lKRONOS",
            "",
            "§e▸ Нажмите ПКМ в руке§7, чтобы открыть:",
            "  §f• §eНастройки погоды, времени и HUD",
            "  §f• §6Правила сервера и привилегии",
            "  §f• §bТелепорт на Спавн и Кабинет защиты",
            "",
            "§8Команды: /menu (/m) • /settings • /rules",
        ],
    );
    stack.set_custom_data(MENU_COMPASS_NAMESPACE, MENU_COMPASS_KEY, &byte_tree(1));
    stack
}

pub fn is_menu_compass(stack: &ItemStack) -> bool {
    if !stack.matches_item("compass") {
        return false;
    }
    if stack.has_custom_data(MENU_COMPASS_NAMESPACE, MENU_COMPASS_KEY) {
        return true;
    }
    if let Some(name_tc) = stack.get_custom_name()
        && (name_tc.get_text().contains(MENU_COMPASS_MARKER)
            || name_tc.to_json().contains(MENU_COMPASS_MARKER))
    {
        return true;
    }
    false
}

pub fn player_has_menu_compass(player: &Player) -> bool {
    for slot in 0..36u8 {
        if player
            .get_inventory_item(slot)
            .is_some_and(|stack| is_menu_compass(&stack))
        {
            return true;
        }
    }
    player
        .get_inventory()
        .get_off_hand()
        .is_some_and(|offhand| is_menu_compass(&offhand))
}

pub fn give_menu_compass_non_destructive(player: &Player) -> CompassGiveOutcome {
    if player.is_movement_locked() {
        return CompassGiveOutcome::Locked;
    }
    if player_has_menu_compass(player) {
        return CompassGiveOutcome::AlreadyPresent;
    }

    let preferred_hotbar: [u8; 9] = [8, 7, 6, 5, 4, 3, 2, 1, 0];
    let mut target_slot: Option<u8> = None;

    for slot in preferred_hotbar {
        if player.get_inventory_item(slot).is_none() {
            target_slot = Some(slot);
            break;
        }
    }

    if target_slot.is_none() {
        for slot in 9..36u8 {
            if player.get_inventory_item(slot).is_none() {
                target_slot = Some(slot);
                break;
            }
        }
    }

    if let Some(slot) = target_slot {
        let compass = create_menu_compass();
        player.set_inventory_item(slot, Some(compass));
        CompassGiveOutcome::PlacedInSlot(slot)
    } else {
        CompassGiveOutcome::InventoryFull
    }
}

pub fn apply_weather_pref(player: &Player, weather: WeatherPref) {
    match weather {
        WeatherPref::Server => player.reset_player_weather(),
        WeatherPref::Clear => player.set_player_weather(PlayerWeather::Clear),
        WeatherPref::Downfall => player.set_player_weather(PlayerWeather::Downfall),
    }
}

pub fn apply_time_pref(player: &Player, time: TimePref) {
    if let Some(ticks) = time.ticks() {
        player.set_player_time(ticks, false);
    } else {
        player.reset_player_time();
    }
}

pub const fn same_uuid(a: Uuid, b: Uuid) -> bool {
    a.high == b.high && a.low == b.low
}

pub fn apply_visibility_pref(server: &Server, viewer: &Player, hide_players: bool) {
    let viewer_id = viewer.get_id();
    for other in server.get_all_players() {
        if same_uuid(other.get_id(), viewer_id) {
            continue;
        }
        if hide_players {
            viewer.hide_player(other);
        } else {
            viewer.show_player(other);
        }
    }
}

pub fn cardinal_from_yaw(yaw: f32) -> &'static str {
    // Minecraft yaw: 0 = South (+Z), 90 = West (-X), 180 = North (-Z), 270 = East (+X)
    let deg = ((yaw % 360.0) + 360.0) % 360.0;
    let idx = (((deg + 22.5) % 360.0) / 45.0).floor() as usize % 8;
    const DIRS: [&str; 8] = ["Ю", "ЮЗ", "З", "СЗ", "С", "СВ", "В", "ЮВ"];
    DIRS[idx]
}

pub fn format_world_clock(ticks_of_day: u64) -> String {
    let total_mc_minutes = ((ticks_of_day % 24000) * 60) / 1000;
    let total_minutes = (360 + total_mc_minutes) % (24 * 60);
    let hours = total_minutes / 60;
    let mins = total_minutes % 60;
    format!("{hours:02}:{mins:02}")
}

pub fn dimension_label(dim: &str) -> &'static str {
    match dim {
        "minecraft:the_nether" => "§c🔥 Нижний мир",
        "minecraft:the_end" => "§d🌌 Край",
        _ => "§a☀ Верхний мир",
    }
}

pub fn ping_colored(ping: u32) -> String {
    if ping < 60 {
        format!("§a{ping}мс")
    } else if ping < 150 {
        format!("§e{ping}мс")
    } else {
        format!("§c{ping}мс")
    }
}

pub fn show_hud_compass_actionbar(player: &Player) {
    let dir = cardinal_from_yaw(player.get_yaw());
    let pos = player.get_position();
    let bx = pos.0.floor() as i32;
    let by = pos.1.floor() as i32;
    let bz = pos.2.floor() as i32;
    let world = player.get_world();
    let clock = format_world_clock(world.get_time_of_day());
    let ping = ping_colored(player.get_ping());

    player.show_actionbar(legacy(&format!(
        "§6🧭 §e§l{dir} §8│ §7XYZ: §f{bx} {by} {bz} §8│ §7🕒 §e{clock} §8│ §7Пинг: {ping}"
    )));
}

pub fn handle_safe_spawn_teleport(player: &Player) {
    let world = player.get_world();
    let spawn = world.get_spawn_location();
    let mut target_y = f64::from(spawn.pos.y) + 0.5;

    let min_y = world.get_min_y();
    let block_at_spawn = world.get_block_state(spawn.pos);
    if spawn.pos.y <= min_y || block_at_spawn.is_solid {
        let top_y = world.get_top_block_y(spawn.pos.x, spawn.pos.z);
        if top_y >= min_y {
            target_y = f64::from(top_y + 1) + 0.1;
        }
    }

    let target = (
        f64::from(spawn.pos.x) + 0.5,
        target_y,
        f64::from(spawn.pos.z) + 0.5,
    );

    player.teleport(target, Some(spawn.yaw), Some(spawn.pitch), world);
    player.spawn_particles(
        Particle::EndRod,
        (target.0, target.1 + 1.0, target.2),
        26,
        (0.55, 0.65, 0.55),
        0.07,
    );
    player.play_custom_sound(
        "minecraft:entity.enderman.teleport",
        SoundCategory::Players,
        0.85,
        1.1,
    );
    player.show_actionbar(legacy("§b⚡ Телепортация на точку спавна выполнена!"));
}

pub fn send_security_cabinet_guide(player: &Player) {
    player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
    player.send_system_message(
        legacy("§6§l✦ KRONOS SECURITY §8• §bКабинет безопасности аккаунта"),
        false,
    );
    player.send_system_message(
        legacy("  §7• §aАвто-вход 24 часа: §fпри совпадении IP пароль не запрашивается"),
        false,
    );
    player.send_system_message(
        legacy("  §7• §eСмена пароля: §b/changepass <старый> <новый>"),
        false,
    );
    player.send_system_message(
        legacy("  §7• §cЗавершить сессию: §b/logout §7(сбросит привязку IP)"),
        false,
    );

    let btn_auth = legacy("§b§l[🔐 Открыть /auth] ")
        .click_run_command("/auth")
        .hover_show_text(legacy("§7Открыть личный кабинет авторизации"));
    let btn_cp = legacy("§e§l[🔑 Сменить пароль] ")
        .click_suggest_command("/changepass ")
        .hover_show_text(legacy("§7Подставить команду /changepass <старый> <новый>"));
    let btn_logout = legacy("§c§l[🚪 Выйти /logout]")
        .click_suggest_command("/logout")
        .hover_show_text(legacy("§7Подставить команду /logout"));

    let actions = legacy("  §8» ")
        .add_child(btn_auth)
        .add_child(btn_cp)
        .add_child(btn_logout);
    player.send_system_message(actions, false);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
}

pub fn send_rule_card_to_chat(player: &Player, rule_idx: usize) {
    let rules: [(&str, &[&str]); 6] = [
        (
            "§6§l📜 Правило 1 — Честная игра без читов",
            &[
                "§7• Запрещены чит-клиенты, X-Ray ресурспаки, KillAura, Fly, AutoClicker и эксплойты.",
                "§7• Разрешены клиентские оптимизации (Sodium, Iris, Lithium) и честные миникарты без радара пещер.",
            ],
        ),
        (
            "§e§l📜 Правило 2 — Уважение и культура в чате",
            &[
                "§7• Общайтесь дружелюбно: запрещены оскорбления, травля, спам, капс-флуд и реклама.",
                "§7• Цените уютную атмосферу выживания KRONOS!",
            ],
        ),
        (
            "§c§l📜 Правило 3 — Запрет грифа и воровства",
            &[
                "§7• Чужие постройки, сундуки, механизмы, фермы и питомцы неприкосновенны.",
                "§7• Не портите ландшафт вблизи чужих баз без согласия владельца.",
            ],
        ),
        (
            "§d§l📜 Правило 4 — Запрет лаг-машин и дюпов",
            &[
                "§7• Запрещено клонирование предметов (дюп) и сокрытие критических багов.",
                "§7• Избегайте бесконечных невыключаемых часов, намеренно просаживающих TPS сервера.",
            ],
        ),
        (
            "§b§l📜 Правило 5 — Безопасность вашего аккаунта",
            &[
                "§7• Используйте надёжный пароль в §e/auth §7и никому его не сообщайте.",
                "§7• При игре с чужого устройства завершайте сессию командой §c/logout§7.",
            ],
        ),
        (
            "§a§l📜 Правило 6 — Справедливая модерация",
            &[
                "§7• Команда KRONOS разбирает спорные ситуации объективно и с уважением к игрокам.",
                "§7• По любым вопросам обращайтесь к Модераторам или Администрации на сервере.",
            ],
        ),
    ];

    if let Some((title, lines)) = rules.get(rule_idx) {
        player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.2);
        player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
        player.send_system_message(legacy(title), false);
        for line in *lines {
            player.send_system_message(legacy(line), false);
        }
        player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
    }
}

pub fn send_rank_card_to_chat(player: &Player, rank_idx: usize) {
    let ranks: [(&str, &str); 7] = [
        (
            "§4§l[👑 ВЛАДЕЛЕЦ]",
            "§7Основатель и главный архитектор проекта KRONOS. Полное управление ядром и развитием мира.",
        ),
        (
            "§c§l[⚡ АДМИН]",
            "§7Техническое администрирование, проведение серверных событий и поддержка стабильности.",
        ),
        (
            "§9§l[🛡 МОДЕР]",
            "§7Защита игроков от грифа, поддержание порядка в чате и помощь новичкам.",
        ),
        (
            "§6§l[✦ KRONOS]",
            "§7Высший почётный статус мецената и легенды сервера с золотым знаком отличия в Табе и чате.",
        ),
        (
            "§d§l[★ ЛЕГЕНДА]",
            "§7Титул выдающихся строителей, первопроходцев и ветеранов мира KRONOS.",
        ),
        (
            "§a§l[♦ VIP]",
            "§7Особый изумрудный префикс, выделение в списке игроков и вклад в развитие проекта.",
        ),
        (
            "§7[Игрок]",
            "§7Полноправный участник выживания KRONOS: доступ к /menu, /settings, /kit, /spawn и кроссплею!",
        ),
    ];

    if let Some((badge, desc)) = ranks.get(rank_idx) {
        player.play_custom_sound(
            "minecraft:block.note_block.chime",
            SoundCategory::Ui,
            0.85,
            1.25,
        );
        player.send_system_message(
            legacy(&format!("§6✦ Ранг {badge} §8— {desc}")),
            false,
        );
    }
}

pub fn send_server_info_to_chat(player: &Player) {
    player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
    player.send_system_message(
        legacy("§6§l✦ ИНФОРМАЦИЯ О СЕРВЕРЕ KRONOS ✦"),
        false,
    );
    player.send_system_message(
        legacy("  §7• §fJava Edition: §a1.20.1 – 26.3 §8(мультиверсионный вход)"),
        false,
    );
    player.send_system_message(
        legacy("  §7• §fBedrock Edition: §bпорт 19132 (UDP) §8(Geyser кроссплей)"),
        false,
    );

    let ip_line = legacy("  §7• §fTailscale IP: §e§n100.90.47.115§r §8(нажмите для копирования)")
        .click_copy_to_clipboard("100.90.47.115")
        .hover_show_text(legacy("§aНажмите, чтобы скопировать 100.90.47.115"));
    player.send_system_message(ip_line, false);

    player.send_system_message(
        legacy("  §7• §fКоманды: §6/menu §7(/m), §e/settings§7, §6/rules§7, §b/spawn§7, §a/kit§7, §d/ranks§7, §b/auth"),
        false,
    );
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
}

fn fill_kronos_frame(gui: &Gui) {
    for slot in 0..54u32 {
        let row = slot / 9;
        let col = slot % 9;
        let pane = if matches!(slot, 0 | 8 | 45 | 53) {
            make_item("yellow_stained_glass_pane", "§e✦", &[])
        } else if matches!(slot, 1 | 7 | 9 | 17 | 36 | 44 | 46 | 52) {
            make_item("orange_stained_glass_pane", "§6✦", &[])
        } else if row == 0 || row == 5 || col == 0 || col == 8 {
            make_item("black_stained_glass_pane", "§8", &[])
        } else {
            make_item("gray_stained_glass_pane", "§7", &[])
        };
        gui.set_item(slot, pane);
    }
}

fn bool_badge(v: bool) -> &'static str {
    if v { "§aВкл ✔" } else { "§7Выкл ✖" }
}

pub fn open_java_menu_page(player: &Player, page: MenuPage, prefs: &PlayerPrefs) {
    match page {
        MenuPage::Main => open_java_main(player, prefs),
        MenuPage::Settings => open_java_settings(player, prefs),
        MenuPage::Rules => open_java_rules(player),
        MenuPage::RanksInfo => open_java_ranks(player),
        MenuPage::ServerInfo => open_java_server_info(player),
    }
}

fn open_java_main(player: &Player, prefs: &PlayerPrefs) {
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
    let bx = pos.0.floor() as i32;
    let by = pos.1.floor() as i32;
    let bz = pos.2.floor() as i32;
    let xp_lvl = player.get_experience_level();

    // Slot 4: Player Profile Summary
    gui.set_item(
        4,
        make_item(
            "nether_star",
            &format!("§6§l✦ ПРОФИЛЬ: §f{name}"),
            &[
                &format!("§7Пинг: {ping} §8│ §7Опыт: §a{xp_lvl} ур."),
                &format!("§7Мир: {dim}"),
                &format!("§7Координаты: §f{bx} {by} {bz}"),
                "",
                "§6Активные настройки:",
                &format!("  §7• Погода: {}", prefs.weather.colored_label()),
                &format!("  §7• Время: {}", prefs.time.colored_label()),
                &format!(
                    "  §7• HUD-компас: {} §8│ §7Упоминания: {}",
                    bool_badge(prefs.hud_compass),
                    bool_badge(prefs.chat_mentions)
                ),
                &format!(
                    "  §7• Уединение: {} §8│ §7Авто-компас: {}",
                    bool_badge(prefs.hide_players),
                    bool_badge(prefs.auto_compass)
                ),
                "",
                "§eНажмите, чтобы обновить статистику",
            ],
        ),
    );

    // Slot 20: Player Settings
    gui.set_item(
        20,
        make_item(
            "comparator",
            "§e§l⚙ Настройки игрока",
            &[
                "§7Персональное управление комфортом:",
                "  §f• §eЛичная погода и время суток",
                "  §f• §6Живой HUD-компас над хотбаром",
                "  §f• §bЗвук упоминаний и режим уединения",
                "",
                "§a▸ Нажмите, чтобы открыть (/settings)",
            ],
        ),
    );

    // Slot 22: Server Rules
    gui.set_item(
        22,
        make_item(
            "writable_book",
            "§6§l📜 Правила сервера",
            &[
                "§7Кодекс честного и уютного выживания:",
                "  §f• §7Без читов, грифа, воровства и лаг-машин",
                "  §f• §7Уважение в чате и защита аккаунта",
                "",
                "§e▸ Нажмите, чтобы прочитать (/rules)",
            ],
        ),
    );

    // Slot 24: Spawn Teleport
    gui.set_item(
        24,
        make_item(
            "ender_pearl",
            "§b§l⚡ Телепорт на Спавн",
            &[
                "§7Мгновенное возвращение на безопасную",
                "§7центральную точку спавна вашего мира.",
                "",
                "§b▸ Нажмите для телепортации (/spawn)",
            ],
        ),
    );

    // Slot 29: Ranks & Privileges
    gui.set_item(
        29,
        make_item(
            "golden_helmet",
            "§d§l👑 Ранги и привилегии",
            &[
                "§7Все 7 статусов проекта KRONOS:",
                "§4Владелец §8• §cАдмин §8• §9Модер §8• §6KRONOS",
                "§dЛегенда §8• §aVIP §8• §7Игрок",
                "",
                "§e▸ Нажмите, чтобы открыть витрину рангов",
            ],
        ),
    );

    // Slot 31: Security Cabinet
    gui.set_item(
        31,
        make_item(
            "totem_of_undying",
            "§a§l🔐 Кабинет безопасности",
            &[
                "§7Управление защитой вашего профиля:",
                "  §f• §aАвто-вход 24ч §7при неизменном IP",
                "  §f• §e/auth§7, §e/changepass§7, §c/logout",
                "",
                "§a▸ Нажмите, чтобы вывести памятку в чат",
            ],
        ),
    );

    // Slot 33: Server Info & Commands
    gui.set_item(
        33,
        make_item(
            "lantern",
            "§b§lℹ О сервере и командах",
            &[
                "§7Техническая справка и навигация:",
                "  §f• §aJava 1.20.1–26.3 §8+ §bBedrock 19132",
                "  §f• §eTailscale IP: §f100.90.47.115",
                "  §f• §7Полный справочник всех команд",
                "",
                "§b▸ Нажмите, чтобы открыть справку",
            ],
        ),
    );

    // Slot 49: Get Menu Compass
    let has_compass = player_has_menu_compass(player);
    let compass_status = if has_compass {
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

fn open_java_settings(player: &Player, prefs: &PlayerPrefs) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8⚙ §e§lНАСТРОЙКИ ИГРОКА §8✦"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "comparator",
            "§e§l⚙ ПЕРСОНАЛЬНЫЕ НАСТРОЙКИ",
            &[
                "§7Все переключатели применяются мгновенно",
                "§7и автоматически сохраняются за вашим ником!",
                "",
                "§8Нажимайте на иконку или на индикатор ниже",
            ],
        ),
    );

    // 1. Personal Weather (Slots 19 / 28)
    let (weather_icon, weather_dye) = match prefs.weather {
        WeatherPref::Server => ("sunflower", "gray_dye"),
        WeatherPref::Clear => ("sunflower", "yellow_dye"),
        WeatherPref::Downfall => ("water_bucket", "light_blue_dye"),
    };
    let weather_lore = [
        &format!("§7Текущий режим: {}", prefs.weather.colored_label()),
        "",
        "§7Визуальная погода только для вашего клиента",
        "§7(не влияет на грозы и фермы других игроков).",
        "",
        "§e▸ Нажмите для переключения режима",
    ];
    gui.set_item(
        19,
        make_item(weather_icon, "§e§l🌤 Личная погода", &weather_lore),
    );
    gui.set_item(
        28,
        make_item(
            weather_dye,
            &format!("§7Погода: {}", prefs.weather.colored_label()),
            &["§e▸ Нажмите, чтобы переключить"],
        ),
    );

    // 2. Personal Time (Slots 20 / 29)
    let time_dye = match prefs.time {
        TimePref::Server => "gray_dye",
        TimePref::Day => "yellow_dye",
        TimePref::Sunset => "orange_dye",
        TimePref::Midnight => "purple_dye",
    };
    let time_lore = [
        &format!("§7Текущий режим: {}", prefs.time.colored_label()),
        "",
        "§7Фиксирует освещение неба на вашем экране:",
        "§7Серверное / День (6000) / Закат / Полночь.",
        "",
        "§e▸ Нажмите для переключения времени",
    ];
    gui.set_item(20, make_item("clock", "§6§l🕒 Личное время суток", &time_lore));
    gui.set_item(
        29,
        make_item(
            time_dye,
            &format!("§7Время: {}", prefs.time.colored_label()),
            &["§e▸ Нажмите, чтобы переключить"],
        ),
    );

    // 3. Live HUD Compass (Slots 21 / 30)
    let hud_dye = if prefs.hud_compass { "lime_dye" } else { "gray_dye" };
    gui.set_item(
        21,
        make_item(
            "recovery_compass",
            "§b§l🧭 Живой HUD-компас",
            &[
                &format!("§7Статус: {}", bool_badge(prefs.hud_compass)),
                "",
                "§7Показывает над хотбаром направление взгляда",
                "§7(С/СВ/В/ЮВ/Ю/ЮЗ/З/СЗ), XYZ, часы мира и пинг.",
                "",
                "§e▸ Нажмите, чтобы включить / выключить",
            ],
        ),
    );
    gui.set_item(
        30,
        make_item(
            hud_dye,
            &format!("§7HUD-компас: {}", bool_badge(prefs.hud_compass)),
            &["§e▸ Нажмите для переключения"],
        ),
    );

    // 4. Chat Mention Sound (Slots 23 / 32)
    let mention_dye = if prefs.chat_mentions { "lime_dye" } else { "gray_dye" };
    gui.set_item(
        23,
        make_item(
            "bell",
            "§d§l🔔 Звук упоминаний в чате",
            &[
                &format!("§7Статус: {}", bool_badge(prefs.chat_mentions)),
                "",
                "§7Мелодичный перезвон нотного блока, когда",
                "§7кто-то пишет ваш никнейм в общем чате.",
                "",
                "§e▸ Нажмите, чтобы включить / выключить",
            ],
        ),
    );
    gui.set_item(
        32,
        make_item(
            mention_dye,
            &format!("§7Упоминания: {}", bool_badge(prefs.chat_mentions)),
            &["§e▸ Нажмите для переключения"],
        ),
    );

    // 5. Hide Players / Solitude Mode (Slots 24 / 33)
    let hide_dye = if prefs.hide_players { "lime_dye" } else { "gray_dye" };
    gui.set_item(
        24,
        make_item(
            "ender_eye",
            "§a§l👁 Режим уединения",
            &[
                &format!("§7Статус: {}", bool_badge(prefs.hide_players)),
                "",
                "§7Скрывает модели других игроков вокруг вас",
                "§7для спокойного строительства или съёмки.",
                "",
                "§e▸ Нажмите, чтобы включить / выключить",
            ],
        ),
    );
    gui.set_item(
        33,
        make_item(
            hide_dye,
            &format!("§7Скрытие игроков: {}", bool_badge(prefs.hide_players)),
            &["§e▸ Нажмите для переключения"],
        ),
    );

    // 6. Auto-Give Compass on Join (Slots 25 / 34)
    let auto_dye = if prefs.auto_compass { "lime_dye" } else { "gray_dye" };
    gui.set_item(
        25,
        make_item(
            "chest",
            "§6§l🎒 Авто-выдача компаса",
            &[
                &format!("§7Статус: {}", bool_badge(prefs.auto_compass)),
                "",
                "§7Бережно кладёт Компас KRONOS в свободный",
                "§7слот при входе и возрождении (без перезаписи!).",
                "",
                "§e▸ Нажмите, чтобы включить / выключить",
            ],
        ),
    );
    gui.set_item(
        34,
        make_item(
            auto_dye,
            &format!("§7Авто-компас: {}", bool_badge(prefs.auto_compass)),
            &["§e▸ Нажмите для переключения"],
        ),
    );

    // Slot 49: Back
    gui.set_item(
        49,
        make_item(
            "arrow",
            "§e§l« Назад в главное меню",
            &["§7Вернуться на главную страницу (/menu)"],
        ),
    );

    player.open_gui(gui);
}

fn open_java_rules(player: &Player) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8📜 §6§lПРАВИЛА СЕРВЕРА KRONOS §8✦"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "enchanted_book",
            "§6§l📜 КОДЕКС МИРА KRONOS",
            &[
                "§7На сервере царит честное и дружелюбное выживание.",
                "§7Нажмите на любую карточку ниже, чтобы вывести",
                "§7подробный текст правила себе в чат!",
            ],
        ),
    );

    gui.set_item(
        19,
        make_item(
            "diamond_sword",
            "§6§l1. Честная игра без читов",
            &[
                "§7Запрещены чит-клиенты, X-Ray, KillAura,",
                "§7Fly, макросы и нечестные модификации.",
                "§a✔ Разрешены: §fSodium, Iris, оптимизация.",
                "",
                "§e▸ Нажмите, чтобы вывести в чат",
            ],
        ),
    );

    gui.set_item(
        21,
        make_item(
            "paper",
            "§e§l2. Уважение в чате",
            &[
                "§7Дружелюбное общение без оскорблений,",
                "§7травли, спама, флуда и рекламы.",
                "",
                "§e▸ Нажмите, чтобы вывести в чат",
            ],
        ),
    );

    gui.set_item(
        23,
        make_item(
            "tnt",
            "§c§l3. Запрет грифа и воровства",
            &[
                "§7Запрещено ломать чужие постройки,",
                "§7брать вещи из чужих сундуков и вредить",
                "§7питомцам или территории других игроков.",
                "",
                "§e▸ Нажмите, чтобы вывести в чат",
            ],
        ),
    );

    gui.set_item(
        29,
        make_item(
            "redstone",
            "§d§l4. Запрет лаг-машин и дюпов",
            &[
                "§7Запрещены любые дюпы предметов,",
                "§7чанк-баны и намеренные лаг-генераторы,",
                "§7мешающие стабильным 20 TPS.",
                "",
                "§e▸ Нажмите, чтобы вывести в чат",
            ],
        ),
    );

    gui.set_item(
        31,
        make_item(
            "tripwire_hook",
            "§b§l5. Безопасность аккаунта",
            &[
                "§7Храните пароль от §e/auth §7в секрете и",
                "§7не передавайте доступ третьим лицам.",
                "",
                "§e▸ Нажмите, чтобы вывести в чат",
            ],
        ),
    );

    gui.set_item(
        33,
        make_item(
            "golden_apple",
            "§a§l6. Справедливая модерация",
            &[
                "§7Спорные ситуации решаются объективно",
                "§7и спокойно ради комфорта всех игроков.",
                "",
                "§e▸ Нажмите, чтобы вывести в чат",
            ],
        ),
    );

    gui.set_item(
        49,
        make_item(
            "arrow",
            "§e§l« Назад в главное меню",
            &["§7Вернуться на главную страницу (/menu)"],
        ),
    );

    player.open_gui(gui);
}

fn open_java_ranks(player: &Player) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8👑 §6§lРАНГИ И ПРИВИЛЕГИИ KRONOS §8✦"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "golden_helmet",
            "§6§l👑 ИЕРАРХИЯ РАНГОВ KRONOS",
            &[
                "§7Каждый статус отображается в Табе и в чате",
                "§7с уникальным значком и приоритетом сортировки.",
            ],
        ),
    );

    gui.set_item(
        11,
        make_item(
            "netherite_helmet",
            "§4§l[👑 ВЛАДЕЛЕЦ]",
            &[
                "§7Основатель и главный архитектор KRONOS.",
                "§7Полное управление ядром и миром.",
                "",
                "§e▸ Нажмите для описания в чате",
            ],
        ),
    );

    gui.set_item(
        13,
        make_item(
            "diamond_helmet",
            "§c§l[⚡ АДМИН]",
            &[
                "§7Администрация проекта, техническая",
                "§7поддержка, ивенты и контроль стабильности.",
                "",
                "§e▸ Нажмите для описания в чате",
            ],
        ),
    );

    gui.set_item(
        15,
        make_item(
            "shield",
            "§9§l[🛡 МОДЕР]",
            &[
                "§7Хранители порядка, защита от грифа",
                "§7и быстрая помощь игрокам на сервере.",
                "",
                "§e▸ Нажмите для описания в чате",
            ],
        ),
    );

    gui.set_item(
        29,
        make_item(
            "amethyst_cluster",
            "§6§l[✦ KRONOS]",
            &[
                "§7Высший почётный ранг меценатов и",
                "§7хранителей легенды мира KRONOS.",
                "",
                "§e▸ Нажмите для описания в чате",
            ],
        ),
    );

    gui.set_item(
        30,
        make_item(
            "enchanted_golden_apple",
            "§d§l[★ ЛЕГЕНДА]",
            &[
                "§7Почётный титул выдающихся строителей,",
                "§7исследователей и ветеранов выживания.",
                "",
                "§e▸ Нажмите для описания в чате",
            ],
        ),
    );

    gui.set_item(
        32,
        make_item(
            "emerald",
            "§a§l[♦ VIP]",
            &[
                "§7Изумрудный префикс в чате и Табе,",
                "§7поддержка развития и почётное место.",
                "",
                "§e▸ Нажмите для описания в чате",
            ],
        ),
    );

    gui.set_item(
        33,
        make_item(
            "iron_helmet",
            "§7[Игрок]",
            &[
                "§7Стартовый статус каждого выжившего:",
                "§7полный доступ к миру, §a/kit§7, §e/spawn§7, §6/menu§7!",
                "",
                "§e▸ Нажмите для описания в чате",
            ],
        ),
    );

    gui.set_item(
        49,
        make_item(
            "arrow",
            "§e§l« Назад в главное меню",
            &["§7Вернуться на главную страницу (/menu)"],
        ),
    );

    player.open_gui(gui);
}

fn open_java_server_info(player: &Player) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8ℹ §b§lО СЕРВЕРЕ И КОМАНДАХ §8✦"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        13,
        make_item(
            "beacon",
            "§6§l✦ СЕРВЕР KRONOS SURVIVAL ✦",
            &[
                "§7Сверхбыстрое ядро на языке §6Rust (Pumpkin)",
                "§7с модульными WASM-плагинами и защитой.",
                "",
                "§e▸ Нажмите, чтобы вывести сводку в чат",
            ],
        ),
    );

    gui.set_item(
        20,
        make_item(
            "compass",
            "§a§l🌐 Версии и Подключение",
            &[
                "§7• Java Edition: §a1.20.1 – 26.3 §8(Velocity + ViaVersion)",
                "§7• Bedrock Edition: §bпорт 19132 (UDP) §8(Geyser)",
                "§7• Адрес Tailscale: §e100.90.47.115",
                "",
                "§e▸ Нажмите, чтобы скопировать IP в чате",
            ],
        ),
    );

    gui.set_item(
        22,
        make_item(
            "writable_book",
            "§e§l⌨ Навигация и Выживание",
            &[
                "§6/menu §7(/m) — Главное меню сервера",
                "§e/settings §7— Личная погода, время и HUD",
                "§6/rules §7— Правила честной игры",
                "§b/spawn §7— Телепорт на точку спавна",
                "§a/kit §7— Стартовый набор выжившего",
                "§d/ranks §7— Список рангов сервера",
            ],
        ),
    );

    gui.set_item(
        24,
        make_item(
            "totem_of_undying",
            "§b§l🔐 Безопасность и Сессии",
            &[
                "§b/auth §7(/account, /pin) — Личный кабинет",
                "§a/register <пароль> §7— Регистрация",
                "§e/login <пароль> §7(/l) — Вход в аккаунт",
                "§e/changepass <ст> <нов> §7— Смена пароля",
                "§c/logout §7— Выход из 24ч IP-сессии",
            ],
        ),
    );

    gui.set_item(
        31,
        make_item(
            "experience_bottle",
            "§d§l⚡ Фишки проекта KRONOS",
            &[
                "§a✔ §fЕдиный мир для друзей с ПК и телефонов",
                "§a✔ §fУмная авто-выдача компаса без потери вещей",
                "§a✔ §fЖивой HUD-компас и личная атмосфера неба",
                "§a✔ §fМгновенный отклик и минимальный пинг",
            ],
        ),
    );

    gui.set_item(
        49,
        make_item(
            "arrow",
            "§e§l« Назад в главное меню",
            &["§7Вернуться на главную страницу (/menu)"],
        ),
    );

    player.open_gui(gui);
}

pub fn open_bedrock_menu_page(
    player: &Player,
    page: MenuPage,
    prefs: &PlayerPrefs,
) -> Option<(u32, BedrockFormKind)> {
    let bedrock = player.as_bedrock()?;
    match page {
        MenuPage::Main => {
            let pos = player.get_position();
            let content = format!(
                "§7Игрок: §f{} §8│ §7Пинг: {}\n§7XYZ: §f{} {} {}\n§7Погода: {} §8│ §7Время: {}",
                player.get_name(),
                ping_colored(player.get_ping()),
                pos.0.floor() as i32,
                pos.1.floor() as i32,
                pos.2.floor() as i32,
                prefs.weather.colored_label(),
                prefs.time.colored_label(),
            );
            let form = SimpleFormBuilder::new(legacy("§6§l✦ МЕНЮ KRONOS ✦"), legacy(&content))
                .button(
                    legacy("§e§l⚙ Настройки игрока\n§8Погода, время, HUD, уединение"),
                    None,
                )
                .button(legacy("§6§l📜 Правила сервера\n§8Кодекс честной игры"), None)
                .button(legacy("§b§l⚡ Телепорт на Спавн\n§8Вернуться на спавн"), None)
                .button(
                    legacy("§d§l👑 Ранги и привилегии\n§8Все 7 статусов KRONOS"),
                    None,
                )
                .button(
                    legacy("§a§l🔐 Кабинет безопасности\n§8Справка /auth и 24ч IP"),
                    None,
                )
                .button(
                    legacy("§b§lℹ О сервере и командах\n§8Версии, IP и команды"),
                    None,
                )
                .button(
                    legacy("§6§l🧭 Получить компас меню\n§8Выдать в свободный слот"),
                    None,
                )
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::Main))
        }
        MenuPage::Settings => {
            let weather_opts = vec![
                WeatherPref::Server.label().to_string(),
                WeatherPref::Clear.label().to_string(),
                WeatherPref::Downfall.label().to_string(),
            ];
            let time_opts = vec![
                TimePref::Server.label().to_string(),
                TimePref::Day.label().to_string(),
                TimePref::Sunset.label().to_string(),
                TimePref::Midnight.label().to_string(),
            ];
            // Zero labels so custom form response indices [0..6] match across all Bedrock clients
            let form = CustomFormBuilder::new(legacy("§e§l⚙ НАСТРОЙКИ KRONOS"))
                .dropdown(
                    legacy("§e🌤 Личная погода"),
                    weather_opts,
                    prefs.weather.to_index(),
                )
                .dropdown(
                    legacy("§6🕒 Личное время суток"),
                    time_opts,
                    prefs.time.to_index(),
                )
                .toggle(
                    legacy("§b🧭 Живой HUD-компас над хотбаром"),
                    prefs.hud_compass,
                )
                .toggle(
                    legacy("§d🔔 Звук упоминаний в чате"),
                    prefs.chat_mentions,
                )
                .toggle(
                    legacy("§a👁 Скрытие игроков (Режим уединения)"),
                    prefs.hide_players,
                )
                .toggle(
                    legacy("§6🎒 Авто-выдача компаса при входе"),
                    prefs.auto_compass,
                )
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::Settings))
        }
        MenuPage::Rules => {
            let content = concat!(
                "§61. Честная игра: §fбез читов, X-Ray, макросов и эксплойтов.\n\n",
                "§e2. Уважение в чате: §fбез оскорблений, травли, спама и рекламы.\n\n",
                "§c3. Без грифа и воровства: §fчужие базы, сундуки и питомцы неприкосновенны.\n\n",
                "§d4. Без лаг-машин и дюпов: §fберегите стабильные 20 TPS.\n\n",
                "§b5. Безопасность: §fхраните пароль /auth в тайне.\n\n",
                "§a6. Справедливая модерация: §fвзаимоуважение и честный разбор ситуаций."
            );
            let form =
                SimpleFormBuilder::new(legacy("§6§l📜 ПРАВИЛА KRONOS"), legacy(content))
                    .button(legacy("§e« Назад в главное меню"), None)
                    .build();
            Some((bedrock.open_form(form), BedrockFormKind::Rules))
        }
        MenuPage::RanksInfo => {
            let content = concat!(
                "§4§l[👑 ВЛАДЕЛЕЦ] §7— Основатель проекта KRONOS\n",
                "§c§l[⚡ АДМИН] §7— Управление и техподдержка\n",
                "§9§l[🛡 МОДЕР] §7— Хранитель порядка и правил\n",
                "§6§l[✦ KRONOS] §7— Высший почётный статус сервера\n",
                "§d§l[★ ЛЕГЕНДА] §7— Ветераны и мастера выживания\n",
                "§a§l[♦ VIP] §7— Изумрудный статус и поддержка\n",
                "§7[Игрок] — Полный доступ к выживанию, /kit и /menu"
            );
            let form =
                SimpleFormBuilder::new(legacy("§6§l👑 РАНГИ KRONOS"), legacy(content))
                    .button(legacy("§e« Назад в главное меню"), None)
                    .build();
            Some((bedrock.open_form(form), BedrockFormKind::RanksInfo))
        }
        MenuPage::ServerInfo => {
            let content = concat!(
                "§6✦ KRONOS SURVIVAL (Rust / Pumpkin)\n\n",
                "§7• Java Edition: §a1.20.1 – 26.3\n",
                "§7• Bedrock Edition: §bпорт 19132 (UDP)\n",
                "§7• Tailscale IP: §e100.90.47.115\n\n",
                "§6Основные команды:\n",
                "§f/menu (/m), /settings, /rules, /spawn, /kit, /ranks\n",
                "§b/auth, /login, /register, /changepass, /logout"
            );
            let form =
                SimpleFormBuilder::new(legacy("§b§lℹ О СЕРВЕРЕ KRONOS"), legacy(content))
                    .button(legacy("§e« Назад в главное меню"), None)
                    .build();
            Some((bedrock.open_form(form), BedrockFormKind::ServerInfo))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinal_from_yaw() {
        assert_eq!(cardinal_from_yaw(0.0), "Ю");
        assert_eq!(cardinal_from_yaw(45.0), "ЮЗ");
        assert_eq!(cardinal_from_yaw(90.0), "З");
        assert_eq!(cardinal_from_yaw(135.0), "СЗ");
        assert_eq!(cardinal_from_yaw(180.0), "С");
        assert_eq!(cardinal_from_yaw(-180.0), "С");
        assert_eq!(cardinal_from_yaw(-135.0), "СВ");
        assert_eq!(cardinal_from_yaw(-90.0), "В");
        assert_eq!(cardinal_from_yaw(-45.0), "ЮВ");
    }

    #[test]
    fn test_format_world_clock() {
        assert_eq!(format_world_clock(0), "06:00");
        assert_eq!(format_world_clock(6000), "12:00");
        assert_eq!(format_world_clock(12000), "18:00");
        assert_eq!(format_world_clock(18000), "00:00");
    }
}
