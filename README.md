# Server Plugins (Pumpkin WASM / WASI Preview 2)

В этой папке разрабатываются модульные плагины для сервера **Pumpkin** на Rust (компилируются в `.wasm` через `wasm32-wasip2` и выполняются в песочнице Wasmtime внутри Pumpkin).

## Структура
- `auth-plugin/` — система регистрации, входа (PIN-окно + чат + команды + Bedrock Form), IP-сессий, защиты аккаунта и стартового меню.
- `tab-plugin/` — анимированный Tab-лист KRONOS, система рангов (`/ranks`, `/setrank`), форматирование чата и мониторинг TPS/MSPT для администрации.
- `menu-plugin/` — фирменный компас меню KRONOS (`ПКМ`, умная авто-выдача без перезаписи слотов), многостраничный хаб `/menu` (`/m`), персональные настройки игрока `/settings` (`settings.json`: личная погода, время суток, живой HUD-компас, звук упоминаний в чате, режим уединения, авто-выдача компаса), правила сервера `/rules`, витрина рангов и поддержка форм Bedrock Edition.

## Сборка плагинов на сервере (`klass-server`)
1. Установить таргет один раз:
   ```bash
   rustup target add wasm32-wasip2
   ```
2. Собрать `.wasm`:
   ```bash
   cd ~/plugins
   cargo build --release --target wasm32-wasip2
   ```
3. Скопировать в папку плагинов сервера:
   ```bash
   cp target/wasm32-wasip2/release/{auth_plugin,tab_plugin,menu_plugin}.wasm ~/pumpkin-server/plugins/
   ```
4. Pumpkin автоматически подхватит изменения (включён `hot_reload = true` в `pumpkin.toml`).
