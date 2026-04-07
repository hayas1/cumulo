//! Leptos ↔ D3.js ブリッジ。
//! D3の関数はmap.jsがwindowに公開した `cumulo*` グローバルを経由して呼ぶ。
//! D3→Leptosのイベントは `window.__cumuloCallbacks` オブジェクト経由で
//! wasm-bindgen Closure を差し込む。

use js_sys::{Function, Reflect};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::window;

// ── helpers ──────────────────────────────────────────────────────────────────

fn win_fn(name: &str) -> Option<Function> {
    let win = window()?;
    Reflect::get(&win, &JsValue::from_str(name))
        .ok()?
        .dyn_into::<Function>()
        .ok()
}

fn callbacks_obj() -> Option<JsValue> {
    let win = window()?;
    Reflect::get(&win, &JsValue::from_str("__cumuloCallbacks")).ok()
}

fn call0(name: &str) {
    if let Some(f) = win_fn(name) {
        let _ = f.call0(&JsValue::NULL);
    }
}

fn call1(name: &str, arg: &str) {
    if let Some(f) = win_fn(name) {
        let _ = f.call1(&JsValue::NULL, &JsValue::from_str(arg));
    }
}

// ── outbound: Leptos → D3 ────────────────────────────────────────────────────

pub fn init_map(canvas_id: &str) {
    call1("cumuloInitMap", canvas_id);
}

pub fn update_resources(json: &str) {
    call1("cumuloUpdateResources", json);
}

pub fn update_filter(json: &str) {
    call1("cumuloUpdateFilter", json);
}

pub fn update_zoom_axes(json: &str) {
    call1("cumuloUpdateZoomAxes", json);
}

pub fn zoom_to_fit() {
    call0("cumuloZoomToFit");
}

pub fn zoom_in() {
    call0("cumuloZoomIn");
}

pub fn zoom_out() {
    call0("cumuloZoomOut");
}

// ── inbound: D3 → Leptos (callback registration) ─────────────────────────────

/// D3がリソースを選択したときに呼ばれるコールバックを登録する。
/// Closure は意図的にリークさせてアプリのライフタイム全体で有効にする。
pub fn on_resource_select(callback: impl Fn(String) + 'static) {
    let closure = Closure::wrap(Box::new(callback) as Box<dyn Fn(String)>);
    if let Some(obj) = callbacks_obj() {
        let _ = Reflect::set(&obj, &JsValue::from_str("onResourceSelect"), closure.as_ref());
    }
    closure.forget();
}

/// D3のズームレベルが変わったときに呼ばれるコールバックを登録する。
pub fn on_zoom_level_change(callback: impl Fn(u32) + 'static) {
    let closure = Closure::wrap(Box::new(callback) as Box<dyn Fn(u32)>);
    if let Some(obj) = callbacks_obj() {
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("onZoomLevelChange"),
            closure.as_ref(),
        );
    }
    closure.forget();
}
