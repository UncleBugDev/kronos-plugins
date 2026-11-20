use pumpkin_plugin_api::Player;

use crate::menu::{clickable_run, clickable_suggest, legacy, LINE_SEP};
use crate::models::{format_cost_minor, CatalogTab, Warp, WarpType};

/// Отправляет интерактивный каталог `/warps` для Java Edition с hover-подсказками и кнопкой `[Телепорт]`.
pub fn send_catalog_chat(
    player: &Player,
    tab: CatalogTab,
    server_warps: &[Warp],
    popular_warps: &[Warp],
    my_warps: &[Warp],
    is_admin: bool,
) {
    let (title, active_list) = match tab {
        CatalogTab::Server => ("СЕРВЕРНЫЕ ВАРПЫ", server_warps),
        CatalogTab::Popular => ("ПОПУЛЯРНЫЕ ВАРПЫ ИГРОКОВ", popular_warps),
        CatalogTab::Mine => ("МОИ ВАРПЫ", my_warps),
    };

    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!("§6§l✦ КАТАЛОГ ВАРПОВ §8| §e{title}")),
        false,
    );

    let server_lbl = tab_label("Серверные", tab == CatalogTab::Server, server_warps.len());
    let pop_lbl = tab_label("Популярные", tab == CatalogTab::Popular, popular_warps.len());
    let my_lbl = tab_label("Мои", tab == CatalogTab::Mine, my_warps.len());

    let tabs_row = legacy(" ")
        .add_child(clickable_run(
            &server_lbl,
            "/warps server",
            "§6Показать официальные серверные варпы",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            &pop_lbl,
            "/warps popular",
            "§bТоп пользовательских варпов по посещениям",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            &my_lbl,
            "/warps mine",
            "§aПоказать ваши личные варпы (/pwarp)",
        ));
    player.send_system_message(tabs_row, false);

    if active_list.is_empty() {
        player.send_system_message(legacy(" §7В этом разделе пока нет варпов."), false);
    } else {
        for warp in active_list.iter().take(12) {
            send_warp_card(player, warp);
        }
    }

    let mut footer = legacy(" ")
        .add_child(clickable_suggest(
            "§a[+ Создать /pwarp]",
            "/pwarp create ",
            "§aСоздать свой публичный варп: /pwarp create <имя> [цена]",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_suggest(
            "§e[Цена входа]",
            "/pwarp setcost ",
            "§eИзменить цену входа: /pwarp setcost <имя> <сумма>",
        ));

    if is_admin {
        footer = footer
            .add_child(legacy(" "))
            .add_child(clickable_suggest(
                "§c[⚙ /setwarp]",
                "/setwarp ",
                "§cСоздать серверный варп: /setwarp <имя> [описание] [право]",
            ));
    }

    player.send_system_message(footer, false);
    player.send_system_message(legacy(LINE_SEP), false);
}

pub fn send_warp_card(player: &Player, warp: &Warp) {
    let icon = match warp.warp_type {
        WarpType::Server => "§6★",
        WarpType::Player => "§b◈",
    };
    let cost_label = format_cost_minor(warp.cost_minor);
    let perm_line = warp
        .permission
        .as_deref()
        .map(|p| format!("\n§7Доступ: §c{p}"))
        .unwrap_or_default();

    let hover = format!(
        "§6§l{}§r\n§7Описание: §f{}\n§7Владелец: §e{}\n§7Мир: §a{}\n§7Координаты: §f{:.1}, {:.1}, {:.1}\n§7Посещений: §b{}\n§7Цена входа: §e{}{}",
        warp.name,
        if warp.description.is_empty() { "—" } else { &warp.description },
        warp.owner,
        warp.location.world,
        warp.location.x,
        warp.location.y,
        warp.location.z,
        warp.visits,
        cost_label,
        perm_line
    );

    let price_tag = if warp.cost_minor > 0 {
        format!(" §8| §e{}", cost_label)
    } else {
        String::new()
    };

    let row = legacy(&format!("  {} ", icon))
        .add_child(legacy(&format!("§f§l{}", warp.name)).hover_show_text(legacy(&hover)))
        .add_child(legacy(&format!(
            " §8(§7{}§8) §7• §b{} визитов{} ",
            warp.owner, warp.visits, price_tag
        )))
        .add_child(clickable_run(
            "§a§l[Телепорт]",
            &format!("/warp {}", warp.name),
            &format!("§aНажмите, чтобы телепортироваться на §f{}", warp.name),
        ));

    player.send_system_message(row, false);
}

fn tab_label(name: &str, active: bool, count: usize) -> String {
    if active {
        format!("§e§l[{name}: {count}]")
    } else {
        format!("§7[{name}: {count}]")
    }
}
