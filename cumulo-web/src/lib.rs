mod app;
mod category;
mod platform;
mod resource;
mod shared;
mod storage;
mod views;

use app::App;
use leptos::prelude::*;
use leptos_router::components::Router;
use platform::Platform;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(|| {
        view! {
            <Router base=Platform::router_base()>
                <App />
            </Router>
        }
    });
}
