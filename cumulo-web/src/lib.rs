mod app;
mod category;
mod client;
mod platform;
mod query;
mod resource;
mod shared;
mod storage;
mod views;

use app::Root;
use leptos::prelude::*;

/// アプリを body にマウントする。Pages 版・拡張版のどちらの wasm エントリからも呼ぶ共通処理。
pub fn mount() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(Root);
}

// Pages 版（cumulo-web 単体を cdylib としてビルド）の wasm エントリ。
// 埋め込み側（cumulo-extension）は embed を有効化し、start を二重に載せないよう外す。
#[cfg(not(feature = "embed"))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    mount();
}
