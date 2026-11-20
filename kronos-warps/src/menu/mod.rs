pub mod bedrock;
pub mod java;

use pumpkin_plugin_api::{text::TextComponent, Player};

use crate::models::{CatalogTab, Warp};
use crate::sounds::play_click_sound;

pub const LINE_SEP: &str = "§8§m────────────────────────────────────────────";

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

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

/// Открывает каталог `/warps` с автоматическим выбором двойного UI:
/// Bedrock-игрокам открывается нативная форма `SimpleForm`, Java-игрокам — чат-карточки.
pub fn open_warps_catalog(
    player: &Player,
    tab: CatalogTab,
    server_warps: &[Warp],
    popular_warps: &[Warp],
    my_warps: &[Warp],
    is_admin: bool,
) {
    play_click_sound(player);

    if player.as_bedrock().is_some() {
        bedrock::open_catalog_form(player, tab, server_warps, popular_warps, my_warps);
    }

    java::send_catalog_chat(
        player,
        tab,
        server_warps,
        popular_warps,
        my_warps,
        is_admin,
    );
}
