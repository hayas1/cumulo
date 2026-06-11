use crate::io::ExportData;
use crate::model::AppStore;
use gloo_storage::{LocalStorage, Storage};

const STORAGE_KEY: &str = "cumulo_store";

static DEFAULT_JSON: &str = include_str!("config/default.json");

impl AppStore {
    pub fn load_from_storage() -> Self {
        match LocalStorage::get::<AppStore>(STORAGE_KEY) {
            Ok(store) => store,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[cumulo] load_from_storage failed ({e:?}), using defaults").into(),
                );
                Self::load_default()
            }
        }
    }

    pub fn save_to_storage(&self) {
        if let Err(e) = LocalStorage::set(STORAGE_KEY, self) {
            web_sys::console::error_1(&format!("[cumulo] save_to_storage failed: {e:?}").into());
        }
    }

    pub fn clear_storage() -> Self {
        LocalStorage::delete(STORAGE_KEY);
        Self::load_default()
    }

    pub fn load_default() -> Self {
        ExportData::parse(DEFAULT_JSON).expect("default.json is invalid")
    }
}
