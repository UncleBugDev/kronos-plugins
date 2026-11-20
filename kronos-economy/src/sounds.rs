use pumpkin_plugin_api::{wit::pumpkin::plugin::sounds::SoundCategory, Player};

/// Звук успешного перевода / начисления монет (1.20.1).
pub fn play_coin_sound(player: &Player) {
    player.play_custom_sound(
        "minecraft:entity.experience_orb.pickup",
        SoundCategory::Players,
        1.0,
        1.0,
    );
}

/// Звук ошибки / отказа операции (1.20.1).
pub fn play_error_sound(player: &Player) {
    player.play_custom_sound(
        "minecraft:entity.villager.no",
        SoundCategory::Players,
        1.0,
        1.0,
    );
}

/// Звук навигации по меню (1.20.1).
pub fn play_click_sound(player: &Player) {
    player.play_custom_sound(
        "minecraft:ui.button.click",
        SoundCategory::Players,
        0.8,
        1.1,
    );
}
