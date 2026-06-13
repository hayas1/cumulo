use crate::platform::{CategoryValue, ResourceValue};
use cumulo_model::Bipartite;
use cumulo_model::ExportData;
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

pub struct AppStorage;

impl AppStorage {
    pub fn load() -> Bipartite<ResourceValue, CategoryValue> {
        match LocalStorage::get::<Bipartite<ResourceValue, CategoryValue>>(STORAGE_KEY) {
            Ok(bipartite) => bipartite,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[cumulo] load failed ({e:?}), using demo").into(),
                );
                Self::demo()
            }
        }
    }

    pub fn save(bipartite: &Bipartite<ResourceValue, CategoryValue>) {
        if let Err(e) = LocalStorage::set(STORAGE_KEY, bipartite) {
            web_sys::console::error_1(&format!("[cumulo] save failed: {e:?}").into());
        }
    }

    pub fn clear() -> Bipartite<ResourceValue, CategoryValue> {
        LocalStorage::delete(STORAGE_KEY);
        Self::demo()
    }

    fn demo() -> Bipartite<ResourceValue, CategoryValue> {
        ExportData::<ResourceValue, CategoryValue>::parse(cumulo_model::demo::CLOUD)
            .expect("invalid demo")
    }
}
