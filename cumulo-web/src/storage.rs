use crate::platform::DimAttrs;
use cumulo_model::io::ExportData;
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

pub trait AppStoreExt {
    fn save_to_storage(&self);
    fn load_from_storage() -> Self;
    fn load_default() -> Self;
    fn clear_storage() -> Self;
}

impl AppStoreExt for cumulo_model::model::AppStore<DimAttrs> {
    fn load_from_storage() -> Self {
        match LocalStorage::get::<Self>(STORAGE_KEY) {
            Ok(store) => store,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[cumulo] load_from_storage failed ({e:?}), using defaults").into(),
                );
                Self::load_default()
            }
        }
    }

    fn save_to_storage(&self) {
        if let Err(e) = LocalStorage::set(STORAGE_KEY, self) {
            web_sys::console::error_1(
                &format!("[cumulo] save_to_storage failed: {e:?}").into(),
            );
        }
    }

    fn clear_storage() -> Self {
        LocalStorage::delete(STORAGE_KEY);
        Self::load_default()
    }

    fn load_default() -> Self {
        ExportData::<DimAttrs>::parse(cumulo_model::demo::CLOUD).expect("invalid demo")
    }
}
