mod app;
mod category;
mod platform;
mod resource;
mod shared;
mod storage;
mod views;

use app::Root;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(Root);
}
