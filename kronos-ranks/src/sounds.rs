use pumpkin_plugin_api::{wit::pumpkin::plugin::sounds::SoundCategory, Player};

pub const SOUND_SUCCESS: &str = "minecraft:block.note_block.pling";
pub const SOUND_RANK_UP: &str = "minecraft:entity.player.levelup";
pub const SOUND_ERROR: &str = "minecraft:entity.villager.no";
pub const SOUND_CLICK: &str = "minecraft:ui.button.click";
pub const SOUND_CHIME: &str = "minecraft:block.note_block.chime";

pub fn play_success_sound(player: &Player) {
    player.play_custom_sound(SOUND_SUCCESS, SoundCategory::Players, 0.9, 1.4);
}

pub fn play_rank_up_sound(player: &Player) {
    player.play_custom_sound(SOUND_RANK_UP, SoundCategory::Players, 1.0, 1.15);
}

pub fn play_error_sound(player: &Player) {
    player.play_custom_sound(SOUND_ERROR, SoundCategory::Players, 0.9, 0.9);
}

pub fn play_click_sound(player: &Player) {
    player.play_custom_sound(SOUND_CLICK, SoundCategory::Players, 0.7, 1.2);
}

pub fn play_chime_sound(player: &Player) {
    player.play_custom_sound(SOUND_CHIME, SoundCategory::Players, 0.9, 1.25);
}
