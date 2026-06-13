use crate::model::Bipartite;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct ExportData<RV = crate::model::NoValue, DV = crate::model::NoValue> {
    pub cumulo_version: u32,
    pub exported_at: String,
    #[serde(rename = "store")]
    pub bipartite: Bipartite<RV, DV>,
}

impl<RV, DV> ExportData<RV, DV>
where
    RV: Serialize + DeserializeOwned,
    DV: Serialize + DeserializeOwned,
{
    pub fn new(bipartite: Bipartite<RV, DV>, exported_at: impl Into<String>) -> Self {
        ExportData {
            cumulo_version: CURRENT_VERSION,
            exported_at: exported_at.into(),
            bipartite,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn parse(json: &str) -> Result<Bipartite<RV, DV>, String> {
        let data: ExportData<RV, DV> =
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
    use crate::model::{AttributeForest, AttributeNode, NoValue, Entity};
    use std::collections::HashMap;

    fn make_bipartite() -> Bipartite {
        Bipartite {
            entities: vec![Entity {
                id: "r1".into(),
                label: Some("BigQuery (prod)".into()),
                attributes: HashMap::from([
                    ("platform".into(), "bigquery".into()),
                    ("env".into(), "prod".into()),
                ]),
                value: NoValue {},
            }],
            attributes: AttributeForest(vec![
                AttributeNode { id: "platform".into(), label: "プラットフォーム".into(), parent: None, value: NoValue {} },
                AttributeNode { id: "bigquery".into(), label: "BigQuery".into(), parent: Some("platform".into()), value: NoValue {} },
                AttributeNode { id: "env".into(), label: "環境".into(), parent: None, value: NoValue {} },
                AttributeNode { id: "prod".into(), label: "prod".into(), parent: Some("env".into()), value: NoValue {} },
            ]),
        }
    }

    #[test]
    fn roundtrip() {
        let bipartite = make_bipartite();
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
        assert!(ExportData::<NoValue>::parse(&json).is_err());
    }
}
