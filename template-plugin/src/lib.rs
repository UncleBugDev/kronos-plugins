use std::fs;
use std::sync::{Mutex, OnceLock};

use pumpkin_plugin_api::{
    Context, Player, Plugin, PluginMetadata, Server,
    command::{
        ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType,
    },
    commands::CommandHandler,
    events::{EventHandler, EventPriority, PlayerJoinEvent},
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    scheduler::SchedulerExt,
    text::TextComponent,
    wit::pumpkin::plugin::{command::Arg, event::PlayerJoinEventData},
};
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Constants & Permissions
// -----------------------------------------------------------------------------
const PERM_USE: &str = "template-plugin:use";
const PERM_ADMIN: &str = "template-plugin:admin";

// -----------------------------------------------------------------------------
// Persistent State
// -----------------------------------------------------------------------------
#[derive(Default, Serialize, Deserialize)]
struct PluginConfig {
    welcome_message: String,
    counter: u64,
}

struct PluginState {
    data_folder: String,
    config: PluginConfig,
}

static STATE: OnceLock<Mutex<PluginState>> = OnceLock::new();

fn with_state<R>(f: impl FnOnce(&mut PluginState) -> R) -> R {
    let mutex = STATE.get_or_init(|| {
        Mutex::new(PluginState {
            data_folder: ".".to_string(),
            config: PluginConfig {
                welcome_message: "§6§l✦ KRONOS §8» §aДобро пожаловать на сервер!".to_string(),
                counter: 0,
            },
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

fn init_state(data_folder: String) {
    let _ = fs::create_dir_all(&data_folder);
    let path = format!("{data_folder}/config.json");
    let config = match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<PluginConfig>(&raw).unwrap_or_default(),
        Err(_) => PluginConfig {
            welcome_message: "§6§l✦ KRONOS §8» §aДобро пожаловать на сервер!".to_string(),
            counter: 0,
        },
    };
    let _ = STATE.set(Mutex::new(PluginState { data_folder, config }));
}

fn save_state() {
    with_state(|st| {
        let _ = fs::create_dir_all(&st.data_folder);
        let path = format!("{}/config.json", st.data_folder);
        if let Ok(json) = serde_json::to_string_pretty(&st.config) {
            let _ = fs::write(path, json);
        }
    });
}

// -----------------------------------------------------------------------------
// Helper Utilities
// -----------------------------------------------------------------------------
fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

// -----------------------------------------------------------------------------
// Event Handlers
// -----------------------------------------------------------------------------
struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, _server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let msg = with_state(|st| {
            st.config.counter += 1;
            st.config.welcome_message.clone()
        });
        save_state();

        let player = &event.player;
        player.send_system_message(legacy(&msg), false);

        // Safe vanilla 1.20.1 sound (avoid play_sound trap):
        player.play_custom_sound(
            "minecraft:entity.experience_orb.pickup",
            pumpkin_plugin_api::wit::pumpkin::plugin::sounds::SoundCategory::Master,
            1.0,
            1.0,
        );

        event
    }
}

// -----------------------------------------------------------------------------
// Commands
// -----------------------------------------------------------------------------
struct TemplateCmd;
impl CommandHandler for TemplateCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let sub = match args.get_value("sub") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };

        if sub == "stats" {
            let count = with_state(|st| st.config.counter);
            sender.send_message(legacy(&format!(
                "§6§l✦ KRONOS §8» §7Всего входов: §e{count}"
            )));
            return Ok(0);
        }

        sender.send_message(legacy(
            "§6§l✦ Template Plugin §8» §aПлагин работает успешно! Используйте: §e/template stats",
        ));
        Ok(0)
    }
}

// -----------------------------------------------------------------------------
// Repeating Tasks
// -----------------------------------------------------------------------------
fn repeating_tick_task(_server: Server) {
    // Executes periodically every N ticks outside player context
}

// -----------------------------------------------------------------------------
// Plugin Entry Point
// -----------------------------------------------------------------------------
pub struct TemplatePlugin;

impl Plugin for TemplatePlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "template-plugin".into(),
            version: "0.1.0".into(),
            authors: vec!["UncleBug".into()],
            description: "Starter template plugin for KRONOS Minecraft server".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_state(context.get_data_folder());

        // 1. Register Permissions
        context.register_permission(&Permission {
            node: PERM_USE.to_string(),
            description: "Use template-plugin commands".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        context.register_permission(&Permission {
            node: PERM_ADMIN.to_string(),
            description: "Administer template-plugin".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: vec![],
        })?;

        // 2. Register Commands
        let cmd = Command::new(&["template".into(), "tmpl".into()], "Template command")
            .execute(TemplateCmd)
            .then(
                CommandNode::argument("sub", &ArgumentType::String(StringType::SingleWord))
                    .execute(TemplateCmd),
            );
        context.register_command(cmd, PERM_USE);

        // 3. Register Events
        context.register_event_handler::<PlayerJoinEvent, _>(
            JoinHandler,
            EventPriority::Normal,
            false,
        )?;

        // 4. Schedule Tasks
        context.schedule_repeating_task(20, 20, repeating_tick_task);

        tracing::info!("Template Plugin v0.1.0 initialized!");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_state();
        tracing::info!("Template Plugin unloaded.");
        Ok(())
    }
}

register_plugin!(TemplatePlugin);
