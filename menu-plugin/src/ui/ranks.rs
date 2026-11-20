use pumpkin_plugin_api::{
    gui::Gui, screens_wit::Screen, wit::pumpkin::plugin::sounds::SoundCategory, Player,
};

use crate::store::MenuPage;
use crate::ui::{fill_kronos_frame, legacy, make_item};

pub struct RankCard {
    pub slot: u32,
    pub item_id: &'static str,
    pub badge: &'static str,
    pub weight: u32,
    pub perks: &'static str,
    pub commands: &'static str,
}

/// Актуальная линейка всех 12 русских рангов KRONOS (`kronos-ranks` v2.0.0).
pub const ALL_12_RANKS: [RankCard; 12] = [
    RankCard { slot: 10, item_id: "iron_helmet", badge: "§7[Игрок]", weight: 0, perks: "1 дом, базовый доступ ко всем мирам", commands: "/menu, /spawn, /rtp, /warps, /bal" },
    RankCard { slot: 11, item_id: "emerald", badge: "§a[VIP]", weight: 10, perks: "1.2x множитель, 3 дома, RTP VIP, чат &a/&b", commands: "/rtp (VIP КД), цветной чат &a и &b" },
    RankCard { slot: 12, item_id: "diamond", badge: "§b[Премиум]", weight: 25, perks: "5 домов, переносной верстак, сытость", commands: "/feed, /workbench (/craft)" },
    RankCard { slot: 13, item_id: "amethyst_shard", badge: "§d[Креатив]", weight: 50, perks: "7 домов, режим полёта и шляпы", commands: "/fly, /hat" },
    RankCard { slot: 14, item_id: "gold_ingot", badge: "§e[Герой]", weight: 80, perks: "10 домов, лечение и эндер-сундук", commands: "/heal, /enderchest (/ec)" },
    RankCard { slot: 15, item_id: "blaze_powder", badge: "§6[Титан]", weight: 120, perks: "15 домов, элитные личные варпы", commands: "/near, /repair, /pwarp (до 7)" },
    RankCard { slot: 16, item_id: "dragon_breath", badge: "§c[Дракон]", weight: 200, perks: "25 домов, мгновенные ТП без задержки", commands: "Обход задержки /warp и /home" },
    RankCard { slot: 22, item_id: "nether_star", badge: "§4§l[СПОНСОР]", weight: 500, perks: "50 домов, все команды, все цвета & и стили", commands: "Все донат-команды + жирный префикс" },
    RankCard { slot: 29, item_id: "lapis_lazuli", badge: "§9[Хелпер]", weight: 600, perks: "Стафф-ранг (Dual-Track), помощь игрокам", commands: "/sc (стафф-чат), модерация" },
    RankCard { slot: 30, item_id: "shield", badge: "§3[Модератор]", weight: 750, perks: "Защита от грифа и контроль порядка", commands: "/sc, /fly, обход задержки ТП" },
    RankCard { slot: 32, item_id: "diamond_helmet", badge: "§c§l[АДМИН]", weight: 900, perks: "Администрирование сервера и рангов", commands: "/rank set|remove|addperm, /eco, /setwarp" },
    RankCard { slot: 33, item_id: "netherite_helmet", badge: "§4§l[СОЗДАТЕЛЬ]", weight: 1000, perks: "Основатель проекта KRONOS (доступ *)", commands: "Полный доступ ко всем модулям" },
];

pub fn open_java_ranks(player: &Player) {
    let gui = Gui::new(
        Screen::Generic9x6,
        legacy("§8👑 §6§lРАНГИ KRONOS (12 СТАТУСОВ) §8✦"),
    );
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "golden_helmet",
            "§6§l👑 12 РУССКИХ РАНГОВ KRONOS",
            &[
                "§7Верхний ряд: §a8 донат-рангов с наследованием прав",
                "§7Нижний ряд: §b4 стафф-ранга (Dual-Track)",
                "",
                "§e▸ Нажмите на любой ранг для подробностей в чате",
            ],
        ),
    );

    for card in &ALL_12_RANKS {
        let weight_line = format!("§7Вес в Табе: §e{}", card.weight);
        let perks_line = format!("§7Возможности: §f{}", card.perks);
        let cmd_line = format!("§7Команды: §b{}", card.commands);
        gui.set_item(
            card.slot,
            make_item(
                card.item_id,
                card.badge,
                &[&weight_line, &perks_line, &cmd_line, "", "§e▸ Нажмите, чтобы вывести в чат"],
            ),
        );
    }

    gui.set_item(49, make_item("arrow", "§e§l« Назад в главное меню", &["§7Вернуться в /menu"]));
    player.open_gui(gui);
}

pub fn handle_ranks_click(player: &Player, slot: i16) -> Option<MenuPage> {
    if slot == 49 {
        player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
        return Some(MenuPage::Main);
    }
    if let Some(card) = ALL_12_RANKS.iter().find(|r| r.slot as i16 == slot) {
        send_rank_card_to_chat(player, card);
    }
    None
}

pub fn send_rank_card_to_chat(player: &Player, card: &RankCard) {
    player.play_custom_sound("minecraft:block.note_block.chime", SoundCategory::Ui, 0.85, 1.25);
    let line = legacy(&format!(
        "§6✦ Ранг {} §8(вес {}) §8— §f{} §8│ §7Команды: §b{} ",
        card.badge, card.weight, card.perks, card.commands
    ))
    .add_child(
        legacy("§e[/ranks]")
            .click_run_command("/ranks")
            .hover_show_text(legacy("§eОткрыть полный каталог /ranks")),
    );
    player.send_system_message(line, false);
}
