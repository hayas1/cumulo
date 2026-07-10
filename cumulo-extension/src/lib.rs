// 拡張版の wasm エントリ。全画面アプリは cumulo-web を再利用し、Chrome 固有の差分
// （popup 判定・Web クリッパー）だけをこの crate に載せる。start は Pages 版と分けてここに置く。
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

mod clip;
mod popup;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // 同じ wasm を popup.html と index.html が読む。popup ページなら Web クリッパーを、
    // それ以外は全画面アプリを body にマウントする。
    if popup::is_popup() {
        mount_to_body(popup::PopupApp);
    } else {
        mount_to_body(cumulo_web::RootLocalStore);
    }
}
