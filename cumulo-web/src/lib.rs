mod app;
mod category;
mod client;
mod platform;
mod query;
mod resource;
mod shared;
mod storage;
mod views;

// 拡張など再利用側（cumulo-extension）が組み立てに使う公開面。
// cumulo-web は「Leptos アプリ本体＋差し替え可能な永続化層」を提供する crate として振る舞う。
pub use app::{Root, RootLocalStore};
pub use category::{CategoryAttribute, CategoryId};
pub use client::Client;
pub use platform::Platform;
pub use resource::{ResourceAttribute, ResourceId};
pub use storage::{DynStore, LocalStore, Store, LOCAL_STORE};

// Pages 版（cumulo-web 単体を cdylib としてビルド）の wasm エントリ。
// 埋め込み側（cumulo-extension）は embed を有効化し、start を二重に載せないよう外す。
#[cfg(not(feature = "embed"))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    use leptos::prelude::*;

    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(RootLocalStore);
}
