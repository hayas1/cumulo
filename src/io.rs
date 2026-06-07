use crate::model::{Dimension, Resource};
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
    /// Added alongside resources; absent in older exports → defaults to empty.
    #[serde(default)]
    pub dimensions: Vec<Dimension>,
}

pub struct ImportResult {
    pub resources: Vec<Resource>,
    pub dimensions: Vec<Dimension>,
}

pub fn export_json(resources: &[Resource], dimensions: &[Dimension]) -> String {
    let now = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    let data = ExportData {
        cumulo_version: CURRENT_VERSION,
        exported_at: now,
        resources: resources.to_vec(),
        dimensions: dimensions.to_vec(),
    };
    serde_json::to_string_pretty(&data).unwrap_or_default()
}

/// Parses a cumulo JSON export, applying any version migrations.
pub fn import_json(json: &str) -> Result<ImportResult, String> {
    let data: ExportData =
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
    match data.cumulo_version {
        1 => Ok(ImportResult {
            resources: data.resources,
            dimensions: data.dimensions,
        }),
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
    use crate::model::{Dimension, DimensionValue, Resource};
    use std::collections::HashMap;

    fn make_export_json(data: &ExportData) -> String {
        serde_json::to_string(data).unwrap()
    }

    #[test]
    fn roundtrip_resources_and_dimensions() {
        let resources = vec![
            Resource {
                id: "r1".into(),
                name: "auth-bigquery-prod".into(),
                attrs: HashMap::from([
                    ("env".into(), "prod".into()),
                    ("vendor".into(), "gcp".into()),
                ]),
                console_url: "https://console.cloud.google.com/bigquery".into(),
                freq: 5,
                parent_id: None,
                created_at: None,
            },
            Resource {
                id: "r2".into(),
                name: "auth-service".into(),
                attrs: HashMap::from([("env".into(), "stg".into())]),
                console_url: "https://console.cloud.google.com".into(),
                freq: 2,
                parent_id: Some("r1".into()),
                created_at: None,
            },
        ];
        let dimensions = vec![Dimension {
            id: "env".into(),
            label: "環境".into(),
            values: vec![
                DimensionValue { value: "prod".into(), color: Some("#4caf50".into()) },
                DimensionValue { value: "stg".into(), color: None },
            ],
        }];

        let json = make_export_json(&ExportData {
            cumulo_version: 1,
            exported_at: "2026-06-07T00:00:00.000Z".into(),
            resources: resources.clone(),
            dimensions: dimensions.clone(),
        });

        let result = import_json(&json).unwrap();
        assert_eq!(result.resources, resources);
        assert_eq!(result.dimensions, dimensions);
    }

    #[test]
    fn import_unknown_version_fails() {
        let json = serde_json::json!({
            "cumulo_version": 99,
            "exported_at": "2026-06-07T00:00:00.000Z",
            "resources": [],
            "dimensions": []
        })
        .to_string();
        assert!(import_json(&json).is_err());
    }

    #[test]
    fn import_legacy_without_dimensions_field() {
        // v1 exports that predate the dimensions field should still import cleanly.
        let json = r#"{
            "cumulo_version": 1,
            "exported_at": "2026-06-07T00:00:00.000Z",
            "resources": []
        }"#;
        let result = import_json(json).unwrap();
        assert!(result.dimensions.is_empty());
    }
}
