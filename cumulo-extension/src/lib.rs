use wasm_bindgen::prelude::*;

mod clip;
mod mode;
mod popup;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mode::Mode::current().mount();
}
