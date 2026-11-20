pub mod bedrock;
pub mod chat_guide;
pub mod main_menu;
pub mod nav;
pub mod ranks;
pub mod settings;

use pumpkin_plugin_api::{
    ItemStackExt, Player,
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

use crate::store::{MenuPage, PlayerPrefs, TimePref, WeatherPref};

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

pub fn fill_kronos_frame(gui: &Gui) {
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

pub fn bool_badge(v: bool) -> &'static str {
    if v { "§aВкл ✔" } else { "§7Выкл ✖" }
}

pub fn create_menu_compass() -> ItemStack {
    let stack = make_item(
        "compass",
        "§6§l✦ Меню KRONOS §7(ПКМ)",
        &[
            "§7Главный навигатор сервера §6§lKRONOS",
            "",
            "§e▸ Нажмите ПКМ в руке§7, чтобы открыть:",
            "  §f• §bТелепорты, Варпы и Дома",
            "  §f• §aЭкономику, Чаты и 12 Рангов",
            "  §f• §eНастройки, Правила и Безопасность",
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
        player.set_inventory_item(slot, Some(create_menu_compass()));
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
    let deg = ((yaw % 360.0) + 360.0) % 360.0;
    let idx = (((deg + 22.5) % 360.0) / 45.0).floor() as usize % 8;
    const DIRS: [&str; 8] = ["Ю", "ЮЗ", "З", "СЗ", "С", "СВ", "В", "ЮВ"];
    DIRS[idx]
}

pub fn format_world_clock(ticks_of_day: u64) -> String {
    let total_mc_minutes = ((ticks_of_day % 24000) * 60) / 1000;
    let total_minutes = (360 + total_mc_minutes) % (24 * 60);
    format!("{:02}:{:02}", total_minutes / 60, total_minutes % 60)
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
    let world = player.get_world();
    let clock = format_world_clock(world.get_time_of_day());
    let ping = ping_colored(player.get_ping());

    player.show_actionbar(legacy(&format!(
        "§6🧭 §e§l{dir} §8│ §7XYZ: §f{} {} {} §8│ §7🕒 §e{clock} §8│ §7Пинг: {ping}",
        pos.0.floor() as i32,
        pos.1.floor() as i32,
        pos.2.floor() as i32
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
    player.spawn_particles(Particle::EndRod, (target.0, target.1 + 1.0, target.2), 26, (0.55, 0.65, 0.55), 0.07);
    player.play_custom_sound("minecraft:entity.enderman.teleport", SoundCategory::Players, 0.85, 1.1);
    player.show_actionbar(legacy("§b⚡ Телепортация на точку спавна выполнена!"));
}

pub fn send_security_cabinet_guide(player: &Player) {
    player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
    player.send_system_message(legacy("§6§l✦ KRONOS SECURITY §8• §bКабинет безопасности аккаунта"), false);
    player.send_system_message(legacy("  §7• §aАвто-вход 24 часа: §fпри совпадении IP пароль не запрашивается"), false);
    player.send_system_message(legacy("  §7• §eСмена пароля: §b/changepass <старый> <новый>"), false);
    player.send_system_message(legacy("  §7• §cЗавершить сессию: §b/logout §7(сбросит привязку IP)"), false);

    let actions = legacy("  §8» ")
        .add_child(legacy("§b§l[🔐 /auth] ").click_run_command("/auth").hover_show_text(legacy("§7Открыть кабинет авторизации")))
        .add_child(legacy("§e§l[🔑 Сменить пароль] ").click_suggest_command("/changepass ").hover_show_text(legacy("§7Подставить /changepass <старый> <новый>")))
        .add_child(legacy("§c§l[🚪 /logout]").click_suggest_command("/logout").hover_show_text(legacy("§7Подставить /logout")));
    player.send_system_message(actions, false);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
}

pub fn send_rule_card_to_chat(player: &Player, rule_idx: usize) {
    let rules: [(&str, &[&str]); 6] = [
        ("§6§l📜 Правило 1 — Честная игра без читов", &["§7• Запрещены читы, X-Ray, KillAura, Fly, макросы.", "§7• Разрешены Sodium, Iris, Lithium и оптимизация."]),
        ("§e§l📜 Правило 2 — Уважение и культура в чате", &["§7• Запрещены оскорбления, спам, капс-флуд и реклама.", "§7• Для рекламы товаров используйте торговый чат §e$ §7или §e/trade§7."]),
        ("§c§l📜 Правило 3 — Запрет грифа и воровства", &["§7• Чужие постройки, сундуки, фермы и питомцы неприкосновенны."]),
        ("§d§l📜 Правило 4 — Запрет лаг-машин и дюпов", &["§7• Запрещены дюпы, чанк-баны и намеренные лаг-генераторы."]),
        ("§b§l📜 Правило 5 — Безопасность аккаунта", &["§7• Храните пароль от §e/auth §7в секрете и используйте §c/logout §7на чужом ПК."]),
        ("§a§l📜 Правило 6 — Справедливая модерация", &["§7• Команда KRONOS разбирает спорные ситуации объективно и спокойно."]),
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

pub fn send_server_info_to_chat(player: &Player) {
    player.play_custom_sound("minecraft:ui.toast.in", SoundCategory::Ui, 0.85, 1.15);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
    player.send_system_message(legacy("§6§l✦ ИНФОРМАЦИЯ О СЕРВЕРЕ KRONOS ✦"), false);
    player.send_system_message(legacy("  §7• §fJava: §a1.20.1 – 26.3 §8| §fBedrock: §b19132 (UDP)"), false);
    let ip_line = legacy("  §7• §fTailscale IP: §e§n100.90.47.115§r §8(клик для копирования)")
        .click_copy_to_clipboard("100.90.47.115")
        .hover_show_text(legacy("§aСкопировать 100.90.47.115"));
    player.send_system_message(ip_line, false);
    player.send_system_message(legacy("  §7• §fПлагины: §6Essentials §8• §bWarps §8• §eEconomy §8• §aChat §8• §dRanks"), false);
    player.send_system_message(legacy("§8§m──────────────────────────────────────────────"), false);
}

pub fn open_java_menu_page(player: &Player, page: MenuPage, prefs: &PlayerPrefs, balance: &str) {
    match page {
        MenuPage::Main => main_menu::open_java_main(player, prefs, balance),
        MenuPage::Navigation => nav::open_java_nav(player),
        MenuPage::Economy => chat_guide::open_java_economy(player, balance),
        MenuPage::ChatGuide => chat_guide::open_java_chat_guide(player),
        MenuPage::RanksInfo => ranks::open_java_ranks(player),
        MenuPage::Settings => settings::open_java_settings(player, prefs),
        MenuPage::Rules => open_java_rules(player),
        MenuPage::ServerInfo => open_java_server_info(player),
    }
}

fn open_java_rules(player: &Player) {
    let gui = Gui::new(Screen::Generic9x6, legacy("§8📜 §6§lПРАВИЛА И БЕЗОПАСНОСТЬ §8✦"));
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(4, make_item("enchanted_book", "§6§l📜 КОДЕКС МИРА KRONOS", &["§7Нажмите на карточку, чтобы вывести в чат."]));
    gui.set_item(19, make_item("diamond_sword", "§6§l1. Честная игра без читов", &["§7Без читов, X-Ray, Fly и макросов.", "", "§e▸ Нажмите для подробностей"]));
    gui.set_item(21, make_item("paper", "§e§l2. Уважение в чате", &["§7Без спама, капса и рекламы.", "", "§e▸ Нажмите для подробностей"]));
    gui.set_item(23, make_item("tnt", "§c§l3. Запрет грифа и воровства", &["§7Чужие постройки и сундуки неприкосновенны.", "", "§e▸ Нажмите для подробностей"]));
    gui.set_item(29, make_item("redstone", "§d§l4. Запрет лаг-машин и дюпов", &["§7Берегите стабильные 20 TPS.", "", "§e▸ Нажмите для подробностей"]));
    gui.set_item(31, make_item("tripwire_hook", "§b§l5. Безопасность аккаунта", &["§7Пароль /auth, /changepass, /logout.", "", "§b▸ Нажмите, чтобы открыть кабинет"]));
    gui.set_item(33, make_item("golden_apple", "§a§l6. Справедливая модерация", &["§7Честный и спокойный разбор ситуаций.", "", "§e▸ Нажмите для подробностей"]));
    gui.set_item(49, make_item("arrow", "§e§l« Назад в главное меню", &["§7Вернуться в /menu"]));
    player.open_gui(gui);
}

fn open_java_server_info(player: &Player) {
    let gui = Gui::new(Screen::Generic9x6, legacy("§8ℹ §b§lО СЕРВЕРЕ KRONOS §8✦"));
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(13, make_item("beacon", "§6§l✦ KRONOS SURVIVAL (Pumpkin 26.3)", &["§7Ядро на Rust + модульные WASM-плагины.", "", "§e▸ Нажмите для сводки в чат"]));
    gui.set_item(20, make_item("compass", "§a§l🌐 Подключение", &["§7Java: §a1.20.1–26.3 §8| §7Bedrock: §b19132", "§7Tailscale IP: §e100.90.47.115"]));
    gui.set_item(22, make_item("writable_book", "§e§l⌨ Плагины KRONOS", &["§b/spawn /rtp /home /warps /pwarp", "§e/bal /pay /baltop §8| §a/msg /r /trade", "§d/ranks /fly /feed /heal /ec"]));
    gui.set_item(24, make_item("totem_of_undying", "§b§l🔐 Безопасность", &["§b/auth §8• §e/changepass §8• §c/logout"]));
    gui.set_item(49, make_item("arrow", "§e§l« Назад в главное меню", &["§7Вернуться в /menu"]));
    player.open_gui(gui);
}
