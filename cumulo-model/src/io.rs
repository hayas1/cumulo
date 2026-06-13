use crate::model::Bipartite;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct ExportData<A = crate::model::NoAttrs> {
    pub cumulo_version: u32,
    pub exported_at: String,
    #[serde(rename = "store")]
    pub bipartite: Bipartite<A>,
}

impl<A: Serialize + DeserializeOwned> ExportData<A> {
    pub fn new(bipartite: Bipartite<A>, exported_at: impl Into<String>) -> Self {
        ExportData {
            cumulo_version: CURRENT_VERSION,
            exported_at: exported_at.into(),
            bipartite,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn parse(json: &str) -> Result<Bipartite<A>, String> {
        let data: ExportData<A> =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
        match data.cumulo_version {
            1 => Ok(data.bipartite),
            v => Err(format!("未対応のバージョン: {v}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DimensionForest, DimensionNode, NoAttrs, Resource};
    use std::collections::HashMap;

    fn make_space() -> Bipartite {
        Bipartite {
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
                DimensionNode { id: "platform".into(), label: "プラットフォーム".into(), parent: None, attrs: NoAttrs {} },
                DimensionNode { id: "bigquery".into(), label: "BigQuery".into(), parent: Some("platform".into()), attrs: NoAttrs {} },
                DimensionNode { id: "env".into(), label: "環境".into(), parent: None, attrs: NoAttrs {} },
                DimensionNode { id: "prod".into(), label: "prod".into(), parent: Some("env".into()), attrs: NoAttrs {} },
            ]),
        }
    }

    #[test]
    fn roundtrip() {
        let bipartite = make_space();
        let json = serde_json::to_string(&ExportData {
            cumulo_version: 1,
            exported_at: "2026-06-10T00:00:00.000Z".into(),
            bipartite: bipartite.clone(),
        })
        .unwrap();
        assert_eq!(ExportData::parse(&json).unwrap(), bipartite);
    }

    #[test]
    fn unknown_version_fails() {
        let json = serde_json::json!({
            "cumulo_version": 99,
            "exported_at": "2026-06-10T00:00:00.000Z",
            "store": { "resources": [], "dimensions": [] }
        })
        .to_string();
        assert!(ExportData::<NoAttrs>::parse(&json).is_err());
    }
}
