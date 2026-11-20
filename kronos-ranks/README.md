# KRONOS Ranks v2.0.0 (`kronos-ranks`)

Модульная система иерархии рангов, русских префиксов, наследования прав, временных донатов, таб-листа, цветного чата, стафф-чата (`/sc`) и донат-команд для **Pumpkin 26.3** (`wasm32-wasip2`, WASI Preview 2).

## Линейка рангов (Dual-Track: Донат + Стафф)

### Донат-ветка (наследование прав снизу вверх)

| ID | Название | Префикс | Вес | Дома | Ключевые права и команды |
| :--- | :--- | :--- | :-: | :-: | :--- |
| `player` | Игрок | `§7[Игрок] §fНик` | 0 | 1 | Базовый доступ (`kronos-essentials:homes.1`) |
| `vip` | VIP | `§a[VIP] §aНик` | 10 | 3 | Множитель 1.2x, `kronos-essentials:homes.3`, `kronos-essentials:rtp.vip`, чат `&a`/`&b` |
| `premium` | Премиум | `§b[Премиум] §bНик` | 25 | 5 | `kronos-essentials:homes.5`, `/feed`, `/workbench` (`/craft`) |
| `creative` | Креатив | `§d[Креатив] §dНик` | 50 | 7 | `kronos-essentials:homes.7`, `/fly`, `/hat` |
| `hero` | Герой | `§e[Герой] §eНик` | 80 | 10 | `kronos-essentials:homes.10`, `/heal`, `/enderchest` (`/ec`) |
| `titan` | Титан | `§6[Титан] §6Ник` | 120 | 15 | `kronos-essentials:homes.15`, `/near`, `/repair`, `kronos-warps:pwarp.elite` |
| `dragon` | Дракон | `§c[Дракон] §cНик` | 200 | 25 | `kronos-essentials:homes.25`, `kronos-warps:bypass-warmup` |
| `sponsor` | Спонсор | `§4§l[СПОНСОР] §fНик` | 500 | 50 | `kronos-essentials:homes.50`, все донат-команды, все `&`-цвета и стили |

### Стафф-ветка (выдаётся параллельно с донатом)

| ID | Название | Префикс | Вес | Права |
| :--- | :--- | :--- | :-: | :--- |
| `helper` | Хелпер | `§9[Хелпер]` | 600 | Стафф-чат `/sc`, базовые права модерации |
| `moderator` | Модератор | `§3[Модератор]` | 750 | Наследует `helper`, расширенная модерация |
| `admin` | Админ | `§c§l[АДМИН]` | 900 | Наследует `moderator`, управление `/rank`, `kronos-ranks:admin` |
| `owner` | Создатель | `§4§l[СОЗДАТЕЛЬ]` | 1000 | Полный доступ `*` ко всем системам |

## Команды

- `/rank set <игрок> <ранг> [срок]` — выдать ранг (`30d`, `90d`, `12h`, `perm`). Поддерживает вызов из консоли для авто-доната.
- `/rank remove <игрок> [ранг|donate|staff]` — снять конкретный ранг или ветку.
- `/rank addperm <ранг|игрок> <право>` — добавить право рангу или игроку.
- `/rank delperm <ранг|игрок> <право>` — удалить право у ранга или игрока.
- `/sc <сообщение>` — закрытый стафф-чат (Хелпер+).
- `/ranks` — каталог донат-рангов (Java: интерактивный чат с hover-подсказками; Bedrock: `SimpleForm`).
- Донат-команды: `/fly`, `/feed`, `/heal`, `/hat`, `/workbench` (`/craft`), `/enderchest` (`/ec`).

## Команды выдачи для сайта авто-доната (RCON / Консоль)

```text
rank set {player} vip 30d
rank set {player} premium 30d
rank set {player} creative 90d
rank set {player} hero perm
rank set {player} titan 30d
rank set {player} dragon perm
rank set {player} sponsor perm
```

## Сборка и тестирование

Добавьте `"examples/kronos-ranks"` в `members` корневого `Cargo.toml` и выполните:

```bash
cargo test -p kronos-ranks
cargo build --release -p kronos-ranks --target wasm32-wasip2
```

Готовый файл `target/wasm32-wasip2/release/kronos_ranks.wasm` скопируйте в папку `plugins/` сервера Pumpkin.
