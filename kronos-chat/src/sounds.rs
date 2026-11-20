use pumpkin_plugin_api::{wit::pumpkin::plugin::sounds::SoundCategory, Player};

pub const SOUND_TRADE: &str = "minecraft:entity.experience_orb.pickup";
pub const SOUND_PM_BELL: &str = "minecraft:block.note_block.bell";
pub const SOUND_ERROR: &str = "minecraft:entity.villager.no";
pub const SOUND_SUCCESS: &str = "minecraft:block.note_block.pling";

pub fn play_trade_sound(player: &Player) {
    player.play_custom_sound(SOUND_TRADE, SoundCategory::Players, 0.85, 1.15);
}

pub fn play_pm_sound(player: &Player) {
    player.play_custom_sound(SOUND_PM_BELL, SoundCategory::Players, 1.0, 1.2);
}

pub fn play_error_sound(player: &Player) {
    player.play_custom_sound(SOUND_ERROR, SoundCategory::Players, 0.9, 0.9);
}

pub fn play_success_sound(player: &Player) {
    player.play_custom_sound(SOUND_SUCCESS, SoundCategory::Players, 0.85, 1.35);
}
