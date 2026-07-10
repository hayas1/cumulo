// 拡張版の wasm エントリ。全画面アプリは cumulo-web を再利用し、Chrome 固有の差分
// （popup 判定・Web クリッパー）だけをこの crate に載せる。start は Pages 版と分けてここに置く。
use wasm_bindgen::prelude::*;

mod clip;
mod mode;
mod popup;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // 同じ wasm を popup.html と index.html が読む。どの mode かを location から解決して mount する。
    mode::Mode::current().mount();
}
