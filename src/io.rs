use crate::model::AppStore;
use js_sys::Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct ExportData {
    pub cumulo_version: u32,
    pub exported_at: String,
    pub store: AppStore,
}

pub fn export_json(store: &AppStore) -> String {
    let now = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    let data = ExportData {
        cumulo_version: CURRENT_VERSION,
        exported_at: now,
        store: store.clone(),
    };
    serde_json::to_string_pretty(&data).unwrap_or_default()
}

pub fn import_json(json: &str) -> Result<AppStore, String> {
    let data: ExportData =
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
    match data.cumulo_version {
        1 => Ok(data.store),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DimensionForest, DimensionNode, Resource};
    use std::collections::HashMap;

    fn make_store() -> AppStore {
        AppStore {
            resources: vec![Resource {
                id: "r1".into(),
                label: Some("BigQuery (prod)".into()),
                dimensions: HashMap::from([
                    ("platform".into(), "bigquery".into()),
                    ("env".into(), "prod".into()),
                ]),
                console_url: "https://console.cloud.google.com/bigquery".into(),
                freq: 5,
                created_at: None,
            }],
            dimensions: DimensionForest(vec![
                DimensionNode {
                    id: "platform".into(),
                    label: "プラットフォーム".into(),
                    color: "#8899AA".into(),
                    parent: None,
                },
                DimensionNode {
                    id: "bigquery".into(),
                    label: "BigQuery".into(),
                    color: "#1D9E75".into(),
                    parent: Some("platform".into()),
                },
                DimensionNode {
                    id: "env".into(),
                    label: "環境".into(),
                    color: "#8899AA".into(),
                    parent: None,
                },
                DimensionNode {
                    id: "prod".into(),
                    label: "prod".into(),
                    color: "#E24B4A".into(),
                    parent: Some("env".into()),
                },
            ]),
        }
    }

    #[test]
    fn roundtrip() {
        let store = make_store();
        let json = serde_json::to_string(&ExportData {
            cumulo_version: 1,
            exported_at: "2026-06-10T00:00:00.000Z".into(),
            store: store.clone(),
        })
        .unwrap();
        assert_eq!(import_json(&json).unwrap(), store);
    }

    #[test]
    fn unknown_version_fails() {
        let json = serde_json::json!({
            "cumulo_version": 99,
            "exported_at": "2026-06-10T00:00:00.000Z",
            "store": { "resources": [], "dimensions": [] }
        })
        .to_string();
        assert!(import_json(&json).is_err());
    }
}
