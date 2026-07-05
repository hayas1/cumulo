// 拡張版の wasm エントリ。UI 本体は cumulo-web を再利用し、この crate には Chrome 固有の
// 差分（今は無し。将来 popup 判定など）を載せる。start は Pages 版と分けてここだけに置く。
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    cumulo_web::mount();
}
