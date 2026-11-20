use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{ChatState, ChatStore};

pub const STORE_FILENAME: &str = "chat_data.json";

static STATE: OnceLock<Mutex<ChatState>> = OnceLock::new();

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn with_state<R>(f: impl FnOnce(&mut ChatState) -> R) -> R {
    let lock = STATE.get_or_init(|| {
        Mutex::new(ChatState::new(".".to_string(), ChatStore::default()))
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

    let path = folder.join(STORE_FILENAME);
    let store = match load_store_from_path(&path) {
        Ok(parsed) => parsed,
        Err(_) => {
            let fresh = ChatStore::default();
            let _ = save_store_to_path(&path, &fresh);
            fresh
        }
    };

    with_state(|st| {
        st.data_folder = data_folder;
        st.store = store;
    });
}

pub fn load_store_from_path(path: &Path) -> Result<ChatStore, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str::<ChatStore>(&raw).map_err(|e| e.to_string())
}

pub fn save_store_to_path(path: &Path, store: &ChatStore) -> Result<(), String> {
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
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

pub fn save_current_store() {
    let (folder, snapshot) = with_state(|st| (st.data_folder.clone(), st.store.clone()));
    let path = PathBuf::from(folder).join(STORE_FILENAME);
    if let Err(err) = save_store_to_path(&path, &snapshot) {
        tracing::warn!("KRONOS Chat: ошибка сохранения chat_data.json: {}", err);
    }
}
