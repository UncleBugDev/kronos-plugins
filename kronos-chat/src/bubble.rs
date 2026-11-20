use pumpkin_plugin_api::{
    text::TextComponent,
    wit::pumpkin::plugin::entity_types::EntityType,
    Player, Server,
};

use crate::filter::sanitize_input;
use crate::models::{normalize_key, ActiveBubble};
use crate::storage::{now_ms, with_state};

const BUBBLE_Y_OFFSET: f64 = 2.25;

/// Форматирует реплику для облачка над головой:
/// очищает от тегов/управляющих символов, обрезает до `max_chars` (40) с `...` и оборачивает в `§e«§f{Текст}§e»`.
pub fn format_bubble_text(raw: &str, max_chars: usize) -> String {
    let clean = sanitize_input(raw);
    let chars: Vec<char> = clean.chars().collect();
    let truncated = if chars.len() > max_chars {
        let keep = max_chars.saturating_sub(3);
        let head: String = chars.into_iter().take(keep).collect();
        format!("{}...", head.trim_end())
    } else {
        clean
    };
    format!("§e«§f{truncated}§e»")
}

/// Создаёт (или обновляет) облачко текста над головой игрока на 4.5 секунды.
pub fn spawn_chat_bubble(player: &Player, server: &Server, clean_text: &str) {
    let p_name = player.get_name();
    let p_key = normalize_key(&p_name);

    let (prev_bubble, duration_ms, max_chars) = with_state(|st| {
        (
            st.active_bubbles.remove(&p_key),
            st.store.config.bubble_duration_ms,
            st.store.config.bubble_max_chars,
        )
    });

    if let Some(old) = prev_bubble {
        despawn_bubble_entity(server, &old);
    }

    let formatted = format_bubble_text(clean_text, max_chars);
    let pos = player.get_position();
    let world = player.get_world();
    let world_name = world.get_name();

    let entity = world.spawn_entity(EntityType::ArmorStand, (pos.0, pos.1 + BUBBLE_Y_OFFSET, pos.2));
    entity.set_invisible(true);
    entity.set_has_gravity(false);
    entity.set_invulnerable(true);
    entity.set_silent(true);
    entity.set_custom_name(TextComponent::from_legacy_string(&formatted));
    entity.set_custom_name_visible(true);

    let entity_id = entity.get_id() as i32;
    let expires_at_ms = now_ms().saturating_add(duration_ms);

    with_state(|st| {
        st.active_bubbles.insert(
            p_key.clone(),
            ActiveBubble {
                player_key: p_key,
                entity_id,
                world_name,
                expires_at_ms,
            },
        );
    });
}

/// Обновляет позиции активных облачков (следование за игроком) и удаляет истёкшие по `ServerTickEndEvent`.
pub fn tick_bubbles(server: &Server) {
    let snapshot: Vec<ActiveBubble> =
        with_state(|st| st.active_bubbles.values().cloned().collect());
    if snapshot.is_empty() {
        return;
    }

    let now = now_ms();
    let online = server.get_all_players();

    for bubble in snapshot {
        if now >= bubble.expires_at_ms {
            with_state(|st| {
                st.active_bubbles.remove(&bubble.player_key);
            });
            despawn_bubble_entity(server, &bubble);
            continue;
        }

        let Some(player) = online
            .iter()
            .find(|p| normalize_key(&p.get_name()) == bubble.player_key)
        else {
            with_state(|st| {
                st.active_bubbles.remove(&bubble.player_key);
            });
            despawn_bubble_entity(server, &bubble);
            continue;
        };

        let p_world = player.get_world();
        if p_world.get_name() != bubble.world_name {
            with_state(|st| {
                st.active_bubbles.remove(&bubble.player_key);
            });
            despawn_bubble_entity(server, &bubble);
            continue;
        }

        let pos = player.get_position();
        if let Some(entity) = p_world
            .get_entities()
            .into_iter()
            .find(|e| e.get_id() as i32 == bubble.entity_id)
        {
            entity.teleport((pos.0, pos.1 + BUBBLE_Y_OFFSET, pos.2), p_world);
        }
    }
}

pub fn remove_player_bubble(server: &Server, player_key: &str) {
    let removed = with_state(|st| st.active_bubbles.remove(player_key));
    if let Some(bubble) = removed {
        despawn_bubble_entity(server, &bubble);
    }
}

fn despawn_bubble_entity(server: &Server, bubble: &ActiveBubble) {
    if let Some(world) = server.get_world_by_name(&bubble.world_name) {
        if let Some(entity) = world
            .get_entities()
            .into_iter()
            .find(|e| e.get_id() as i32 == bubble.entity_id)
        {
            entity.remove();
        }
    }
}
