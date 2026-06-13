use std::collections::HashSet;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::attribute::{Attribute, AttributeForest};
use crate::entity::Entity;
use crate::id::Id;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bipartite<RV, DV> {
    pub entities: Vec<Entity<RV, DV>>,
    pub attributes: AttributeForest<DV>,
}

impl<RV, DV: Clone + PartialEq> Bipartite<RV, DV> {
    pub fn filter_entities<'a>(
        &'a self,
        selected_tags: &[(Id<Attribute<DV>>, Id<Attribute<DV>>)],
    ) -> Vec<&'a Entity<RV, DV>> {
        self.entities
            .iter()
            .filter(|r| selected_tags.iter().all(|(k, v)| self.tag_matches(r, k, v)))
            .collect()
    }

    fn tag_matches(
        &self,
        r: &Entity<RV, DV>,
        k: &Id<Attribute<DV>>,
        v: &Id<Attribute<DV>>,
    ) -> bool {
        let Some(rv) = r.attributes.get(k.as_str()) else {
            return false;
        };
        if rv == v {
            return true;
        }
        self.attributes.ancestry(rv).iter().any(|a| a == v)
    }

    pub fn available_tags(
        &self,
        selected: &[(Id<Attribute<DV>>, Id<Attribute<DV>>)],
    ) -> Vec<(Id<Attribute<DV>>, Id<Attribute<DV>>)> {
        let filtered = self.filter_entities(selected);
        let selected_set: HashSet<(&str, &str)> = selected
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let mut tags: HashSet<(Id<Attribute<DV>>, Id<Attribute<DV>>)> = HashSet::new();
        for r in &filtered {
            for (k, v) in &r.attributes {
                if !selected_set.contains(&(k.as_str(), v.as_str())) {
                    tags.insert((k.clone(), v.clone()));
                }
                for anc in self.attributes.ancestry(v) {
                    if !selected_set.contains(&(k.as_str(), anc.as_str())) {
                        tags.insert((k.clone(), anc));
                    }
                }
            }
        }

        let mut tags_vec: Vec<(Id<Attribute<DV>>, Id<Attribute<DV>>)> =
            tags.into_iter().collect();
        tags_vec.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        tags_vec
    }
}

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct ExportData<RV, DV> {
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

/// サブシーケンス照合によるインクリメンタル検索。
/// "bq" → "bigquery" のような略称にも対応する。
pub struct Query(pub String);

impl Query {
    pub fn new(s: impl Into<String>) -> Self {
        Query(s.into())
    }

    pub fn matches(&self, target: &str) -> bool {
        let mut target_iter = target.chars();
        for qc in self.0.chars() {
            if !target_iter.any(|tc| tc == qc) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::{tests::test_forest, Attribute, AttributeForest};
    use std::collections::HashMap;

    #[test]
    fn filter_selects_by_ancestry() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            attributes: f,
            entities: vec![
                Entity {
                    id: "a".into(),
                    label: None,
                    attributes: HashMap::from([("platform".into(), "bigquery".into())]),
                    value: (),
                },
                Entity {
                    id: "b".into(),
                    label: None,
                    attributes: HashMap::from([("platform".into(), "s3".into())]),
                    value: (),
                },
                Entity {
                    id: "c".into(),
                    label: None,
                    attributes: HashMap::from([("platform".into(), "bigtable".into())]),
                    value: (),
                },
            ],
        };
        let got = bipartite.filter_entities(&[("platform".into(), "gcp".into())]);
        assert!(got.iter().any(|r| r.id.as_str() == "a"));
        assert!(got.iter().any(|r| r.id.as_str() == "c"));
        assert!(!got.iter().any(|r| r.id.as_str() == "b"));
    }

    #[test]
    fn roundtrip() {
        let bipartite: Bipartite<(), ()> = Bipartite {
            entities: vec![Entity {
                id: "r1".into(),
                label: Some("BigQuery (prod)".into()),
                attributes: HashMap::from([
                    ("platform".into(), "bigquery".into()),
                    ("env".into(), "prod".into()),
                ]),
                value: (),
            }],
            attributes: AttributeForest(vec![
                Attribute { id: "platform".into(), label: "プラットフォーム".into(), parent: None, value: () },
                Attribute { id: "bigquery".into(), label: "BigQuery".into(), parent: Some("platform".into()), value: () },
                Attribute { id: "env".into(), label: "環境".into(), parent: None, value: () },
                Attribute { id: "prod".into(), label: "prod".into(), parent: Some("env".into()), value: () },
            ]),
        };
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
            "store": { "entities": [], "attributes": [] }
        })
        .to_string();
        assert!(ExportData::<(), ()>::parse(&json).is_err());
    }

    #[test]
    fn abbreviation_matches() {
        assert!(Query::new("bq").matches("bigquery"));
        assert!(Query::new("gcs").matches("google-cloud-storage"));
    }

    #[test]
    fn substring_matches() {
        assert!(Query::new("big").matches("bigquery"));
        assert!(Query::new("query").matches("bigquery"));
    }

    #[test]
    fn no_match_when_char_missing() {
        assert!(!Query::new("bq").matches("bigtable"));
        assert!(!Query::new("bq").matches("storage"));
    }

    #[test]
    fn order_matters() {
        assert!(!Query::new("qb").matches("bigquery"));
    }

    #[test]
    fn empty_query_matches_any() {
        assert!(Query::new("").matches("bigquery"));
        assert!(Query::new("").matches(""));
    }
}
