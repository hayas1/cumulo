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
            categories: std::collections::HashMap::new(),
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
