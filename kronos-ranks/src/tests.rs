use std::fs;

use crate::models::{
    default_permission_tree, parse_duration_ms, PlayerRankData, UsersStore,
};
use crate::storage::{
    load_ranks_from_path, load_users_from_path, save_ranks_to_path, save_users_to_path,
    RANKS_FILENAME, USERS_FILENAME,
};
use crate::tab_chat::{build_tab_name, format_chat_message, tab_sort_order};

#[test]
fn permission_inheritance_cascades_across_donate_and_staff_tracks() {
    let tree = default_permission_tree();
    assert_eq!(tree.ranks.len(), 12);

    // Дракон (вес 200) наследует права Титана, Героя, Креатива, Премиума, VIP и Игрока
    let dragon_perms = tree.inherited_permissions("dragon");
    assert!(dragon_perms.contains("kronos-warps:bypass-warmup"));
    assert!(dragon_perms.contains("kronos-warps:pwarp.elite"));
    assert!(dragon_perms.contains("kronos-ranks:cmd.heal"));
    assert!(dragon_perms.contains("kronos-ranks:cmd.enderchest"));
    assert!(dragon_perms.contains("kronos-ranks:cmd.fly"));
    assert!(dragon_perms.contains("kronos-ranks:cmd.feed"));
    assert!(dragon_perms.contains("kronos-essentials:rtp.vip"));
    assert!(dragon_perms.contains("kronos-essentials:homes.25"));

    // VIP не имеет прав Премиума (/feed) и Креатива (/fly)
    let vip_perms = tree.inherited_permissions("vip");
    assert!(vip_perms.contains("kronos-essentials:homes.3"));
    assert!(!vip_perms.contains("kronos-ranks:cmd.feed"));
    assert!(!vip_perms.contains("kronos-ranks:cmd.fly"));

    // Dual-Track: игрок одновременно с VIP и Хелпером получает права обеих веток
    let mut dual = PlayerRankData::new("Steve");
    dual.donate_rank = "vip".to_string();
    dual.staff_rank = Some("helper".to_string());
    assert!(tree.has_permission(&dual, "kronos-essentials:rtp.vip"));
    assert!(tree.has_permission(&dual, "kronos-ranks:sc"));
    assert!(tree.is_staff(&dual));

    // Явный запрет (denied_permissions) перекрывает унаследованное право
    dual.denied_permissions.push("kronos-essentials:rtp.vip".to_string());
    assert!(!tree.has_permission(&dual, "kronos-essentials:rtp.vip"));
}

#[test]
fn parses_durations_30d_90d_12h_and_perm() {
    assert_eq!(parse_duration_ms("30d"), Ok(Some(30 * 86_400 * 1_000)));
    assert_eq!(parse_duration_ms("90d"), Ok(Some(90 * 86_400 * 1_000)));
    assert_eq!(parse_duration_ms("12h"), Ok(Some(12 * 3_600 * 1_000)));
    assert_eq!(parse_duration_ms("perm"), Ok(None));
    assert_eq!(parse_duration_ms("forever"), Ok(None));
    assert!(parse_duration_ms("0d").is_err());
    assert!(parse_duration_ms("30w").is_err());
    assert!(parse_duration_ms("abc").is_err());
}

#[test]
fn temporary_rank_expiration_reverts_to_player_without_touching_permanent_staff() {
    let mut users = UsersStore::default();
    let (u, _) = users.ensure_user("Alex");
    u.donate_rank = "dragon".to_string();
    u.donate_expires_at_ms = Some(5_000);
    u.staff_rank = Some("moderator".to_string());
    u.staff_expires_at_ms = None;

    // До истечения срока
    assert!(users.expire_ranks(4_999).is_empty());
    assert_eq!(users.get_or_default("Alex").donate_rank, "dragon");

    // После истечения срока донат сбрасывается до player, стафф остаётся
    let expired = users.expire_ranks(5_000);
    assert_eq!(expired, vec![("Alex".to_string(), "dragon".to_string())]);
    let after = users.get_or_default("Alex");
    assert_eq!(after.donate_rank, "player");
    assert_eq!(after.donate_expires_at_ms, None);
    assert_eq!(after.staff_rank.as_deref(), Some("moderator"));
}

#[test]
fn tab_sort_weights_and_chat_color_formatting_match_rank_hierarchy() {
    let tree = default_permission_tree();

    let mut p_player = PlayerRankData::new("Noob");
    p_player.donate_rank = "player".to_string();

    let mut p_vip = PlayerRankData::new("Viper");
    p_vip.donate_rank = "vip".to_string();

    let mut p_sponsor = PlayerRankData::new("Rich");
    p_sponsor.donate_rank = "sponsor".to_string();

    let mut p_owner = PlayerRankData::new("Boss");
    p_owner.donate_rank = "vip".to_string();
    p_owner.staff_rank = Some("owner".to_string());

    assert_eq!(tab_sort_order(&tree, &p_player), 0);
    assert_eq!(tab_sort_order(&tree, &p_vip), 10);
    assert_eq!(tab_sort_order(&tree, &p_sponsor), 500);
    assert_eq!(tab_sort_order(&tree, &p_owner), 1000);
    assert!(tab_sort_order(&tree, &p_owner) > tab_sort_order(&tree, &p_sponsor));

    assert_eq!(build_tab_name(&tree, &p_sponsor), "§4§l[СПОНСОР] §fRich");
    assert_eq!(build_tab_name(&tree, &p_owner), "§4§l[СОЗДАТЕЛЬ] §4Boss");

    // VIP может &a и &b, но не &l и &4
    let vip_chat = format_chat_message(&tree, &p_vip, "&aПривет &4мир &lжирный");
    assert!(vip_chat.contains("§aПривет &4мир &lжирный"));

    // Спонсор может все цвета и жирный стиль
    let sponsor_chat = format_chat_message(&tree, &p_sponsor, "&4&lПривет всем");
    assert!(sponsor_chat.contains("§4§lПривет всем"));
}

#[test]
fn atomic_storage_roundtrip_for_ranks_and_users_json() {
    let temp_dir = std::env::temp_dir().join(format!("kronos_ranks_v2_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let ranks_path = temp_dir.join(RANKS_FILENAME);
    let users_path = temp_dir.join(USERS_FILENAME);

    let tree = default_permission_tree();
    let mut users = UsersStore::default();
    let (u, _) = users.ensure_user("Notch");
    u.donate_rank = "titan".to_string();

    save_ranks_to_path(&ranks_path, &tree).expect("save ranks.json");
    save_users_to_path(&users_path, &users).expect("save users.json");

    assert!(!ranks_path.with_extension("json.tmp").exists());
    assert!(!users_path.with_extension("json.tmp").exists());

    let loaded_tree = load_ranks_from_path(&ranks_path).expect("load ranks.json");
    let loaded_users = load_users_from_path(&users_path).expect("load users.json");
    assert_eq!(loaded_tree.ranks.len(), 12);
    assert_eq!(loaded_users.get_or_default("Notch").donate_rank, "titan");

    let _ = fs::remove_dir_all(&temp_dir);
}
