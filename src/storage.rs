use crate::model::{AppStore, default_app_store};
use gloo_storage::{LocalStorage, Storage};

const STORAGE_KEY: &str = "cumulo_store";

#[derive(Debug)]
pub enum StorageError {
    SerializeError(String),
    DeserializeError(String),
    WriteError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::SerializeError(e) => write!(f, "Serialize error: {e}"),
            StorageError::DeserializeError(e) => write!(f, "Deserialize error: {e}"),
            StorageError::WriteError(e) => write!(f, "Write error: {e}"),
        }
    }
}

pub fn load_store() -> AppStore {
    match LocalStorage::get::<AppStore>(STORAGE_KEY) {
        Ok(store) => store,
        Err(_) => {
            let store = default_app_store();
            let _ = save_store(&store);
            store
        }
    }
}

pub fn save_store(store: &AppStore) -> Result<(), StorageError> {
    LocalStorage::set(STORAGE_KEY, store)
        .map_err(|e| StorageError::WriteError(e.to_string()))
}
