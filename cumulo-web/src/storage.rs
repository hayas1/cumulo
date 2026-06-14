use crate::platform::{CategoryAttribute, ResourceAttribute};
use cumulo_model::Bipartite;
use cumulo_model::ExportData;
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

pub struct AppStorage;

impl AppStorage {
    pub fn load() -> Bipartite<ResourceAttribute, CategoryAttribute> {
        match LocalStorage::get::<Bipartite<ResourceAttribute, CategoryAttribute>>(STORAGE_KEY) {
            Ok(bipartite) => match bipartite.validated() {
                Ok(b) => b,
                Err(errs) => {
                    web_sys::console::warn_1(
                        &format!("[cumulo] loaded data is invalid ({errs}), using demo").into(),
                    );
                    Self::demo()
                }
            },
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[cumulo] load failed ({e:?}), using demo").into(),
                );
                Self::demo()
            }
        }
    }

    pub fn save(bipartite: &Bipartite<ResourceAttribute, CategoryAttribute>) {
        if let Err(e) = LocalStorage::set(STORAGE_KEY, bipartite) {
            web_sys::console::error_1(&format!("[cumulo] save failed: {e:?}").into());
        }
    }

    pub fn clear() -> Bipartite<ResourceAttribute, CategoryAttribute> {
        LocalStorage::delete(STORAGE_KEY);
        Self::demo()
    }

    fn demo() -> Bipartite<ResourceAttribute, CategoryAttribute> {
        ExportData::<ResourceAttribute, CategoryAttribute>::parse(cumulo_model::demo::CLOUD)
            .expect("demo data must be valid")
    }
}
