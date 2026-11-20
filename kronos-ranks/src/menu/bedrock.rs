use pumpkin_plugin_api::{
    wit::pumpkin::plugin::forms::{Form, SimpleForm, SimpleFormButton},
    Player,
};

use crate::models::{
    format_remaining_ru, normalize_key, BedrockRanksAction, PendingBedrockForm, PermissionTree,
    PlayerRankData,
};
use crate::storage::with_state;
use crate::tab_chat::legacy;

pub fn open_ranks_form(
    player: &Player,
    tree: &PermissionTree,
    viewer: &PlayerRankData,
    now_ms: u64,
) {
    let Some(bedrock) = player.as_bedrock() else {
        return;
    };

    let donate_ranks = tree.donate_ranks_sorted();
    let mut buttons = Vec::with_capacity(donate_ranks.len() + 1);
    let mut actions = Vec::with_capacity(donate_ranks.len() + 1);

    for rank in &donate_ranks {
        let active = if viewer.donate_rank == rank.id { " §a(Ваш)" } else { "" };
        buttons.push(SimpleFormButton {
            text: legacy(&format!(
                "{}{}\n§7{} §8| §a{} домов",
                rank.prefix.trim(),
                active,
                rank.summary,
                rank.max_homes
            )),
            image: None,
        });
        actions.push(BedrockRanksAction::ShowRankDetails(rank.id.clone()));
    }

    buttons.push(SimpleFormButton {
        text: legacy("§e⟳ Обновить каталог"),
        image: None,
    });
    actions.push(BedrockRanksAction::RefreshCatalog);

    let remaining = format_remaining_ru(viewer.donate_expires_at_ms, now_ms);
    let content = format!(
        "§7Ваш донат-ранг: §e{} §8({remaining})\n§7Нажмите на любой ранг, чтобы увидеть его преимущества и права:",
        viewer.donate_rank.to_uppercase()
    );

    let form = Form::Simple(SimpleForm {
        title: legacy("§6§l✦ РАНГИ KRONOS"),
        content: legacy(&content),
        buttons,
    });

    let form_id = bedrock.open_form(form);
    let player_key = normalize_key(&player.get_name());
    with_state(|st| {
        st.pending_forms.insert(form_id, PendingBedrockForm { player_key, actions });
    });
}
