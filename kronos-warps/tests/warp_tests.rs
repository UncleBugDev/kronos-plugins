use std::fs;

use kronos_warps::models::{
    format_cost_minor, parse_cost_minor, validate_warp_name, WarpError, WarpLocation, WarpStore,
    WarpType, COIN_SCALE,
};
use kronos_warps::storage::{
    charge_warp_entry_at_path, load_store_from_path, save_store_to_path, STORE_FILENAME,
};

fn sample_location(world: &str, x: f64, y: f64, z: f64) -> WarpLocation {
    WarpLocation::rounded(world.to_string(), x, y, z, 90.0, -15.5)
}

#[test]
fn validates_warp_names_strictly() {
    assert_eq!(validate_warp_name("shop_01"), Ok("shop_01".to_string()));
    assert_eq!(validate_warp_name("PvP_Arena"), Ok("pvp_arena".to_string()));
    assert_eq!(validate_warp_name("abc"), Ok("abc".to_string()));
    assert_eq!(
        validate_warp_name("a123456789012345"),
        Ok("a123456789012345".to_string())
    );

    assert_eq!(validate_warp_name("ab"), Err(WarpError::InvalidName));
    assert_eq!(
        validate_warp_name("a1234567890123456"),
        Err(WarpError::InvalidName)
    );
    assert_eq!(validate_warp_name("bad-name"), Err(WarpError::InvalidName));
    assert_eq!(validate_warp_name("варп"), Err(WarpError::InvalidName));
    assert_eq!(validate_warp_name("my warp"), Err(WarpError::InvalidName));
}

#[test]
fn serializes_and_deserializes_warp_coordinates_atomically() {
    let temp_dir = std::env::temp_dir().join(format!("kronos_warps_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("create temp dir");

    let path = temp_dir.join(STORE_FILENAME);
    let tmp_path = path.with_extension("json.tmp");

    let mut store = WarpStore::default();
    let loc = sample_location("minecraft:the_nether", 123.456, 64.0, -987.654);
    store
        .set_server_warp(
            "nether_hub",
            loc,
            "Главный хаб Незера".to_string(),
            Some("kronos-warps:vip".to_string()),
            "Admin",
            1_000,
        )
        .expect("create server warp");

    save_store_to_path(&path, &store).expect("atomic write warps.json");
    assert!(path.exists());
    assert!(!tmp_path.exists(), ".tmp file must be atomically renamed");

    let loaded = load_store_from_path(&path).expect("load warps.json");
    let warp = loaded.get("nether_hub").expect("warp exists");
    assert_eq!(warp.warp_type, WarpType::Server);
    assert_eq!(warp.location.world, "minecraft:the_nether");
    assert_eq!(warp.location.x, 123.46);
    assert_eq!(warp.location.y, 64.0);
    assert_eq!(warp.location.z, -987.65);
    assert_eq!(warp.location.yaw, 90.0);
    assert_eq!(warp.location.pitch, -15.5);
    assert_eq!(warp.permission.as_deref(), Some("kronos-warps:vip"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn enforces_warp_access_permissions_and_player_limits() {
    let mut store = WarpStore::default();
    let vip_warp = store
        .set_server_warp(
            "vip_lounge",
            sample_location("minecraft:overworld", 0.0, 80.0, 0.0),
            "VIP зона".to_string(),
            Some("kronos-warps:vip".to_string()),
            "Owner",
            10,
        )
        .unwrap();

    assert!(!vip_warp.can_access(false, |_| false));
    assert!(vip_warp.can_access(false, |p| p == "kronos-warps:vip"));
    assert!(vip_warp.can_access(true, |_| false));

    // Лимит пользовательских варпов = 1
    store
        .create_player_warp(
            "alex_shop",
            "Alex",
            sample_location("minecraft:overworld", 10.0, 70.0, 10.0),
            50 * COIN_SCALE,
            1,
            20,
        )
        .expect("first pwarp succeeds");

    assert_eq!(
        store.create_player_warp(
            "alex_farm",
            "Alex",
            sample_location("minecraft:overworld", 20.0, 70.0, 20.0),
            0,
            1,
            21,
        ),
        Err(WarpError::LimitReached { limit: 1 })
    );

    // Чужой игрок не может удалить или изменить цену варпа
    assert_eq!(
        store.delete_player_warp("alex_shop", "Steve", false),
        Err(WarpError::NotOwner)
    );
    assert_eq!(
        store.set_player_warp_cost("alex_shop", "Steve", false, 0),
        Err(WarpError::NotOwner)
    );
}

#[test]
fn increments_visits_and_sorts_popular_player_warps() {
    let mut store = WarpStore::default();
    store
        .create_player_warp(
            "shop_alpha",
            "Alice",
            sample_location("minecraft:overworld", 1.0, 64.0, 1.0),
            0,
            5,
            1,
        )
        .unwrap();
    store
        .create_player_warp(
            "shop_beta",
            "Bob",
            sample_location("minecraft:overworld", 2.0, 64.0, 2.0),
            0,
            5,
            2,
        )
        .unwrap();

    assert_eq!(store.record_visit("shop_beta"), Some(1));
    assert_eq!(store.record_visit("shop_beta"), Some(2));
    assert_eq!(store.record_visit("shop_alpha"), Some(1));

    let popular = store.popular_player_warps(10);
    assert_eq!(popular.len(), 2);
    assert_eq!(popular[0].name, "shop_beta");
    assert_eq!(popular[0].visits, 2);
    assert_eq!(popular[1].name, "shop_alpha");
    assert_eq!(popular[1].visits, 1);
}

#[test]
fn parses_costs_and_charges_via_kronos_economy_file() {
    assert_eq!(parse_cost_minor("0"), Ok(0));
    assert_eq!(parse_cost_minor("25.50"), Ok(2_550));
    assert_eq!(parse_cost_minor("-10"), Err(WarpError::InvalidCost));
    assert_eq!(format_cost_minor(0), "Бесплатно");
    assert_eq!(format_cost_minor(2_550), "25.50 ⛃");

    let temp_dir =
        std::env::temp_dir().join(format!("kronos_warps_eco_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    let eco_path = temp_dir.join("economy.json");

    let seed_json = r#"{
        "version": 1,
        "currency_symbol": "⛃",
        "currency_name": "Кронов",
        "starting_balance": 10000,
        "accounts": {
            "steve": { "player_key": "steve", "player_name": "Steve", "balance": 5000 },
            "alex":  { "player_key": "alex",  "player_name": "Alex",  "balance": 1000 }
        }
    }"#;
    fs::write(&eco_path, seed_json).unwrap();

    // Успешная оплата входа 25.00 монет
    charge_warp_entry_at_path(&eco_path, "Steve", "Alex", 25 * COIN_SCALE, 500)
        .expect("charge must succeed");

    // Отказ при нехватке средств (осталось 25.00, просим 50.00)
    assert!(matches!(
        charge_warp_entry_at_path(&eco_path, "Steve", "Alex", 50 * COIN_SCALE, 501),
        Err(WarpError::EconomyError(_))
    ));

    let updated: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&eco_path).unwrap()).unwrap();
    assert_eq!(updated["accounts"]["steve"]["balance"], 2500);
    assert_eq!(updated["accounts"]["alex"]["balance"], 3500);

    let _ = fs::remove_dir_all(&temp_dir);
}
