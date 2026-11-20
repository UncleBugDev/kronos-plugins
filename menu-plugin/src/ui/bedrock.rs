use pumpkin_plugin_api::{
    forms::{CustomFormBuilder, SimpleFormBuilder},
    Player,
};

use crate::store::{BedrockFormKind, MenuPage, PlayerPrefs, TimePref, WeatherPref};
use crate::ui::{legacy, ping_colored, ranks::ALL_12_RANKS};

pub fn open_bedrock_menu_page(
    player: &Player,
    page: MenuPage,
    prefs: &PlayerPrefs,
    balance: &str,
) -> Option<(u32, BedrockFormKind)> {
    let bedrock = player.as_bedrock()?;
    match page {
        MenuPage::Main => {
            let pos = player.get_position();
            let content = format!(
                "§7Игрок: §f{} §8│ §7Баланс: §e{}\n§7Пинг: {} §8│ §7XYZ: §f{} {} {}\n§7Погода: {} §8│ §7Время: {}",
                player.get_name(),
                balance,
                ping_colored(player.get_ping()),
                pos.0.floor() as i32,
                pos.1.floor() as i32,
                pos.2.floor() as i32,
                prefs.weather.colored_label(),
                prefs.time.colored_label(),
            );
            let form = SimpleFormBuilder::new(legacy("§6§l✦ МЕНЮ KRONOS ✦"), legacy(&content))
                .button(legacy("§b§l⚡ Телепортация и Дома\n§8/spawn, /rtp, /warps, /homes"), None)
                .button(legacy(&format!("§a§l💰 Экономика и Рынок\n§8Баланс: {balance} • /pay • /baltop")), None)
                .button(legacy("§e§l💬 Чаты и Общение\n§8Локал, !Глобал, $Торг, /msg"), None)
                .button(legacy("§d§l👑 Ранги и Привилегии\n§8Актуальные 12 русских рангов"), None)
                .button(legacy("§e§l⚙ Настройки игрока\n§8Погода, время, HUD, уединение"), None)
                .button(legacy("§6§l📜 Правила и Безопасность\n§8/rules, /auth, /changepass"), None)
                .button(legacy("§b§lℹ О сервере KRONOS\n§8Версии, IP и список плагинов"), None)
                .button(legacy("§6§l🧭 Получить компас меню\n§8Выдать в свободный слот"), None)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::Main))
        }
        MenuPage::Navigation => {
            let content = concat!(
                "§b/spawn §7— мгновенный возврат на спавн\n",
                "§a/rtp §7— безопасный случайный телепорт\n",
                "§6/warps §7— каталог серверных и личных варпов\n",
                "§e/homes §7и §e/sharehome §7— ваши и общие дома"
            );
            let form = SimpleFormBuilder::new(legacy("§b§l⚡ НАВИГАЦИЯ KRONOS"), legacy(content))
                .button(legacy("§b⚡ Телепорт на Спавн сейчас"), None)
                .button(legacy("§e« Назад в главное меню"), None)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::Navigation))
        }
        MenuPage::Economy => {
            let content = format!(
                "§7Ваш баланс: §e§l{balance}\n\n§e/bal §7— карточка баланса\n§a/pay <ник> <сумма> §7— перевод монет\n§6/baltop §7— топ-10 богачей сервера\n§e$текст §7или §e/trade §7— торговый чат (КД 30с)"
            );
            let form = SimpleFormBuilder::new(legacy("§a§l💰 ЭКОНОМИКА KRONOS"), legacy(&content))
                .button(legacy("§e« Назад в главное меню"), None)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::Economy))
        }
        MenuPage::ChatGuide => {
            let content = concat!(
                "§fЛокал: §7обычный текст (100 блоков + облачко над головой)\n",
                "§6Глобал: §e!текст §7(весь сервер, КД 3с)\n",
                "§eТорг: §e$текст §7или §e/trade §7(КД 30с + звук монетки)\n",
                "§dЛС: §d/msg <ник> <текст>§7, §d/r <текст>\n",
                "§cЧС: §c/ignore <ник>"
            );
            let form = SimpleFormBuilder::new(legacy("§e§l💬 ЧАТЫ KRONOS"), legacy(content))
                .button(legacy("§e« Назад в главное меню"), None)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::ChatGuide))
        }
        MenuPage::RanksInfo => {
            let mut body = String::new();
            for r in &ALL_12_RANKS {
                body.push_str(&format!("{} §7— {} §8({})\n", r.badge, r.perks, r.commands));
            }
            let form = SimpleFormBuilder::new(legacy("§d§l👑 12 РАНГОВ KRONOS"), legacy(&body))
                .button(legacy("§e« Назад в главное меню"), None)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::RanksInfo))
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
            let form = CustomFormBuilder::new(legacy("§e§l⚙ НАСТРОЙКИ KRONOS"))
                .dropdown(legacy("§e🌤 Личная погода"), weather_opts, prefs.weather.to_index())
                .dropdown(legacy("§6🕒 Личное время суток"), time_opts, prefs.time.to_index())
                .toggle(legacy("§b🧭 Живой HUD-компас над хотбаром"), prefs.hud_compass)
                .toggle(legacy("§d🔔 Звук упоминаний в чате"), prefs.chat_mentions)
                .toggle(legacy("§a👁 Скрытие игроков (Режим уединения)"), prefs.hide_players)
                .toggle(legacy("§6🎒 Авто-выдача компаса при входе"), prefs.auto_compass)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::Settings))
        }
        MenuPage::Rules => {
            let content = concat!(
                "§61. Честная игра: §fбез читов, X-Ray и макросов.\n",
                "§e2. Уважение в чате: §fбез спама, капса и рекламы.\n",
                "§c3. Без грифа: §fчужие базы и сундуки неприкосновенны.\n",
                "§d4. Без лаг-машин и дюпов.\n",
                "§b5. Безопасность: §f/auth, /changepass, /logout."
            );
            let form = SimpleFormBuilder::new(legacy("§6§l📜 ПРАВИЛА KRONOS"), legacy(content))
                .button(legacy("§b🔐 Памятка безопасности /auth"), None)
                .button(legacy("§e« Назад в главное меню"), None)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::Rules))
        }
        MenuPage::ServerInfo => {
            let content = concat!(
                "§6✦ KRONOS SURVIVAL (Rust / Pumpkin 26.3)\n\n",
                "§7• Java: §a1.20.1 – 26.3 §8| §7Bedrock: §b19132 (UDP)\n",
                "§7• Tailscale IP: §e100.90.47.115\n",
                "§7• Плагины: Essentials, Warps, Economy, Chat, Ranks"
            );
            let form = SimpleFormBuilder::new(legacy("§b§lℹ О СЕРВЕРЕ KRONOS"), legacy(content))
                .button(legacy("§e« Назад в главное меню"), None)
                .build();
            Some((bedrock.open_form(form), BedrockFormKind::ServerInfo))
        }
    }
}
