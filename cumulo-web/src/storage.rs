use std::fmt;

use crate::category::CategoryAttribute;
use crate::resource::ResourceAttribute;
use cumulo_model::Bipartite;
use cumulo_model::ExportData;
use cumulo_model::{Errors, ValidationError};
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

pub enum SaveError {
    Invalid(Errors<ValidationError>),
    Storage(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Invalid(errs) => write!(f, "invalid data: {errs}"),
            SaveError::Storage(msg) => write!(f, "storage error: {msg}"),
        }
    }
}

pub trait Store {
    fn load(&self) -> Bipartite<ResourceAttribute, CategoryAttribute>;
    fn save(
        &self,
        bipartite: &Bipartite<ResourceAttribute, CategoryAttribute>,
    ) -> Result<(), SaveError>;
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
    ) -> Result<(), SaveError> {
        bipartite.validate().map_err(SaveError::Invalid)?;
        LocalStorage::set(STORAGE_KEY, bipartite)
            .map_err(|e| SaveError::Storage(format!("{e:?}")))?;
        Ok(())
    }

    fn clear(&self) -> Bipartite<ResourceAttribute, CategoryAttribute> {
        LocalStorage::delete(STORAGE_KEY);
        Self::demo()
    }
}
