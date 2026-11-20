pub mod bedrock;
pub mod java;

use pumpkin_plugin_api::{text::TextComponent, Player};

use crate::models::{PermissionTree, PlayerRankData};
use crate::sounds::play_click_sound;
use crate::tab_chat::legacy;

pub const LINE_SEP: &str = "§8§m────────────────────────────────────────────";

pub fn clickable_run(label: &str, command: &str, hover: &str) -> TextComponent {
    legacy(label)
        .click_run_command(command)
        .hover_show_text(legacy(hover))
}

pub fn clickable_suggest(label: &str, command: &str, hover: &str) -> TextComponent {
    legacy(label)
        .click_suggest_command(command)
        .hover_show_text(legacy(hover))
}

/// Открывает каталог донат-рангов `/ranks` в двойном UI (Java чат + Bedrock `SimpleForm`).
pub fn open_ranks_catalog(
    player: &Player,
    tree: &PermissionTree,
    viewer: &PlayerRankData,
    now_ms: u64,
) {
    play_click_sound(player);
    if player.as_bedrock().is_some() {
        bedrock::open_ranks_form(player, tree, viewer, now_ms);
    }
    java::send_ranks_chat(player, tree, viewer, now_ms);
}
