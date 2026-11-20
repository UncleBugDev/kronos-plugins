pub mod commands;
pub mod menu;
pub mod models;
pub mod sounds;
pub mod storage;
pub mod tab_chat;

use pumpkin_plugin_api::{
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin, Context, Plugin, PluginMetadata,
};

use crate::commands::{PERM_ADMIN, PERM_PLAYER, PERM_STAFF};
use crate::storage::{init_storage, save_all_stores};

pub struct KronosRanksPlugin;

impl Plugin for KronosRanksPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kronos-ranks".into(),
            version: "2.0.0".into(),
            authors: vec!["KRONOS Team".into()],
            description: "Modular Dual-Track rank hierarchy, Russian prefixes, permission inheritance, temporary ranks, /sc, and donator commands for Pumpkin 26.3".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_storage(context.get_data_folder());

        for (node, desc, default) in [
            (PERM_PLAYER, "Access to /ranks catalog and basic rank info", PermissionDefault::Allow),
            (PERM_STAFF, "Access to staff chat /sc", PermissionDefault::Op(PermissionLevel::Two)),
            (PERM_ADMIN, "Manage ranks and permissions via /rank", PermissionDefault::Op(PermissionLevel::Three)),
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

        tracing::info!("KRONOS Ranks v2.0.0 loaded (12 Dual-Track ranks, inheritance, wasm32-wasip2).");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_all_stores();
        tracing::info!("KRONOS Ranks unloaded: ranks.json and users.json saved.");
        Ok(())
    }
}

register_plugin!(KronosRanksPlugin);
#[cfg(test)] mod tests;
