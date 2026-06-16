use cumulo_model::{Category, Id, Resource};
use js_sys::Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

/// Web 層が Resource に付与する値。
/// `#[serde(flatten)]` で JSON にインライン展開されるため、既存データと後方互換。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceAttribute {
    pub console_url: String,
    pub created_at: Option<String>,
    pub freq: u32,
}

/// Web 層が Category に付与するビジュアル属性。
/// `#[serde(flatten)]` で JSON にインライン展開されるため、既存データと後方互換。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CategoryAttribute {
    pub color: String,
}

pub type CategoryId = Id<Category<CategoryAttribute>>;
pub type ResourceId = Id<Resource<ResourceAttribute, CategoryAttribute>>;
/// 軸→値の絞り込み選択。web 層では CA を固定して扱う。
pub type Filters = cumulo_model::Filters<CategoryAttribute>;

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

    pub fn random_color() -> String {
        const PALETTE: &[&str] = &[
            "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e", "#10b981", "#14b8a6",
            "#06b6d4", "#3b82f6", "#6366f1", "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e",
        ];
        let idx = (js_sys::Math::random() * PALETTE.len() as f64) as usize;
        PALETTE[idx.min(PALETTE.len() - 1)].to_string()
    }

    /// leptos_router の `base` に渡す path prefix を返す。
    /// 値はビルド時の env `CUMULO_BASE_PATH` から取る。
    /// trunk は `--public-url` を cargo ビルドへ渡さず、また自身の `TRUNK_*` env は
    /// cargo 子プロセスへ素通ししないため、router base 用には独立した env を使う。
    pub fn router_base() -> &'static str {
        Self::normalize_base(option_env!("CUMULO_BASE_PATH"))
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
}
