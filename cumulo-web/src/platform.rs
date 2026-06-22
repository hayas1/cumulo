use cumulo_model::Resource;
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

use crate::category::{CategoryAttribute, CategoryId, Color};
use crate::resource::{ResourceAttribute, ResourceId};

/// ブラウザ固有の副作用（ID 生成、色生成、ダウンロード、URL 開放）をまとめる。
/// js_sys / web_sys を使うため core クレートには含めない。
pub struct Platform;

impl Platform {
    pub fn new_node_id() -> CategoryId {
        let n = (js_sys::Math::random() * 1e15) as u64;
        // "node" プレフィックスを付けるので空文字列にはならない
        format!("node{n:x}").try_into().unwrap()
    }

    pub fn new_resource_id() -> ResourceId {
        let n = (js_sys::Math::random() * 1e15) as u64;
        // "r" プレフィックスを付けるので空文字列にはならない
        format!("r{n:x}").try_into().unwrap()
    }

    pub fn new_resource() -> Resource<ResourceAttribute, CategoryAttribute> {
        Resource {
            id: Self::new_resource_id(),
            label: None,
            parent: None,
            categories: Vec::new(),
            attribute: ResourceAttribute::default(),
        }
    }

    pub fn random_color() -> Color {
        const PALETTE: &[&str] = &[
            "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e", "#10b981", "#14b8a6",
            "#06b6d4", "#3b82f6", "#6366f1", "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e",
        ];
        let idx = (js_sys::Math::random() * PALETTE.len() as f64) as usize;
        Color::from_hex(PALETTE[idx.min(PALETTE.len() - 1)]).expect("palette entries are valid hex")
    }

    /// leptos_router の `base` に渡す path prefix を返す。
    /// 値はビルド時の env `CUMULO_BASE_PATH` から取る。
    /// trunk は `--public-url` を cargo ビルドへ渡さず、また自身の `TRUNK_*` env は
    /// cargo 子プロセスへ素通ししないため、router base 用には独立した env を使う。
    pub fn router_base() -> &'static str {
        Self::normalize_base(option_env!("CUMULO_BASE_PATH"))
    }

    /// `<A href>` に渡す絶対パスを返す。`route` は `path!("/facet")` 等のルート定義に対応する。
    /// leptos_router の `<A>` は絶対パス（先頭 '/'）を base 前置せず素通しし、相対パスは
    /// 現在ルートからの不安定な解決になるため、base を自前で前置した絶対パスを組む。
    pub fn href(route: &str) -> String {
        Self::join_base(Self::router_base(), route)
    }

    /// base と route を結合する純粋ロジック。
    /// 不変条件: base は末尾スラッシュなし（normalize_base 保証）、route は先頭スラッシュあり。
    fn join_base(base: &str, route: &str) -> String {
        format!("{base}{route}")
    }

    /// public_url を router base にできる形へ整える。
    /// 末尾スラッシュは router base として不正なので除く。
    /// ローカル（trunk serve, public_url 未指定 or "/"）では "" を返し、Router は base なしとして扱う。
    fn normalize_base(public_url: Option<&str>) -> &str {
        match public_url {
            Some(url) if url != "/" && !url.is_empty() => url.trim_end_matches('/'),
            _ => "",
        }
    }

    pub fn now_iso() -> String {
        js_sys::Date::new_0()
            .to_iso_string()
            .as_string()
            .unwrap_or_default()
    }

    pub fn open_url(url: &str) {
        if let Some(win) = web_sys::window() {
            let _ = win.open_with_url_and_target(url, "_blank");
        }
    }

    pub fn trigger_download(filename: &str, content: &str) {
        let arr = Array::new();
        arr.push(&JsValue::from_str(content));
        let opts = BlobPropertyBag::new();
        opts.set_type("application/json");
        let blob = Blob::new_with_str_sequence_and_options(&arr, &opts).unwrap();
        let url = Url::create_object_url_with_blob(&blob).unwrap();
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let a: HtmlAnchorElement = document.create_element("a").unwrap().dyn_into().unwrap();
        a.set_href(&url);
        a.set_download(filename);
        let body = document.body().unwrap();
        body.append_child(&a).unwrap();
        a.click();
        body.remove_child(&a).unwrap();
        Url::revoke_object_url(&url).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn 末尾スラッシュを除いた_path_prefix_を返す() {
        assert_eq!(Platform::normalize_base(Some("/cumulo/")), "/cumulo");
    }

    #[test]
    fn スラッシュなしはそのまま返す() {
        assert_eq!(Platform::normalize_base(Some("/cumulo")), "/cumulo");
    }

    #[test]
    fn ローカルの_未指定_空_ルートは_base_なし() {
        // trunk serve はアセットを "/" 配信するので base は付けない
        assert_eq!(Platform::normalize_base(None), "");
        assert_eq!(Platform::normalize_base(Some("")), "");
        assert_eq!(Platform::normalize_base(Some("/")), "");
    }

    #[test]
    fn href_は_base_を前置した絶対パスを返す() {
        // 絶対パスなら現在地に依らず base 配下の同じルートへ解決される
        assert_eq!(Platform::join_base("/cumulo", "/facet"), "/cumulo/facet");
        assert_eq!(Platform::join_base("/cumulo", "/"), "/cumulo/");
    }

    #[test]
    fn href_は_base_なしでもルートを保つ() {
        assert_eq!(Platform::join_base("", "/facet"), "/facet");
        assert_eq!(Platform::join_base("", "/"), "/");
    }
}
