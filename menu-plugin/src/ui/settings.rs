use pumpkin_plugin_api::{
    gui::Gui, screens_wit::Screen, server::Server, wit::pumpkin::plugin::sounds::SoundCategory,
    Player,
};

use crate::store::{save_current_db, with_state, MenuPage, PlayerPrefs, TimePref, WeatherPref};
use crate::ui::{
    apply_time_pref, apply_visibility_pref, apply_weather_pref, bool_badge, fill_kronos_frame,
    give_menu_compass_non_destructive, legacy, make_item, show_hud_compass_actionbar,
};

pub fn open_java_settings(player: &Player, prefs: &PlayerPrefs) {
    let gui = Gui::new(Screen::Generic9x6, legacy("§8⚙ §e§lНАСТРОЙКИ ИГРОКА §8✦"));
    gui.set_allow_grab_items(false);
    gui.set_allow_put_items(false);
    fill_kronos_frame(&gui);

    gui.set_item(
        4,
        make_item(
            "comparator",
            "§e§l⚙ ПЕРСОНАЛЬНЫЕ НАСТРОЙКИ",
            &["§7Переключатели применяются мгновенно", "§7и атомарно сохраняются в JSON."],
        ),
    );

    let (w_icon, w_dye) = match prefs.weather {
        WeatherPref::Server => ("sunflower", "gray_dye"),
        WeatherPref::Clear => ("sunflower", "yellow_dye"),
        WeatherPref::Downfall => ("water_bucket", "light_blue_dye"),
    };
    gui.set_item(19, make_item(w_icon, "§e§l🌤 Личная погода", &[&format!("§7Режим: {}", prefs.weather.colored_label()), "", "§e▸ Нажмите для переключения"]));
    gui.set_item(28, make_item(w_dye, &format!("§7Погода: {}", prefs.weather.colored_label()), &["§e▸ Переключить"]));

    let t_dye = match prefs.time {
        TimePref::Server => "gray_dye",
        TimePref::Day => "yellow_dye",
        TimePref::Sunset => "orange_dye",
        TimePref::Midnight => "purple_dye",
    };
    gui.set_item(20, make_item("clock", "§6§l🕒 Личное время суток", &[&format!("§7Режим: {}", prefs.time.colored_label()), "", "§e▸ Нажмите для переключения"]));
    gui.set_item(29, make_item(t_dye, &format!("§7Время: {}", prefs.time.colored_label()), &["§e▸ Переключить"]));

    let hud_dye = if prefs.hud_compass { "lime_dye" } else { "gray_dye" };
    gui.set_item(21, make_item("recovery_compass", "§b§l🧭 Живой HUD-компас", &[&format!("§7Статус: {}", bool_badge(prefs.hud_compass)), "", "§7Показывает стороны света, XYZ, часы и пинг."]));
    gui.set_item(30, make_item(hud_dye, &format!("§7HUD-компас: {}", bool_badge(prefs.hud_compass)), &["§e▸ Переключить"]));

    let m_dye = if prefs.chat_mentions { "lime_dye" } else { "gray_dye" };
    gui.set_item(23, make_item("bell", "§d§l🔔 Звук упоминаний в чате", &[&format!("§7Статус: {}", bool_badge(prefs.chat_mentions)), "", "§7Звон при упоминании вашего ника."]));
    gui.set_item(32, make_item(m_dye, &format!("§7Упоминания: {}", bool_badge(prefs.chat_mentions)), &["§e▸ Переключить"]));

    let h_dye = if prefs.hide_players { "lime_dye" } else { "gray_dye" };
    gui.set_item(24, make_item("ender_eye", "§a§l👁 Режим уединения", &[&format!("§7Статус: {}", bool_badge(prefs.hide_players)), "", "§7Скрывает других игроков вокруг вас."]));
    gui.set_item(33, make_item(h_dye, &format!("§7Скрытие игроков: {}", bool_badge(prefs.hide_players)), &["§e▸ Переключить"]));

    let a_dye = if prefs.auto_compass { "lime_dye" } else { "gray_dye" };
    gui.set_item(25, make_item("chest", "§6§l🎒 Авто-выдача компаса", &[&format!("§7Статус: {}", bool_badge(prefs.auto_compass)), "", "§7Выдаёт компас в свободный слот при входе."]));
    gui.set_item(34, make_item(a_dye, &format!("§7Авто-компас: {}", bool_badge(prefs.auto_compass)), &["§e▸ Переключить"]));

    gui.set_item(49, make_item("arrow", "§e§l« Назад в главное меню", &["§7Вернуться в /menu"]));
    player.open_gui(gui);
}

pub fn handle_settings_click(server: &Server, player: &Player, slot: i16) -> Option<MenuPage> {
    if slot == 49 {
        player.play_custom_sound("minecraft:ui.button.click", SoundCategory::Ui, 0.8, 1.0);
        return Some(MenuPage::Main);
    }

    let key = player.get_name().to_lowercase();
    match slot {
        19 | 28 => {
            let updated = with_state(|st| st.update_prefs(&key, |p| p.weather = p.weather.cycle()));
            save_current_db();
            apply_weather_pref(player, updated.weather);
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.8, 1.3);
            Some(MenuPage::Settings)
        }
        20 | 29 => {
            let updated = with_state(|st| st.update_prefs(&key, |p| p.time = p.time.cycle()));
            save_current_db();
            apply_time_pref(player, updated.time);
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.8, 1.3);
            Some(MenuPage::Settings)
        }
        21 | 30 => {
            let updated = with_state(|st| st.update_prefs(&key, |p| p.hud_compass = !p.hud_compass));
            save_current_db();
            if updated.hud_compass {
                show_hud_compass_actionbar(player);
            } else {
                player.show_actionbar(legacy(""));
            }
            let pitch = if updated.hud_compass { 1.4 } else { 0.95 };
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.8, pitch);
            Some(MenuPage::Settings)
        }
        23 | 32 => {
            let updated = with_state(|st| st.update_prefs(&key, |p| p.chat_mentions = !p.chat_mentions));
            save_current_db();
            let pitch = if updated.chat_mentions { 1.4 } else { 0.95 };
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.8, pitch);
            Some(MenuPage::Settings)
        }
        24 | 33 => {
            let updated = with_state(|st| st.update_prefs(&key, |p| p.hide_players = !p.hide_players));
            save_current_db();
            apply_visibility_pref(server, player, updated.hide_players);
            let pitch = if updated.hide_players { 1.4 } else { 0.95 };
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.8, pitch);
            Some(MenuPage::Settings)
        }
        25 | 34 => {
            let updated = with_state(|st| st.update_prefs(&key, |p| p.auto_compass = !p.auto_compass));
            save_current_db();
            if updated.auto_compass {
                let _ = give_menu_compass_non_destructive(player);
            }
            let pitch = if updated.auto_compass { 1.4 } else { 0.95 };
            player.play_custom_sound("minecraft:block.note_block.pling", SoundCategory::Ui, 0.8, pitch);
            Some(MenuPage::Settings)
        }
        _ => None,
    }
}
