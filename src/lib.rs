mod components;
mod logic;
mod map_bridge;
mod model;
mod storage;

use components::app::App;
use leptos::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(|| view! { <App /> });
}
