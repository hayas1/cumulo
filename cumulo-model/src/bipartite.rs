use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::id::Id;
use crate::resource::Resource;
use crate::taxonomy::{Category, Taxonomy};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bipartite<RV, DV> {
    pub resources: Vec<Resource<RV, DV>>,
    pub taxonomy: Taxonomy<DV>,
}

impl<RV, DV: Clone + PartialEq> Bipartite<RV, DV> {
    pub fn filter_resources<'a>(
        &'a self,
        selected_tags: &[(Id<Category<DV>>, Id<Category<DV>>)],
    ) -> Vec<&'a Resource<RV, DV>> {
        self.resources
            .iter()
            .filter(|r| selected_tags.iter().all(|(k, v)| self.tag_matches(r, k, v)))
            .collect()
    }

    fn tag_matches(
        &self,
        r: &Resource<RV, DV>,
        k: &Id<Category<DV>>,
        v: &Id<Category<DV>>,
    ) -> bool {
        let Some(rv) = r.categories.get(k.as_str()) else {
            return false;
        };
        if rv == v {
            return true;
        }
        self.taxonomy.ancestry(rv).iter().any(|a| a == v)
    }

    /// カテゴリフォレストの non-root ノード（選択可能なカテゴリ値）へのビューを返す。
    pub fn category_view(&self) -> CategoryView<'_, RV, DV> {
        let view = self
            .taxonomy
            .iter()
            .filter(|a| a.parent.is_some())
            .collect();
        CategoryView {
            bipartite: self,
            view,
        }
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

/// Bipartite のカテゴリフォレストに対するフィルタ可能なビュー。
pub struct CategoryView<'a, RV, DV> {
    pub bipartite: &'a Bipartite<RV, DV>,
    pub view: Vec<&'a Category<DV>>,
}

impl<'a, RV, DV> CategoryView<'a, RV, DV> {
    /// id または label に対してサブシーケンス照合でフィルタする。大文字小文字は区別しない。
    pub fn query(self, q: &str) -> Self {
        if q.is_empty() {
            return self;
        }
        let q_lower = q.to_lowercase();
        let view = self
            .view
            .into_iter()
            .filter(|a| {
                Self::subsequence_matches(&q_lower, &a.id.to_lowercase())
                    || Self::subsequence_matches(&q_lower, &a.label.to_lowercase())
            })
            .collect();
        CategoryView {
            bipartite: self.bipartite,
            view,
        }
    }

    /// "bq" → "bigquery" のような略称にも対応するサブシーケンス照合。
    fn subsequence_matches(query: &str, target: &str) -> bool {
        let mut target_iter = target.chars();
        for qc in query.chars() {
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
    use crate::taxonomy::{tests::test_forest, Category, Taxonomy};
    use std::collections::HashMap;

    #[test]
    fn filter_selects_by_ancestry() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            resources: vec![
                Resource {
                    id: "a".into(),
                    label: None,
                    categories: HashMap::from([("platform".into(), "bigquery".into())]),
                    value: (),
                },
                Resource {
                    id: "b".into(),
                    label: None,
                    categories: HashMap::from([("platform".into(), "s3".into())]),
                    value: (),
                },
                Resource {
                    id: "c".into(),
                    label: None,
                    categories: HashMap::from([("platform".into(), "bigtable".into())]),
                    value: (),
                },
            ],
        };
        let got = bipartite.filter_resources(&[("platform".into(), "gcp".into())]);
        assert!(got.iter().any(|r| r.id.as_str() == "a"));
        assert!(got.iter().any(|r| r.id.as_str() == "c"));
        assert!(!got.iter().any(|r| r.id.as_str() == "b"));
    }

    #[test]
    fn roundtrip() {
        let bipartite: Bipartite<(), ()> = Bipartite {
            resources: vec![Resource {
                id: "r1".into(),
                label: Some("BigQuery (prod)".into()),
                categories: HashMap::from([
                    ("platform".into(), "bigquery".into()),
                    ("env".into(), "prod".into()),
                ]),
                value: (),
            }],
            taxonomy: Taxonomy(vec![
                Category {
                    id: "platform".into(),
                    label: "プラットフォーム".into(),
                    parent: None,
                    value: (),
                },
                Category {
                    id: "bigquery".into(),
                    label: "BigQuery".into(),
                    parent: Some("platform".into()),
                    value: (),
                },
                Category {
                    id: "env".into(),
                    label: "環境".into(),
                    parent: None,
                    value: (),
                },
                Category {
                    id: "prod".into(),
                    label: "prod".into(),
                    parent: Some("env".into()),
                    value: (),
                },
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
            "store": { "resources": [], "taxonomy": [] }
        })
        .to_string();
        assert!(ExportData::<(), ()>::parse(&json).is_err());
    }

    #[test]
    fn abbreviation_matches() {
        assert!(CategoryView::<(), ()>::subsequence_matches(
            "bq", "bigquery"
        ));
        assert!(CategoryView::<(), ()>::subsequence_matches(
            "gcs",
            "google-cloud-storage"
        ));
    }

    #[test]
    fn substring_matches() {
        assert!(CategoryView::<(), ()>::subsequence_matches(
            "big", "bigquery"
        ));
        assert!(CategoryView::<(), ()>::subsequence_matches(
            "query", "bigquery"
        ));
    }

    #[test]
    fn no_match_when_char_missing() {
        assert!(!CategoryView::<(), ()>::subsequence_matches(
            "bq", "bigtable"
        ));
        assert!(!CategoryView::<(), ()>::subsequence_matches(
            "bq", "storage"
        ));
    }

    #[test]
    fn order_matters() {
        assert!(!CategoryView::<(), ()>::subsequence_matches(
            "qb", "bigquery"
        ));
    }

    #[test]
    fn empty_query_matches_any() {
        assert!(CategoryView::<(), ()>::subsequence_matches("", "bigquery"));
        assert!(CategoryView::<(), ()>::subsequence_matches("", ""));
    }

    #[test]
    fn category_view_excludes_roots() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            resources: vec![],
        };
        let view = bipartite.category_view();
        assert!(view.view.iter().all(|a| a.parent.is_some()));
        assert!(!view.view.is_empty());
    }

    #[test]
    fn category_view_query_filters_by_id_and_label() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            resources: vec![],
        };
        let view = bipartite.category_view().query("bq");
        assert!(view.view.iter().any(|a| a.id.as_str() == "bigquery"));
        assert!(!view.view.iter().any(|a| a.id.as_str() == "s3"));
    }

    #[test]
    fn category_view_empty_query_returns_all_non_roots() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            resources: vec![],
        };
        let all = bipartite.category_view().query("").view;
        let all_non_roots = bipartite
            .taxonomy
            .iter()
            .filter(|a| a.parent.is_some())
            .count();
        assert_eq!(all.len(), all_non_roots);
    }
}
