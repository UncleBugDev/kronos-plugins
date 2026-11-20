# KRONOS Warps v1.0.0 (`kronos-warps`)

Модульная система серверных и пользовательских варпов для Minecraft-сервера на ядре **Pumpkin 26.3** (`pumpkin-plugin-api = "0.1.0"`, Rust, WASI Preview 2 / `wasm32-wasip2`).

## Возможности

- **Серверные варпы (администрация):** создание на точной позиции игрока `(x, y, z, yaw, pitch, world)` с описанием и опциональным правом доступа (например, `kronos-warps:vip`), удаление и мгновенная/безопасная телепортация.
- **Безопасный Warmup (3 секунды):** обратный отсчёт в actionbar с автоматической отменой при движении игрока (`> 0.35` блока) или получении урона (`EntityDamageEvent`). Игроки с правом `kronos-warps:bypass-warmup` телепортируются мгновенно.
- **Пользовательские варпы (`/pwarp`):** создание с валидацией имени (`3-16` символов, `a-z0-9_`), лимиты по правам (`1 / 3 / 7` варпов), платное посещение с интеграцией списания/начисления через `kronos-economy` (`economy.json`), счётчик посещений `visits` и рейтинг популярных локаций.
- **Двойной UI (Java + Bedrock):**
  - **Java Edition:** каталог `/warps` с интерактивными кликабельными карточками в чате, hover-подсказками (владелец, координаты, посещения, цена) и кнопкой `[Телепорт]`.
  - **Bedrock Edition:** нативная форма `SimpleForm` с вкладками **«Серверные варпы»**, **«Популярные варпы игроков»** и **«Мои варпы»**.
- **Атомарное хранилище:** сохранение в `context.get_data_folder() / "warps.json"` через `.json.tmp` -> `fs::rename` без удержания `Mutex` на время I/O.

## Структура проекта

```text
kronos-warps/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs               # Точка входа Plugin, register_plugin!, права и события
│   ├── models.rs            # Warp, WarpType, WarpLocation, PlayerWarp, валидация
│   ├── storage.rs           # State, атомарная запись warps.json, мост kronos-economy
│   ├── sounds.rs            # Звуки 1.20.1 через play_custom_sound (SoundCategory::Players)
│   ├── menu/
│   │   ├── mod.rs           # Общие TextComponent-хелперы и роутинг двойного UI
│   │   ├── java.rs          # Интерактивный каталог /warps для Java Edition
│   │   └── bedrock.rs       # Нативные формы SimpleForm для Bedrock Edition
│   └── commands/
│       ├── mod.rs           # Регистрация команд, событий и прав
│       ├── warp.rs          # /warp, /warps и движок 3-секундного warmup
│       ├── setwarp.rs       # /setwarp <имя> [описание] [право]
│       ├── delwarp.rs       # /delwarp <имя>
│       └── pwarp.rs         # /pwarp create|delete|setcost|list
└── tests/
    └── warp_tests.rs        # Комплексные тесты валидации, координат, прав и визитов
```

## Команды и права

| Команда | Право | Описание |
| :--- | :--- | :--- |
| `/warp <имя>` | `kronos-warps:use` (`Allow`) | Телепортация на серверный или пользовательский варп (warmup 3с) |
| `/warps [server\|popular\|mine]` | `kronos-warps:use` (`Allow`) | Интерактивный каталог варпов (Java чат / Bedrock `SimpleForm`) |
| `/setwarp <имя> [описание] [право]` | `kronos-warps:admin` (`Op(3)`) | Создать или обновить серверный варп на текущей позиции |
| `/delwarp <имя>` | `kronos-warps:admin` (`Op(3)`) | Удалить серверный варп |
| `/pwarp create <имя> [цена]` | `kronos-warps:pwarp` (`Allow`) | Создать пользовательский варп (опционально платный) |
| `/pwarp delete <имя>` | `kronos-warps:pwarp` (`Allow`) | Удалить свой варп |
| `/pwarp setcost <имя> <сумма>` | `kronos-warps:pwarp` (`Allow`) | Установить стоимость посещения варпа |
| `/pwarp list [игрок]` | `kronos-warps:use` (`Allow`) | Показать список варпов игрока |

**Дополнительные права:**
- `kronos-warps:vip` — доступ к VIP-варпам и лимит до 3 личных варпов.
- `kronos-warps:elite` — лимит до 7 личных варпов.
- `kronos-warps:bypass-warmup` — мгновенная телепортация без 3-секундной задержки.

## Сборка и тестирование

1. Распакуйте папку `kronos-warps` в `examples/kronos-warps` внутри `pumpkin-plugin-dev-kit`.
2. Добавьте `"examples/kronos-warps"` в `members` корневого `Cargo.toml`.
3. Выполните из корня workspace:

```bash
# Запуск интеграционных юнит-тестов (tests/warp_tests.rs)
cargo test -p kronos-warps

# Сборка релизного .wasm плагина под Pumpkin 26.3
cargo build --release -p kronos-warps --target wasm32-wasip2
```

Скопируйте `target/wasm32-wasip2/release/kronos_warps.wasm` в папку `plugins/` сервера Pumpkin.
