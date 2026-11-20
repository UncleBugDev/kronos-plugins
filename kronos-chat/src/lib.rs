pub mod bubble;
pub mod commands;
pub mod filter;
pub mod models;
pub mod sounds;
pub mod storage;

use pumpkin_plugin_api::{
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin, Context, Plugin, PluginMetadata,
};

use crate::commands::{
    PERM_BYPASS_COOLDOWN, PERM_BYPASS_FILTER, PERM_CHAT_COLOR, PERM_SOCIALSPY, PERM_USE,
};
use crate::storage::{init_storage, save_current_store};

pub struct KronosChatPlugin;

impl Plugin for KronosChatPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kronos-chat".into(),
            version: "1.0.0".into(),
            authors: vec!["KRONOS Team".into()],
            description: "Local/Global/Trade chat channels, private messages, anti-spam, injection guard, and overhead speech bubbles for Pumpkin 26.3".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_storage(context.get_data_folder());

        for (node, desc, default) in [
            (PERM_USE, "Use chat channels, /msg, /r, /trade and /ignore", PermissionDefault::Allow),
            (PERM_CHAT_COLOR, "Use &-color codes in chat", PermissionDefault::Op(PermissionLevel::One)),
            (PERM_BYPASS_COOLDOWN, "Bypass global and trade chat cooldowns", PermissionDefault::Op(PermissionLevel::Two)),
            (PERM_BYPASS_FILTER, "Bypass anti-caps, anti-flood and ad filters", PermissionDefault::Op(PermissionLevel::Two)),
            (PERM_SOCIALSPY, "Monitor private messages via /socialspy", PermissionDefault::Op(PermissionLevel::Two)),
        ] {
            context.register_permission(&Permission {
                node: node.to_string(),
                description: desc.to_string(),
                default,
                children: vec![],
            })?;
        }

        commands::register_all(&context);
        commands::register_events(&context)?;

        tracing::info!("KRONOS Chat v1.0.0 loaded (Local/Global/Trade, PM, Anti-Spam, Speech Bubbles).");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_current_store();
        tracing::info!("KRONOS Chat unloaded: chat_data.json saved.");
        Ok(())
    }
}

register_plugin!(KronosChatPlugin);
#[cfg(test)] mod tests;
