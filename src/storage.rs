use crate::model::*;
use gloo_storage::{LocalStorage, Storage};

const STORAGE_KEY: &str = "cumulo_store";

static DIMENSIONS_JSON: &str = include_str!("config/dimensions.json");
static RESOURCES_JSON: &str = include_str!("config/resources.json");
static MAP_CONFIG_JSON: &str = include_str!("config/map_config.json");

pub fn load_from_storage() -> AppStore {
    match LocalStorage::get::<AppStore>(STORAGE_KEY) {
        Ok(store) => store,
        Err(_) => default_app_store(),
    }
}

pub fn save_to_storage(store: &AppStore) {
    let _ = LocalStorage::set(STORAGE_KEY, store);
}

pub fn default_app_store() -> AppStore {
    AppStore {
        resources: serde_json::from_str(RESOURCES_JSON).expect("resources.json is invalid"),
        dimensions: serde_json::from_str(DIMENSIONS_JSON).expect("dimensions.json is invalid"),
        map_config: serde_json::from_str(MAP_CONFIG_JSON).expect("map_config.json is invalid"),
    }
}
