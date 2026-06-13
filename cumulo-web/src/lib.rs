mod components;
mod map_bridge;
mod model;
mod platform;
mod storage;

use components::app::App;
use leptos::*;
use leptos_router::*;
use wasm_bindgen::prelude::*;

// trunk build --public-url /cumulo/ sets TRUNK_PUBLIC_URL at compile time.
// Strip the trailing slash so it becomes a valid router base (e.g. "/cumulo").
// Returns "" when running locally (trunk serve uses "/"), which Router treats as no base.
fn router_base() -> &'static str {
    match option_env!("TRUNK_PUBLIC_URL") {
        Some(url) if url != "/" && !url.is_empty() => url.trim_end_matches('/'),
        _ => "",
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(|| {
        view! {
            <Router base=router_base()>
                <App />
            </Router>
        }
    });
}
