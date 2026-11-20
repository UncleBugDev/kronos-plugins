use pumpkin_plugin_api::{
    wit::pumpkin::plugin::forms::{Form, SimpleForm, SimpleFormButton},
    Player,
};

use crate::menu::legacy;
use crate::models::{
    format_cost_minor, normalize_key, BedrockWarpAction, CatalogTab, PendingBedrockWarpForm, Warp,
};
use crate::storage::with_state;

/// Открывает нативную форму `SimpleForm` для Bedrock Edition с тремя вкладками:
/// «Серверные варпы», «Популярные варпы игроков» и «Мои варпы».
pub fn open_catalog_form(
    player: &Player,
    active_tab: CatalogTab,
    server_warps: &[Warp],
    popular_warps: &[Warp],
    my_warps: &[Warp],
) {
    let Some(bedrock) = player.as_bedrock() else {
        return;
    };

    let mut buttons = Vec::new();
    let mut actions = Vec::new();

    // Кнопки переключения вкладок
    for (tab, label) in [
        (
            CatalogTab::Server,
            format!("§6★ Серверные варпы ({})", server_warps.len()),
        ),
        (
            CatalogTab::Popular,
            format!("§b◈ Популярные варпы игроков ({})", popular_warps.len()),
        ),
        (
            CatalogTab::Mine,
            format!("§a✦ Мои варпы ({})", my_warps.len()),
        ),
    ] {
        let marker = if tab == active_tab { " §e[Открыто]" } else { "" };
        buttons.push(SimpleFormButton {
            text: legacy(&format!("{label}{marker}")),
            image: None,
        });
        actions.push(BedrockWarpAction::OpenTab(tab));
    }

    let (section_title, items) = match active_tab {
        CatalogTab::Server => ("Серверные варпы", server_warps),
        CatalogTab::Popular => ("Популярные варпы игроков", popular_warps),
        CatalogTab::Mine => ("Мои варпы", my_warps),
    };

    for warp in items.iter().take(15) {
        let cost_text = format_cost_minor(warp.cost_minor);
        buttons.push(SimpleFormButton {
            text: legacy(&format!(
                "§f§l{} §8| §b{} виз.\n§7{} §8• §e{}",
                warp.name, warp.visits, warp.owner, cost_text
            )),
            image: None,
        });
        actions.push(BedrockWarpAction::TeleportTo(warp.name.clone()));
    }

    let content = format!(
        "§7Активная вкладка: §e{section_title}\n§7Нажмите на варп для телепортации или переключите вкладку:"
    );

    let form = Form::Simple(SimpleForm {
        title: legacy("§6§l✦ KRONOS WARPS"),
        content: legacy(&content),
        buttons,
    });

    let form_id = bedrock.open_form(form);
    let player_key = normalize_key(&player.get_name());
    with_state(|st| {
        st.pending_forms.insert(
            form_id,
            PendingBedrockWarpForm {
                player_key,
                actions,
            },
        );
    });
}
