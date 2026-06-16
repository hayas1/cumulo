mod components;
mod map_bridge;
mod platform;
mod storage;

use components::app::App;
use leptos::*;
use leptos_router::*;
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
