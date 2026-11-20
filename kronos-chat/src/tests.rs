use crate::bubble::format_bubble_text;
use crate::filter::{
    apply_anti_caps, apply_color_codes, contains_advertisement, is_duplicate_flood,
    remaining_cooldown_secs, sanitize_input,
};
use crate::models::{is_within_local_radius, Channel, ChatStore};

#[test]
fn sanitizes_section_signs_and_control_characters() {
    assert_eq!(sanitize_input("§c§lFAKE SYSTEM§r Привет\n\t\x07"), "FAKE SYSTEM Привет");
    assert_eq!(sanitize_input("  §4Admin§f: тест  "), "Admin: тест");

    // Без права &-коды остаются обычным текстом
    assert_eq!(apply_color_codes("&aПривет &lмир", false), "&aПривет &lмир");
    // С правом kronos-ranks:chat.color &-коды преобразуются в §
    assert_eq!(apply_color_codes("&aПривет &lмир", true), "§aПривет §lмир");
}

#[test]
fn parses_local_global_and_trade_channels_and_formats() {
    assert_eq!(Channel::parse_message("привет рядом"), (Channel::Local, "привет рядом"));
    assert_eq!(Channel::parse_message("!всем привет"), (Channel::Global, "всем привет"));
    assert_eq!(Channel::parse_message("$продам алмазы"), (Channel::Trade, "продам алмазы"));

    assert_eq!(
        Channel::Local.format_line("§a[VIP] ", "Steve", "ку"),
        "§8[§fЛ§8] §a[VIP] Steve §8» §fку"
    );
    assert_eq!(
        Channel::Global.format_line("", "Alex", "всем хай"),
        "§8[§6Г§8] Alex §8» §fвсем хай"
    );
    assert_eq!(
        Channel::Trade.format_line("", "Notch", "куплю меч"),
        "§8[§e§lТОРГ§8] Notch §8» §eкуплю меч"
    );
}

#[test]
fn anti_caps_triggers_above_5_chars_and_60_percent_uppercase() {
    // Короткие сообщения (<= 5 символов) не трогаются
    assert_eq!(apply_anti_caps("ПРИВ"), "ПРИВ");
    assert_eq!(apply_anti_caps("HELLO"), "HELLO");

    // > 5 символов и > 60% заглавных (кириллица и латиница)
    assert_eq!(apply_anti_caps("ПРИВЕТ ВСЕМ НА СЕРВЕРЕ"), "привет всем на сервере");
    assert_eq!(apply_anti_caps("SELLING DIAMONDS NOW"), "selling diamonds now");

    // Обычное предложение не трогается
    assert_eq!(apply_anti_caps("Привет, как дела на сервере?"), "Привет, как дела на сервере?");
}

#[test]
fn anti_advertisement_blocks_ipv4_and_domains_without_false_positives() {
    assert!(contains_advertisement("Заходите на 192.168.1.50 прямо сейчас"));
    assert!(contains_advertisement("Лучший сервер play.myserver.ru"));
    assert!(contains_advertisement("Мой сайт example.com/shop"));
    assert!(contains_advertisement("Залетай в discord.gg/kronos"));
    assert!(contains_advertisement("Пиши в t.me/mychannel"));
    assert!(contains_advertisement("mc.hypixel.net"));

    // Обычный текст и дробные числа не должны блокироваться
    assert!(!contains_advertisement("Продам меч за 12.50 монет"));
    assert!(!contains_advertisement("Встречаемся на координатах 100 64 -200"));
    assert!(!contains_advertisement("Привет, пошли в шахту!"));
}

#[test]
fn checks_anti_flood_cooldowns_ignore_and_local_radius() {
    assert!(is_duplicate_flood(Some("Продам алмазы"), "продам алмазы"));
    assert!(!is_duplicate_flood(Some("Продам алмазы"), "Куплю железо"));

    assert_eq!(remaining_cooldown_secs(Some(10_000), 11_500, 3_000), Some(2));
    assert_eq!(remaining_cooldown_secs(Some(10_000), 13_000, 3_000), None);

    // Радиус 100 блоков и проверка одного мира
    assert!(is_within_local_radius("minecraft:overworld", (0.0, 64.0, 0.0), "minecraft:overworld", (60.0, 64.0, 80.0), 100.0));
    assert!(!is_within_local_radius("minecraft:overworld", (0.0, 64.0, 0.0), "minecraft:overworld", (101.0, 64.0, 0.0), 100.0));
    assert!(!is_within_local_radius("minecraft:overworld", (0.0, 64.0, 0.0), "minecraft:the_nether", (1.0, 64.0, 1.0), 100.0));

    let mut store = ChatStore::default();
    assert!(store.toggle_ignore("Alex", "Spammer"));
    assert!(store.is_ignoring("Alex", "spammer"));
    assert!(!store.toggle_ignore("Alex", "Spammer"));
    assert!(!store.is_ignoring("Alex", "spammer"));
}

#[test]
fn speech_bubble_sanitizes_and_truncates_to_40_chars() {
    assert_eq!(format_bubble_text("§cПривет!", 40), "§e«§fПривет!§e»");
    let long = "Очень длинное сообщение которое точно превышает лимит в сорок символов";
    let bubble = format_bubble_text(long, 40);
    assert!(bubble.starts_with("§e«§f"));
    assert!(bubble.ends_with("...§e»"));
}
