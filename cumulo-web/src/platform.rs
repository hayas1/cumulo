use cumulo_model::Resource;
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

use crate::category::{CategoryAttribute, CategoryId};
use crate::resource::{ResourceAttribute, ResourceId};
use crate::shared::Color;

pub struct Platform;

impl Platform {
    pub fn new_node_id() -> CategoryId {
        let n = (js_sys::Math::random() * 1e15) as u64;
        format!("node{n:x}").try_into().unwrap()
    }

    pub fn new_resource_id() -> ResourceId {
        let n = (js_sys::Math::random() * 1e15) as u64;
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

    pub fn router_base() -> &'static str {
        Self::normalize_base(option_env!("CUMULO_BASE_PATH"))
    }

    pub fn href(route: &str) -> String {
        Self::join_base(Self::router_base(), route)
    }

    fn join_base(base: &str, route: &str) -> String {
        format!("{base}{route}")
    }

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
    fn normalize_base_strips_trailing_slash() {
        assert_eq!(Platform::normalize_base(Some("/cumulo/")), "/cumulo");
    }

    #[test]
    fn normalize_base_keeps_path_without_slash() {
        assert_eq!(Platform::normalize_base(Some("/cumulo")), "/cumulo");
    }

    #[test]
    fn normalize_base_is_empty_for_local_unset_or_root() {
        assert_eq!(Platform::normalize_base(None), "");
        assert_eq!(Platform::normalize_base(Some("")), "");
        assert_eq!(Platform::normalize_base(Some("/")), "");
    }

    #[test]
    fn join_base_prefixes_base_onto_absolute_path() {
        assert_eq!(Platform::join_base("/cumulo", "/facet"), "/cumulo/facet");
        assert_eq!(Platform::join_base("/cumulo", "/"), "/cumulo/");
    }

    #[test]
    fn join_base_keeps_route_without_base() {
        assert_eq!(Platform::join_base("", "/facet"), "/facet");
        assert_eq!(Platform::join_base("", "/"), "/");
    }
}
