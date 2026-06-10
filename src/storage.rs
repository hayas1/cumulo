use crate::io::import_json;
use crate::model::*;
use gloo_storage::{LocalStorage, Storage};

const STORAGE_KEY: &str = "cumulo_store";

static DEFAULT_JSON: &str = include_str!("config/default.json");

pub fn load_from_storage() -> AppStore {
    match LocalStorage::get::<AppStore>(STORAGE_KEY) {
        Ok(store) => store,
        Err(e) => {
            web_sys::console::warn_1(
                &format!("[cumulo] load_from_storage failed ({e:?}), using defaults").into(),
            );
            default_app_store()
        }
    }
}

pub fn save_to_storage(store: &AppStore) {
    if let Err(e) = LocalStorage::set(STORAGE_KEY, store) {
        web_sys::console::error_1(&format!("[cumulo] save_to_storage failed: {e:?}").into());
    }
}

/// LocalStorageの保存データを消し、組み込みの初期データを返す。
pub fn clear_storage() -> AppStore {
    LocalStorage::delete(STORAGE_KEY);
    default_app_store()
}

pub fn default_app_store() -> AppStore {
    import_json(DEFAULT_JSON).expect("default.json is invalid")
}
