mod app;
mod category;
mod client;
mod platform;
mod query;
mod resource;
mod shared;
mod storage;
mod views;

pub use app::{Root, RootLocalStore};
pub use category::{CategoryAttribute, CategoryId};
pub use client::Client;
pub use platform::Platform;
pub use resource::{ResourceAttribute, ResourceId};
pub use storage::{DynStore, LocalStore, Store, LOCAL_STORE};

#[cfg(not(feature = "embed"))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    use leptos::prelude::*;

    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(RootLocalStore);
}
