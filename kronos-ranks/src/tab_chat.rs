use pumpkin_plugin_api::{text::TextComponent, Player, Server};

use crate::commands::{PERM_ADMIN, PERM_STAFF};
use crate::models::{normalize_key, PermissionTree, PlayerRankData};
use crate::storage::with_state;

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

/// Возвращает форматированное имя в табе: `{prefix}{name_color}{player_name}`.
pub fn build_tab_name(tree: &PermissionTree, user: &PlayerRankData) -> String {
    let primary = tree.primary_rank(user);
    format!("{}{}{}", primary.prefix, primary.name_color, user.player_name)
}

/// Вес сортировки в таб-листе (чем выше вес, тем выше игрок в списке).
pub fn tab_sort_order(tree: &PermissionTree, user: &PlayerRankData) -> i32 {
    tree.primary_rank(user).weight as i32
}

/// Применяет разрешённые `&`-коды чата согласно правам ранга игрока.
pub fn format_chat_message(tree: &PermissionTree, user: &PlayerRankData, raw_msg: &str) -> String {
    let allow_all_colors = tree.has_permission(user, "kronos-ranks:chat.color.all");
    let allow_styles = tree.has_permission(user, "kronos-ranks:chat.style.all");
    let allow_vip_colors = tree.has_permission(user, "kronos-ranks:chat.color.vip");

    let mut out = String::with_capacity(raw_msg.len());
    let mut chars = raw_msg.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(&next) = chars.peek() {
                let low = next.to_ascii_lowercase();
                let is_color = matches!(low, '0'..='9' | 'a'..='f');
                let is_style = matches!(low, 'l' | 'm' | 'n' | 'o' | 'r');
                let is_vip = matches!(low, 'a' | 'b');
                if (is_color && allow_all_colors)
                    || (is_vip && allow_vip_colors)
                    || (is_style && allow_styles)
                {
                    out.push('§');
                    out.push(chars.next().unwrap());
                    continue;
                }
            }
        }
        out.push(c);
    }

    let primary = tree.primary_rank(user);
    format!(
        "{}{}{} §8» §f{}",
        primary.prefix, primary.name_color, user.player_name, out
    )
}

pub fn format_staff_chat_line(tree: &PermissionTree, user: &PlayerRankData, message: &str) -> String {
    let primary = tree.primary_rank(user);
    format!(
        "§c§l[SC] §r{}{}{} §8» §e{}",
        primary.prefix, primary.name_color, user.player_name, message
    )
}

/// Обновляет таб-лист, порядок сортировки и заголовок/подвал без удержания Mutex.
pub fn refresh_player_tab(
    player: &Player,
    tree: &PermissionTree,
    user: &PlayerRankData,
    online_count: u32,
    max_players: u32,
) {
    let tab_name = build_tab_name(tree, user);
    let order = tab_sort_order(tree, user);
    let primary = tree.primary_rank(user);

    player.set_tab_list_name(Some(legacy(&tab_name)));
    player.set_tab_list_order(order);
    player.set_display_name(legacy(&tab_name));

    let header = "§6§l✦ KRONOS NETWORK §8| §ePumpkin 26.3";
    let footer = format!(
        "§7Ранг: {} §8| §7Онлайн: §e{}/{}",
        primary.prefix.trim(),
        online_count,
        max_players
    );
    player.set_tab_list_header_footer(legacy(header), legacy(&footer));
}

/// Синхронизирует эффективные права игрока (включая наследование) в нативную таблицу прав Pumpkin.
pub fn sync_player_permissions(player: &Player, tree: &PermissionTree, user: &PlayerRankData) {
    let is_staff = tree.is_staff(user);
    let is_admin = tree.has_permission(user, PERM_ADMIN);

    if is_staff {
        player.set_permission(PERM_STAFF, true);
    } else if player.has_permission_set(PERM_STAFF) == Some(true) {
        player.unset_permission(PERM_STAFF);
    }

    if is_admin {
        player.set_permission(PERM_ADMIN, true);
    } else if player.has_permission_set(PERM_ADMIN) == Some(true) {
        player.unset_permission(PERM_ADMIN);
    }

    for node in tree.effective_permissions(user) {
        if !node.contains('*') {
            player.set_permission(&node, true);
        }
    }
    for denied in &user.denied_permissions {
        if !denied.contains('*') {
            player.set_permission(denied, false);
        }
    }
}

pub fn find_online_player_ci(server: &Server, target_name: &str) -> Option<Player> {
    let key = normalize_key(target_name);
    server
        .get_all_players()
        .into_iter()
        .find(|p| normalize_key(&p.get_name()) == key)
}

pub fn refresh_online_player(server: &Server, target_name: &str) {
    let Some(player) = find_online_player_ci(server, target_name) else {
        return;
    };
    let (tree, user) = with_state(|st| (st.tree.clone(), st.users.get_or_default(target_name)));
    let online = server.get_player_count().max(1);
    let max_players = server.get_max_players();
    sync_player_permissions(&player, &tree, &user);
    refresh_player_tab(&player, &tree, &user, online, max_players);
}
