use std::fs;

use menu_plugin::store::{
    load_db_from_path, read_balance_from_economy_file, save_db_to_path, PlayerPrefs, SettingsDb,
    TimePref, WeatherPref,
};
use menu_plugin::ui::{cardinal_from_yaw, format_world_clock, ranks::ALL_12_RANKS};

#[test]
fn weather_and_time_cycles_wrap_properly() {
    let w = WeatherPref::Server;
    assert_eq!(w.cycle(), WeatherPref::Clear);
    assert_eq!(w.cycle().cycle(), WeatherPref::Downfall);
    assert_eq!(w.cycle().cycle().cycle(), WeatherPref::Server);

    let t = TimePref::Server;
    assert_eq!(t.cycle(), TimePref::Day);
    assert_eq!(t.cycle().cycle(), TimePref::Sunset);
    assert_eq!(t.cycle().cycle().cycle(), TimePref::Midnight);
    assert_eq!(t.cycle().cycle().cycle().cycle(), TimePref::Server);
}

#[test]
fn prefs_atomic_persistence_roundtrip() {
    let dir = std::env::temp_dir().join(format!("menu_plugin_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.json");

    let mut db = SettingsDb::default();
    db.players.insert(
        "steve".to_string(),
        PlayerPrefs {
            weather: WeatherPref::Clear,
            time: TimePref::Sunset,
            hud_compass: true,
            chat_mentions: false,
            hide_players: true,
            auto_compass: false,
        },
    );

    save_db_to_path(&path, &db).expect("atomic save settings.json");
    assert!(!path.with_extension("json.tmp").exists());

    let loaded = load_db_from_path(&path).expect("load settings.json");
    assert_eq!(loaded.players.get("steve"), db.players.get("steve"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cardinal_directions_and_world_clock_format_correctly() {
    assert_eq!(cardinal_from_yaw(0.0), "Ю");
    assert_eq!(cardinal_from_yaw(90.0), "З");
    assert_eq!(cardinal_from_yaw(180.0), "С");
    assert_eq!(cardinal_from_yaw(-90.0), "В");

    assert_eq!(format_world_clock(0), "06:00");
    assert_eq!(format_world_clock(6000), "12:00");
    assert_eq!(format_world_clock(18000), "00:00");
}

#[test]
fn ranks_showcase_contains_all_12_russian_ranks_in_order() {
    assert_eq!(ALL_12_RANKS.len(), 12);
    assert_eq!(ALL_12_RANKS[0].badge, "§7[Игрок]");
    assert_eq!(ALL_12_RANKS[1].badge, "§a[VIP]");
    assert_eq!(ALL_12_RANKS[7].badge, "§4§l[СПОНСОР]");
    assert_eq!(ALL_12_RANKS[11].badge, "§4§l[СОЗДАТЕЛЬ]");

    for w in ALL_12_RANKS.windows(2) {
        assert!(w[0].weight < w[1].weight, "rank weights must strictly increase");
    }
}

#[test]
fn reads_live_balance_from_kronos_economy_json() {
    let dir = std::env::temp_dir().join(format!("menu_eco_bridge_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let eco_path = dir.join("economy.json");

    fs::write(
        &eco_path,
        r#"{"currency_symbol":"⛃","starting_balance":10000,"accounts":{"alex":{"balance":25550}}}"#,
    )
    .unwrap();

    assert_eq!(
        read_balance_from_economy_file(&eco_path, "Alex"),
        Some("255.50 ⛃".to_string())
    );
    assert_eq!(
        read_balance_from_economy_file(&eco_path, "NewPlayer"),
        Some("100 ⛃".to_string())
    );
    let _ = fs::remove_dir_all(&dir);
}
