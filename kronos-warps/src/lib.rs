pub mod commands;
pub mod menu;
pub mod models;
pub mod sounds;
pub mod storage;

use pumpkin_plugin_api::{
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin, Context, Plugin, PluginMetadata,
};

use crate::commands::{
    PERM_ADMIN, PERM_BYPASS_WARMUP, PERM_PWARP_CREATE, PERM_PWARP_ELITE, PERM_USE, PERM_VIP,
};
use crate::storage::{init_storage, save_current_store};

pub struct KronosWarpsPlugin;

impl Plugin for KronosWarpsPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kronos-warps".into(),
            version: "1.0.0".into(),
            authors: vec!["KRONOS Team".into()],
            description: "Server and player warps with 3s warmup, paid visits via kronos-economy, and Java/Bedrock UI for Pumpkin 26.3".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_storage(context.get_data_folder());

        for (node, desc, default) in [
            (PERM_USE, "Teleport to warps and open /warps catalog", PermissionDefault::Allow),
            (PERM_PWARP_CREATE, "Create and manage personal /pwarp locations", PermissionDefault::Allow),
            (PERM_VIP, "Access VIP warps and up to 3 personal warps", PermissionDefault::Op(PermissionLevel::One)),
            (PERM_PWARP_ELITE, "Create up to 7 personal warps", PermissionDefault::Op(PermissionLevel::Two)),
            (PERM_BYPASS_WARMUP, "Bypass 3-second warp teleport warmup", PermissionDefault::Op(PermissionLevel::Two)),
            (PERM_ADMIN, "Create and delete server warps (/setwarp, /delwarp)", PermissionDefault::Op(PermissionLevel::Three)),
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

        tracing::info!("KRONOS Warps v1.0.0 loaded (wasm32-wasip2, warmup 3s, Java/Bedrock UI).");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_current_store();
        tracing::info!("KRONOS Warps unloaded: warps.json saved.");
        Ok(())
    }
}

register_plugin!(KronosWarpsPlugin);
#[cfg(test)] mod tests;
