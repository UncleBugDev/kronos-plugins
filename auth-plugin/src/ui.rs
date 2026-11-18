use pumpkin_plugin_api::{
    EntityStatus, ItemStackExt, Player,
    boss_bar::{BossBar, BossBarColor, BossBarDivision},
    forms::CustomFormBuilder,
    gui::Gui,
    item_stack::ItemStack,
    particles::Particle,
    player::{BedrockDisconnectReason, BedrockKickOptions, JavaKickOptions},
    screens_wit::Screen,
    server::Server,
    text::TextComponent,
    uuid::Uuid,
    wit::pumpkin::plugin::sounds::SoundCategory,
};

use crate::store::{AccountRecord, normalize_ip};

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

pub fn make_pin_stars(pin: &str) -> String {
    let mut stars = String::new();
    let max_dots = 6usize;
    for i in 0..max_dots {
        if i > 0 {
            stars.push(' ');
        }
        if i < pin.len() {
            stars.push_str("§a★");
        } else {
            stars.push_str("§8☆");
        }
    }
    if pin.len() > max_dots {
        for _ in max_dots..pin.len() {
            stars.push_str(" §a★");
        }
    }
    stars
}

pub fn open_auth_gui(player: &Player, pin_buf: &str, registered: bool) {
    let title_str = if registered {
        "§8§l🔐 §6§lKRONOS §8• §eВход в аккаунт"
    } else {
        "§8§l⚡ §6§lKRONOS §8• §aРегистрация"
    };
    let gui = Gui::new(Screen::Generic9x6, legacy(title_str));
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);

    // Top & Bottom border rows + side columns
    for slot in 0..54u32 {
        let row = slot / 9;
        let col = slot % 9;
        if row == 0 || row == 5 || col == 0 || col == 8 {
            gui.set_item(slot, make_item("black_stained_glass_pane", "§8", &[]));
        } else {
            gui.set_item(slot, make_item("gray_stained_glass_pane", "§7", &[]));
        }
    }

    let name = player.get_name();
    let ip = normalize_ip(&player.get_ip());

    // Slot 11: Welcome / Status Totem
    let mode_line = if registered {
        "§eСтатус: §6Требуется авторизация"
    } else {
        "§aСтатус: §bНовый аккаунт (Регистрация)"
    };
    let welcome_item = make_item(
        "totem_of_undying",
        "§6§l✦ ДОБРО ПОЖАЛОВАТЬ НА KRONOS ✦",
        &[
            &format!("§7Игрок: §f§l{name}"),
            mode_line,
            &format!("§7Ваш IP: §b{ip}"),
            "",
            "§7Выбирайте цифры PIN-кода мышкой",
            "§7или просто напишите пароль в §eчат§7!",
        ],
    );
    gui.set_item(11, welcome_item);

    // Slot 13: Live PIN Input Display
    let stars = make_pin_stars(pin_buf);
    let pin_hint = if pin_buf.is_empty() {
        "§7Нажмите цифры ниже (от 4 до 8 цифр)"
    } else {
        "§aНажмите зеленую кнопку §l[✔ ВОЙТИ]§a справа!"
    };
    let pin_display = make_item(
        if pin_buf.len() >= 4 {
            "emerald"
        } else {
            "amethyst_cluster"
        },
        &format!("§b§lPIN-КОД: {stars}"),
        &[
            &format!("§7Введено символов: §f§l{}", pin_buf.len()),
            pin_hint,
            "",
            "§8Пароль шифруется солёным SHA-256",
        ],
    );
    gui.set_item(13, pin_display);

    // Slot 15: IP 24h Auto-Login Info
    let ip_badge = make_item(
        "ender_eye",
        "§d§l🛡 ЗАЩИТА СЕССИИ",
        &[
            "§7• §aАвто-вход 24 часа §7при неизменном IP",
            "§7• §eЗащита инвентаря и координат",
            "§7• §bАнти-утечка пароля в чате",
        ],
    );
    gui.set_item(15, ip_badge);

    // Interactive Keypad: 1..9 in 3x3 block + 0
    let keypad: [(u32, char, &str); 10] = [
        (21, '1', "light_blue_stained_glass_pane"),
        (22, '2', "light_blue_stained_glass_pane"),
        (23, '3', "light_blue_stained_glass_pane"),
        (30, '4', "cyan_stained_glass_pane"),
        (31, '5', "cyan_stained_glass_pane"),
        (32, '6', "cyan_stained_glass_pane"),
        (39, '7', "blue_stained_glass_pane"),
        (40, '8', "blue_stained_glass_pane"),
        (41, '9', "blue_stained_glass_pane"),
        (49, '0', "light_blue_stained_glass_pane"),
    ];

    for (slot, digit, mat) in keypad {
        let stack = make_item(
            mat,
            &format!("§b§l[ Цифра {digit} ]"),
            &[&format!("§7Нажмите, чтобы добавить §f{digit}")],
        );
        stack.set_count(if digit == '0' { 10 } else { (digit as u8) - b'0' });
        gui.set_item(slot, stack);
    }

    // Slot 28: Switch to Chat Input Mode
    let chat_btn = make_item(
        "writable_book",
        "§e§l💬 ВВЕСТИ ПАРОЛЬ В ЧАТ",
        &[
            "§7Хотите сложный буквенный пароль?",
            "§fПросто закройте окно (ESC) и напишите",
            "§aпароль прямо в чат §7(он не виден другим)",
            "§7или введите: §b/register <пароль>§7 / §b/l <пароль>",
        ],
    );
    gui.set_item(28, chat_btn);

    // Slot 34: Rules & Features Overview
    let info_btn = make_item(
        "enchanted_book",
        "§d§l📖 ПРАВИЛА И ФИШКИ СЕРВЕРА",
        &[
            "§a✔ §fЧистое выживание 1.20.1+ / Caves & Cliffs",
            "§a✔ §fПоддержка Java (1.20.1–26.3) и Bedrock",
            "§a✔ §fСтартовый набор новичка §e/kit",
            "§a✔ §fЛичный кабинет §b/auth",
            "",
            "§eНажмите, чтобы вывести памятку в чат!",
        ],
    );
    gui.set_item(34, info_btn);

    // Slot 48: Clear PIN
    let clear_btn = make_item(
        "barrier",
        "§c§l✖ ОЧИСТИТЬ PIN",
        &["§7Сбросить набранные цифры"],
    );
    gui.set_item(48, clear_btn);

    // Slot 50: Submit PIN
    let submit_label = if registered {
        "§a§l✔ ПОДТВЕРДИТЬ ВХОД"
    } else {
        "§a§l✔ ЗАРЕГИСТРИРОВАТЬСЯ"
    };
    let submit_btn = make_item(
        "lime_concrete",
        submit_label,
        &[
            &format!("§7Текущий PIN: {stars}"),
            if pin_buf.len() >= 4 {
                "§aГотово! Нажмите для входа."
            } else {
                "§cМинимум 4 символа!"
            },
        ],
    );
    gui.set_item(50, submit_btn);

    player.open_gui(gui);
}

pub fn open_account_cabinet_gui(player: &Player, acc: Option<&AccountRecord>) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8§l🔐 §6§lKRONOS §8• §bЛичный кабинет"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);

    for slot in 0..54u32 {
        let row = slot / 9;
        let col = slot % 9;
        if row == 0 || row == 5 || col == 0 || col == 8 {
            gui.set_item(slot, make_item("black_stained_glass_pane", "§8", &[]));
        } else {
            gui.set_item(slot, make_item("light_gray_stained_glass_pane", "§7", &[]));
        }
    }

    let name = player.get_name();
    let ip = normalize_ip(&player.get_ip());
    let logins = acc.map_or(1, |a| a.login_count);
    let kit_status = if acc.is_some_and(|a| a.kit_claimed) {
        "§aУже получен"
    } else {
        "§eДоступен!"
    };

    gui.set_item(
        13,
        make_item(
            "nether_star",
            &format!("§6§l★ ПРОФИЛЬ: §f{name}"),
            &[
                "§a✔ Статус: §fАвторизован",
                &format!("§7Всего входов: §e{logins}"),
                &format!("§7Текущий IP: §b{ip}"),
                "§7Авто-вход по IP: §aАктивен (24ч)",
                "",
                "§8Нажмите ESC, чтобы закрыть меню и играть",
            ],
        ),
    );

    gui.set_item(
        20,
        make_item(
            "chest",
            "§6§l🎁 СТАРТОВЫЙ НАБОР",
            &[
                &format!("§7Состояние: {kit_status}"),
                "§7Железная броня, инструменты и еда",
                "§eНажмите, чтобы получить (/kit)",
            ],
        ),
    );

    gui.set_item(
        22,
        make_item(
            "ender_pearl",
            "§b§l🧭 ТЕЛЕПОРТ НА СПАВН",
            &[
                "§7Быстрое возвращение на точку спавна",
                "§eНажмите, чтобы телепортироваться (/spawn)",
            ],
        ),
    );

    gui.set_item(
        24,
        make_item(
            "redstone_block",
            "§c§l🚪 ВЫЙТИ ИЗ СЕССИИ",
            &[
                "§7Сбросить текущую IP-сессию и заблокировать",
                "§7аккаунт до повторного ввода пароля.",
                "§cНажмите для выхода (/logout)",
            ],
        ),
    );

    gui.set_item(
        40,
        make_item(
            "writable_book",
            "§e§l🔑 СМЕНА ПАРОЛЯ",
            &[
                "§7Чтобы сменить пароль, введите команду:",
                "§b/changepass <старый> <новый>",
            ],
        ),
    );

    player.open_gui(gui);
}

pub fn open_bedrock_auth_form(player: &Player, registered: bool) -> Option<u32> {
    let bedrock = player.as_bedrock()?;
    let title = if registered {
        "§6§lKRONOS • Вход"
    } else {
        "§6§lKRONOS • Регистрация"
    };
    let subtitle = if registered {
        "§eВведите ваш пароль для входа на сервер:"
    } else {
        "§aПридумайте надёжный пароль (минимум 4 символа):"
    };
    let form = CustomFormBuilder::new(legacy(title))
        .label(legacy(&format!("§7Игрок: §f{}", player.get_name())))
        .label(legacy(subtitle))
        .input(legacy("§bПароль / PIN:"), "От 4 до 32 символов", "")
        .build();
    Some(bedrock.open_form(form))
}

pub fn give_starter_kit(player: &Player) -> usize {
    let inv = player.get_inventory().as_inventory();
    let cooked_beef = ItemStack::of("cooked_beef", 32);
    cooked_beef.set_custom_name(Some(legacy("§6§lСытный Стейк Выжившего").italic(false)));

    let kit_items = [
        make_item(
            "iron_sword",
            "§b§lКлинок Первопроходца",
            &["§7Стартовое снаряжение KRONOS"],
        ),
        make_item(
            "iron_pickaxe",
            "§b§lКирка Рудокопа",
            &["§7Стартовое снаряжение KRONOS"],
        ),
        make_item(
            "iron_axe",
            "§b§lТопор Лесоруба",
            &["§7Стартовое снаряжение KRONOS"],
        ),
        cooked_beef,
        ItemStack::of("torch", 32),
        ItemStack::of("oak_log", 16),
        make_item(
            "iron_chestplate",
            "§7§lНагрудник Защитника",
            &["§7Стартовое снаряжение KRONOS"],
        ),
    ];

    let mut placed = 0usize;
    let size = inv.get_size();
    for item in kit_items {
        for slot in 0..size {
            if inv.get_item(slot).is_none() {
                inv.set_item(slot, Some(item));
                placed += 1;
                break;
            }
        }
    }
    placed
}

pub fn trigger_welcome_effects(player: &Player, is_new_registration: bool) {
    let pos = player.get_position();
    let entity_id = player.as_entity().get_id() as i32;

    if let Some(java) = player.as_java() {
        java.send_entity_status(entity_id, EntityStatus::ProtectedFromDeath);
    }

    player.play_custom_sound(
        "minecraft:ui.toast.challenge_complete",
        SoundCategory::Master,
        1.0,
        1.1,
    );
    player.play_custom_sound(
        "minecraft:entity.player.levelup",
        SoundCategory::Players,
        0.9,
        1.25,
    );

    player.spawn_particles(
        Particle::TotemOfUndying,
        (pos.0, pos.1 + 1.2, pos.2),
        45,
        (0.8, 0.9, 0.8),
        0.25,
    );
    player.spawn_particles(
        Particle::EndRod,
        (pos.0, pos.1 + 1.0, pos.2),
        24,
        (0.6, 0.6, 0.6),
        0.08,
    );

    player.send_title_animation(10, 55, 20);
    if is_new_registration {
        player.show_title(legacy("§a§l✦ ДОБРО ПОЖАЛОВАТЬ! ✦"));
        player.show_subtitle(legacy("§fАккаунт защищён • Приятной игры на §6§lKRONOS"));
        player.show_actionbar(legacy("§a✔ Регистрация завершена! Выдан стартовый набор"));
    } else {
        player.show_title(legacy("§b§l✦ С ВОЗВРАЩЕНИЕМ! ✦"));
        player.show_subtitle(legacy("§fАвторизация пройдена • Удачи на §6§lKRONOS"));
        player.show_actionbar(legacy("§a✔ Вы успешно вошли в аккаунт"));
    }

    send_clickable_dashboard(player);
}

pub fn send_clickable_dashboard(player: &Player) {
    player.send_system_message(legacy("§8§m────────────────────────────────────────"), false);

    let btn_kit = legacy("§a[🎁 Набор /kit] ")
        .click_run_command("/kit")
        .hover_show_text(legacy("§7Нажмите, чтобы получить стартовый набор"));
    let btn_spawn = legacy("§e[🧭 На спавн] ")
        .click_run_command("/spawn")
        .hover_show_text(legacy("§7Телепортироваться на точку спавна"));
    let btn_cab = legacy("§d[🔐 Кабинет]")
        .click_run_command("/auth")
        .hover_show_text(legacy("§7Открыть профиль и настройки защиты"));

    let line = legacy("§6§l✦ KRONOS §8» ")
        .add_child(btn_kit)
        .add_child(btn_spawn)
        .add_child(btn_cab);
    player.send_system_message(line, false);
    player.send_system_message(legacy("§8§m────────────────────────────────────────"), false);
}

pub fn apply_tab_branding(_player: &Player, _online_count: u32) {
    // Tab-list rendering and player rows are managed by tab-plugin.
}

pub fn create_auth_boss_bar(secs: u32) -> BossBar {
    let bar = BossBar::new(
        legacy(&format!(
            "§6§l✦ KRONOS SECURITY §8• §eОсталось §c§l{secs} §eсек для входа"
        )),
        BossBarColor::Yellow,
        BossBarDivision::Notches12,
    );
    bar.set_health(1.0);
    bar
}

pub fn attach_boss_bar(server: &Server, uuid: Uuid, bar: &BossBar) {
    if let Some(p) = server.get_player_by_uuid(uuid) {
        bar.add_player(p);
    }
}

pub fn detach_boss_bar(server: &Server, uuid: Uuid, bar: &BossBar) {
    if let Some(p) = server.get_player_by_uuid(uuid) {
        bar.remove_player(p);
    }
    bar.remove_all();
}

pub fn kick_player(player: &Player, reason: &str) {
    if let Some(java) = player.as_java() {
        java.kick(JavaKickOptions::new(legacy(reason)));
    } else if let Some(bedrock) = player.as_bedrock() {
        bedrock.kick(&BedrockKickOptions::new(
            BedrockDisconnectReason::Kicked,
            reason,
        ));
    }
}
