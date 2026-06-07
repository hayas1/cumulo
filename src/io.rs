use crate::model::Resource;
use js_sys::Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct ExportData {
    pub cumulo_version: u32,
    pub exported_at: String,
    pub resources: Vec<Resource>,
}

pub fn export_json(resources: &[Resource]) -> String {
    let now = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    let data = ExportData {
        cumulo_version: CURRENT_VERSION,
        exported_at: now,
        resources: resources.to_vec(),
    };
    serde_json::to_string_pretty(&data).unwrap_or_default()
}

/// Returns the resources from the JSON, applying any version migrations.
pub fn import_json(json: &str) -> Result<Vec<Resource>, String> {
    let data: ExportData =
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
    match data.cumulo_version {
        1 => Ok(data.resources),
        v => Err(format!("未対応のバージョン: {v}")),
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
