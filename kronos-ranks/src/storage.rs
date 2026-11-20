use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{PermissionTree, RanksState, UsersStore};

pub const RANKS_FILENAME: &str = "ranks.json";
pub const USERS_FILENAME: &str = "users.json";

static STATE: OnceLock<Mutex<RanksState>> = OnceLock::new();

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn with_state<R>(f: impl FnOnce(&mut RanksState) -> R) -> R {
    let lock = STATE.get_or_init(|| {
        Mutex::new(RanksState::new(
            ".".to_string(),
            PermissionTree::default(),
            UsersStore::default(),
        ))
    });
    let mut guard = match lock.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    f(&mut guard)
}

pub fn init_storage(data_folder: String) {
    let folder = PathBuf::from(&data_folder);
    let _ = fs::create_dir_all(&folder);

    let ranks_path = folder.join(RANKS_FILENAME);
    let users_path = folder.join(USERS_FILENAME);

    let tree = match load_ranks_from_path(&ranks_path) {
        Ok(parsed) if !parsed.ranks.is_empty() => parsed,
        _ => {
            let fresh = PermissionTree::default();
            let _ = save_ranks_to_path(&ranks_path, &fresh);
            fresh
        }
    };

    let users = match load_users_from_path(&users_path) {
        Ok(parsed) => parsed,
        Err(_) => {
            let fresh = UsersStore::default();
            let _ = save_users_to_path(&users_path, &fresh);
            fresh
        }
    };

    with_state(|st| {
        st.data_folder = data_folder;
        st.tree = tree;
        st.users = users;
    });
}

pub fn load_ranks_from_path(path: &Path) -> Result<PermissionTree, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str::<PermissionTree>(&raw).map_err(|e| e.to_string())
}

pub fn save_ranks_to_path(path: &Path, tree: &PermissionTree) -> Result<(), String> {
    let json = serde_json::to_string_pretty(tree).map_err(|e| e.to_string())?;
    atomic_write(path, &json)
}

pub fn load_users_from_path(path: &Path) -> Result<UsersStore, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str::<UsersStore>(&raw).map_err(|e| e.to_string())
}

pub fn save_users_to_path(path: &Path, users: &UsersStore) -> Result<(), String> {
    let json = serde_json::to_string_pretty(users).map_err(|e| e.to_string())?;
    atomic_write(path, &json)
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::write(path, content);
        let _ = fs::remove_file(&tmp_path);
        e.to_string()
    })
}

pub fn save_users_store() {
    let (folder, snapshot) = with_state(|st| (st.data_folder.clone(), st.users.clone()));
    let path = PathBuf::from(folder).join(USERS_FILENAME);
    if let Err(err) = save_users_to_path(&path, &snapshot) {
        tracing::warn!("KRONOS Ranks: ошибка сохранения users.json: {}", err);
    }
}

pub fn save_ranks_store() {
    let (folder, snapshot) = with_state(|st| (st.data_folder.clone(), st.tree.clone()));
    let path = PathBuf::from(folder).join(RANKS_FILENAME);
    if let Err(err) = save_ranks_to_path(&path, &snapshot) {
        tracing::warn!("KRONOS Ranks: ошибка сохранения ranks.json: {}", err);
    }
}

pub fn save_all_stores() {
    let (folder, tree, users) =
        with_state(|st| (st.data_folder.clone(), st.tree.clone(), st.users.clone()));
    let base = PathBuf::from(folder);
    let _ = save_ranks_to_path(&base.join(RANKS_FILENAME), &tree);
    let _ = save_users_to_path(&base.join(USERS_FILENAME), &users);
}
