# KRONOS / Pumpkin Rust WASM Plugin Development Guide for AI & Developers

> **Инструкция для разработчиков и нейросетей (ChatGPT, Claude, Cursor и др.) по созданию высокопроизводительных WASM-плагинов на Rust для Minecraft сервера KRONOS (ядро Pumpkin).**

---

## 🎯 Промпт для загрузки в любую нейросеть (Copy & Paste)

Скопируйте этот блок в начало диалога с нейросетью:

```text
Ты — ведущий Rust-разработчик серверных плагинов для Minecraft сервера KRONOS.
Сервер работает на высокопроизводительном ядре Pumpkin (Rust) и использует модульную систему WASM-плагинов (WASI Preview 2, target wasm32-wasip2).
Сервер поддерживает мультиверсионность (1.20.1–26.3 Java + Bedrock) через связку Velocity + ViaBackwards.

Твоя задача — писать надежные, производительные плагины на Rust строго по правилам архитектуры Pumpkin Plugin API:
1. Крейт компилируется как cdylib под target wasm32-wasip2.
2. Вся бизнес-логика использует trait Plugin, макрос register_plugin!(MyPlugin).
3. Соблюдать правила безопасности мультиверсионности:
   - Звуки: ТОЛЬКО player.play_custom_sound("minecraft:...", ...) со стандартными ID из 1.20.1. НИКОГДА не вызывать raw play_sound.
   - Потокобезопасность: НИКОГДА не держать Mutex<State> во время вызова WIT host-функций (player.open_gui, send_message, schedule и т.д.) во избежание deadlock.
   - GUI/Инвентарь: проверять 0 <= raw_slot < container_size и debounce кликов.
   - Текст: использовать TextComponent::from_legacy_string("§6...").
Пиши чистый, идиоматичный, строго типизированный код без unwrap в рантайме.
```

---

## 🏗 Архитектура плагинов Pumpkin

Плагины в Pumpkin компилируются в WebAssembly под стандарт **WASI Preview 2 (`wasm32-wasip2`)** и выполняются в безопасной песочнице Wasmtime внутри процесса сервера.

### Структура проекта
```
plugins/
├── Cargo.toml               # Корневой workspace
├── AI_PLUGIN_DEV_GUIDE.md  # Этот гайд для ИИ
├── README.md               # Инструкция по сборке и деплою
├── auth-plugin/            # Авторизация, PIN GUI, сессии
├── tab-plugin/             # Кастомный Tab-лист, ранги, TPS
├── menu-plugin/            # Интерактивное меню-компас, настройки, правила
└── template-plugin/        # Чистый шаблон для быстрого старта
```

---

## ⚠️ 5 Золотых правил (Критично для нейросетей!)

### 1. Звуки: только `play_custom_sound`
В текущей версии Wasmtime-рантайма Pumpkin `player.play_sound(Sound::...)` вызывает ловушку переполнения енума на старых клиентах 1.20.1.
* ❌ **НЕЛЬЗЯ**: `player.play_sound(Sound::UiButtonClick, ...)`
* ✅ **НАДО**:
  ```rust
  player.play_custom_sound(
      "minecraft:ui.button.click",
      pumpkin_plugin_api::wit::pumpkin::plugin::sound::SoundCategory::Master,
      1.0,
      1.0,
  );
  ```
  **Безопасные звуки 1.20.1:**
  - `"minecraft:ui.button.click"`
  - `"minecraft:block.note_block.pling"`
  - `"minecraft:block.note_block.chime"`
  - `"minecraft:entity.experience_orb.pickup"`
  - `"minecraft:entity.enderman.teleport"`
  - `"minecraft:entity.villager.no"`
  - `"minecraft:ui.toast.in"`

### 2. Защита от Deadlock при работе с состоянием
`Player`, `Server` и `Context` — это ресурсы WIT-хоста. Если вы залочите `Mutex` состояния и внутри лока вызовете метод игрока, сервер может зависнуть на re-entrant вызове!
* ❌ **НЕЛЬЗЯ**:
  ```rust
  with_state(|st| {
      player.send_message(legacy("hello")); // ОПАСНО!
  });
  ```
* ✅ **НАДО**:
  ```rust
  let msg = with_state(|st| st.get_welcome_message().clone());
  player.send_message(legacy(&msg)); // Безопасно снаружи лока!
  ```

### 3. Пермишены: формат `plugin-name:action`
Имя пермишена ДОЛЖНО начинаться с имени плагина из `metadata.name`:
```rust
const PERM_USE: &str = "my-plugin:use";
context.register_permission(&Permission {
    node: PERM_USE.to_string(),
    description: "Use my plugin".to_string(),
    default: PermissionDefault::Allow, // или PermissionDefault::Op(PermissionLevel::Two)
    children: vec![],
})?;
```

### 4. Мультиверсионность предметов (1.20.1 — 26.3)
В GUI и выдаче предметов используйте ванильные ключи предметов, присутствующие в версии 1.20.1:
- `minecraft:compass`
- `minecraft:book` / `minecraft:writable_book`
- `minecraft:paper`
- `minecraft:nether_star`
- `minecraft:clock`
- `minecraft:iron_sword` / `minecraft:diamond_pickaxe`
- `minecraft:barrier`
- `minecraft:emerald` / `minecraft:gold_ingot`
- Цвета стекол: `minecraft:gray_stained_glass_pane`, `minecraft:lime_stained_glass_pane` и т.д.

### 5. Безопасность кликов в GUI
При обработке `InventoryClickEvent`:
1. Проверяйте, что открыта страница именно вашего плагина.
2. Проверяйте диапазон слотов: `0 <= raw_slot < 54 && raw_slot == slot` (чтобы игрок не кликал в свой личный инвентарь внизу).
3. Добавляйте debounce (например, не чаще 1 клика в 150 мс на игрока), чтобы исключить сетевой спам пакетами.

---

## 📚 Справочник API (Cheat Sheet)

### Регистрация плагина
```rust
use pumpkin_plugin_api::{
    register_plugin, Context, Plugin, PluginMetadata,
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
};

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn new() -> Self { Self }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".into(),
            version: "0.1.0".into(),
            authors: vec!["Author".into()],
            description: "Plugin description".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        // Регистрация прав, команд, событий, тасок
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        Ok(())
    }
}

register_plugin!(MyPlugin);
```

### Регистрация команд
```rust
use pumpkin_plugin_api::{
    command::{ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType},
    commands::CommandHandler, Server, text::TextComponent,
    wit::pumpkin::plugin::command::Arg,
};

struct MyCmd;
impl CommandHandler for MyCmd {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        sender.send_message(TextComponent::from_legacy_string("§aКоманда выполнена!"));
        Ok(0)
    }
}

// В on_load:
let cmd = Command::new(&["mycmd".into(), "mc".into()], "Описание команды")
    .execute(MyCmd)
    .then(
        CommandNode::argument("target", &ArgumentType::String(StringType::Word))
            .execute(MyCmd)
    );
context.register_command(cmd, "my-plugin:use");
```

### Подписка на события
```rust
use pumpkin_plugin_api::events::{EventHandler, EventPriority, PlayerJoinEvent};
use pumpkin_plugin_api::wit::pumpkin::plugin::event::PlayerJoinEventData;

struct JoinListener;
impl EventHandler<PlayerJoinEvent> for JoinListener {
    fn handle(&self, server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let player = &event.player;
        tracing::info!("Player {} joined!", player.get_name());
        event
    }
}

// В on_load:
context.register_event_handler::<PlayerJoinEvent, _>(JoinListener, EventPriority::Normal, false)?;
```

Доступные события:
- `PlayerJoinEvent`
- `PlayerLeaveEvent`
- `PlayerRespawnEvent`
- `PlayerChatEvent`
- `PlayerCommandEvent`
- `BlockPlaceEvent`
- `BlockBreakEvent`
- `InventoryClickEvent`
- `InventoryCloseEvent`
- `ServerLoadEvent`

### Периодические задачи (Scheduler)
```rust
use pumpkin_plugin_api::scheduler::SchedulerExt;

fn my_task(server: Server) {
    let count = server.get_player_count();
    // Фоновое действие каждые N тиков
}

// В on_load: 20 тиков задержка, 20 тиков период (1 секунда)
context.schedule_repeating_task(20, 20, my_task);
```

### Работа с игроком
```rust
player.get_name() -> String
player.get_uuid() -> String
player.get_ping() -> u32
player.get_position() -> (f64, f64, f64)
player.get_world() -> World
player.get_permission_level() -> PermissionLevel
player.has_permission(node: &str) -> bool
player.send_system_message(text, overlay_actionbar)
player.send_title(title, subtitle, fade_in, stay, fade_out)
player.teleport(x, y, z, yaw, pitch)
player.set_tab_list_name(Some(text))
player.set_tab_list_header_footer(header, footer)
player.open_gui(gui)
player.close_container()
```

---

## 🛠 Как собрать и проверить плагин

1. **Добавить плагин в корневой `Cargo.toml`:**
   ```toml
   [workspace]
   members = [
       ...,
       "my-plugin",
   ]
   ```

2. **Собрать проект под WASM:**
   ```bash
   cargo build --release --target wasm32-wasip2
   ```

3. **Скопировать `.wasm` в `pumpkin-server/plugins/`:**
   ```bash
   cp target/wasm32-wasip2/release/my_plugin.wasm ~/pumpkin-server/plugins/
   ```

4. Сервер автоматически выполнит hot-reload плагина за ~50 мс без перезапуска сервера!
