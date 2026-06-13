use crate::platform::DimAttrs;
use cumulo_model::io::ExportData;
use cumulo_model::model::Bipartite;
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

pub struct AppStorage;

impl AppStorage {
    pub fn load() -> Bipartite<DimAttrs> {
        match LocalStorage::get::<Bipartite<DimAttrs>>(STORAGE_KEY) {
            Ok(bipartite) => bipartite,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[cumulo] load failed ({e:?}), using demo").into(),
                );
                Self::demo()
            }
        }
    }

    pub fn save(bipartite: &Bipartite<DimAttrs>) {
        if let Err(e) = LocalStorage::set(STORAGE_KEY, bipartite) {
            web_sys::console::error_1(&format!("[cumulo] save failed: {e:?}").into());
        }
    }

    pub fn clear() -> Bipartite<DimAttrs> {
        LocalStorage::delete(STORAGE_KEY);
        Self::demo()
    }

    fn demo() -> Bipartite<DimAttrs> {
        ExportData::<DimAttrs>::parse(cumulo_model::demo::CLOUD).expect("invalid demo")
    }
}
