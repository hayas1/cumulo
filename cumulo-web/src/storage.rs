use crate::category::CategoryAttribute;
use crate::resource::ResourceAttribute;
use cumulo_model::Bipartite;
use cumulo_model::ExportData;
use cumulo_model::{Errors, ValidationError};
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

pub trait Store {
    fn load(&self) -> Bipartite<ResourceAttribute, CategoryAttribute>;
    fn save(
        &self,
        bipartite: &Bipartite<ResourceAttribute, CategoryAttribute>,
    ) -> Result<(), Errors<ValidationError>>;
    fn clear(&self) -> Bipartite<ResourceAttribute, CategoryAttribute>;
}

pub type DynStore = dyn Store + Send + Sync;

pub struct LocalStore;

pub static LOCAL_STORE: LocalStore = LocalStore;

impl LocalStore {
    fn demo() -> Bipartite<ResourceAttribute, CategoryAttribute> {
        ExportData::<ResourceAttribute, CategoryAttribute>::parse(cumulo_model::demo::CLOUD)
            .expect("demo data must be valid")
    }
}

impl Store for LocalStore {
    fn load(&self) -> Bipartite<ResourceAttribute, CategoryAttribute> {
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

    fn save(
        &self,
        bipartite: &Bipartite<ResourceAttribute, CategoryAttribute>,
    ) -> Result<(), Errors<ValidationError>> {
        bipartite.validate()?;
        if let Err(e) = LocalStorage::set(STORAGE_KEY, bipartite) {
            web_sys::console::error_1(&format!("[cumulo] save failed: {e:?}").into());
        }
        Ok(())
    }

    fn clear(&self) -> Bipartite<ResourceAttribute, CategoryAttribute> {
        LocalStorage::delete(STORAGE_KEY);
        Self::demo()
    }
}
