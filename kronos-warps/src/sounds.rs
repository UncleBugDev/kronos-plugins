use pumpkin_plugin_api::{wit::pumpkin::plugin::sounds::SoundCategory, Player};

pub const SOUND_TELEPORT: &str = "minecraft:entity.enderman.teleport";
pub const SOUND_WARMUP_TICK: &str = "minecraft:block.note_block.hat";
pub const SOUND_SUCCESS: &str = "minecraft:block.note_block.pling";
pub const SOUND_ERROR: &str = "minecraft:entity.villager.no";
pub const SOUND_CLICK: &str = "minecraft:ui.button.click";
pub const SOUND_COIN: &str = "minecraft:entity.experience_orb.pickup";

pub fn play_teleport_sound(player: &Player) {
    player.play_custom_sound(SOUND_TELEPORT, SoundCategory::Players, 1.0, 1.0);
}

pub fn play_warmup_tick_sound(player: &Player, seconds_left: u64) {
    let pitch = match seconds_left {
        1 => 1.5,
        2 => 1.2,
        _ => 1.0,
    };
    player.play_custom_sound(SOUND_WARMUP_TICK, SoundCategory::Players, 0.75, pitch);
}

pub fn play_success_sound(player: &Player) {
    player.play_custom_sound(SOUND_SUCCESS, SoundCategory::Players, 0.9, 1.4);
}

pub fn play_error_sound(player: &Player) {
    player.play_custom_sound(SOUND_ERROR, SoundCategory::Players, 0.9, 0.9);
}

pub fn play_click_sound(player: &Player) {
    player.play_custom_sound(SOUND_CLICK, SoundCategory::Players, 0.7, 1.15);
}

pub fn play_coin_sound(player: &Player) {
    player.play_custom_sound(SOUND_COIN, SoundCategory::Players, 0.9, 1.1);
}
