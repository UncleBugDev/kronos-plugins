use pumpkin_plugin_api::Player;

use crate::menu::{clickable_run, clickable_suggest, LINE_SEP};
use crate::models::{format_remaining_ru, PermissionTree, PlayerRankData, Rank};
use crate::tab_chat::legacy;

pub fn send_ranks_chat(
    player: &Player,
    tree: &PermissionTree,
    viewer: &PlayerRankData,
    now_ms: u64,
) {
    let donate_ranks = tree.donate_ranks_sorted();
    let remaining = format_remaining_ru(viewer.donate_expires_at_ms, now_ms);

    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§6§l✦ КАТАЛОГ ДОНАТ-РАНГОВ KRONOS §8| §7Ваш: §e{} §8({remaining})",
            viewer.donate_rank.to_uppercase()
        )),
        false,
    );

    for rank in &donate_ranks {
        let is_current = viewer.donate_rank == rank.id;
        let mark = if is_current { "§a✔ " } else { "§6▸ " };
        let inherited_count = tree.inherited_permissions(&rank.id).len();
        let mult = format!("{}.{}x", rank.multiplier_x10 / 10, rank.multiplier_x10 % 10);

        let hover = format!(
            "{}§f{} §8(Вес: {})\n§7Преимущества: §e{}\n§7Лимит домов: §a{} §8| §7Множитель: §b{}\n§7Всего прав (с наследованием): §f{}\n\n§7Выдача (авто-донат): §f/rank set {} {} 30d",
            rank.prefix,
            rank.display_name,
            rank.weight,
            rank.summary,
            rank.max_homes,
            mult,
            inherited_count,
            viewer.player_name,
            rank.id
        );

        let mut row = legacy(&format!("  {mark}"))
            .add_child(clickable_run(
                &format!("{}§f{}", rank.prefix, rank.display_name),
                &format!("/rank info {}", viewer.player_name),
                &hover,
            ))
            .add_child(legacy(&format!(" §8— §7{}", rank.summary)));

        if is_current {
            row = row.add_child(legacy(" §a[АКТИВЕН]"));
        }
        player.send_system_message(row, false);
    }

    let footer = legacy(" ")
        .add_child(clickable_run(
            "§e[Мой профиль]",
            "/rank info",
            "§eПоказать ваши текущие ранги, сроки и права",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_suggest(
            "§b[Донат-команды]",
            "/fly",
            "§bДоступные команды по рангу: /feed, /craft, /fly, /hat, /heal, /ec",
        ));

    player.send_system_message(footer, false);
    player.send_system_message(legacy(LINE_SEP), false);
}

pub fn send_rank_detail_card(player: &Player, tree: &PermissionTree, rank: &Rank) {
    let perms = tree.inherited_permissions(&rank.id);
    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "{}§f{} §8(ID: {}, вес: {})",
            rank.prefix, rank.display_name, rank.id, rank.weight
        )),
        false,
    );
    player.send_system_message(legacy(&format!(" §7Описание: §e{}", rank.summary)), false);
    player.send_system_message(
        legacy(&format!(
            " §7Дома: §a{} §8| §7Всего наследуемых прав: §b{}",
            rank.max_homes,
            perms.len()
        )),
        false,
    );
    player.send_system_message(legacy(LINE_SEP), false);
}
