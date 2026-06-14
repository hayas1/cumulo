use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::category::{Category, Taxonomy};
use crate::forest::Forest;
use crate::id::Id;
use crate::resource::{Catalog, Resource};
use crate::error::{Errors, ParseError, ValidationError};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bipartite<RA, CA> {
    pub catalog: Catalog<RA, CA>,
    pub taxonomy: Taxonomy<CA>,
}

impl<RA, CA> Bipartite<RA, CA> {
    /// 全整合性を検証してから構築する。検証を通った場合のみ Ok を返す。
    pub fn try_new(
        catalog: crate::resource::Catalog<RA, CA>,
        taxonomy: crate::category::Taxonomy<CA>,
    ) -> Result<Self, Errors<ValidationError>> {
        Bipartite { catalog, taxonomy }.validated()
    }

    /// 検証を通れば所有権ごと返す。`?` と相性がよく構築境界で使う。
    pub fn validated(self) -> Result<Self, Errors<ValidationError>> {
        self.validate()?;
        Ok(self)
    }

    /// Catalog・Taxonomy それぞれの森構造整合性と、categories のクロス整合性を全件検証する。
    pub fn validate(&self) -> Result<&Self, Errors<ValidationError>> {
        let mut errors: Vec<ValidationError> = Vec::new();
        if let Err(e) = self.catalog.validate() {
            errors.extend(e.into_iter().map(ValidationError::Catalog));
        }
        if let Err(e) = self.taxonomy.validate() {
            errors.extend(e.into_iter().map(ValidationError::Taxonomy));
        }

        for resource in self.catalog.nodes() {
            let rid = resource.id.as_str().to_string();
            // 軸（root_of）の重複を検出するため、見た軸を覚えておく
            let mut seen_axes: std::collections::HashSet<Id<Category<CA>>> =
                std::collections::HashSet::new();
            for value in &resource.categories {
                // B2: value は存在する Category
                if self.taxonomy.node(value).is_none() {
                    errors.push(ValidationError::CategoryValueMissing {
                        resource: rid.clone(),
                        value: value.as_str().to_string(),
                    });
                    continue;
                }

                // B3: value は非根（＝軸を持つ）。根そのものは選べない
                let Some(axis) = self.taxonomy.root_of(value) else {
                    errors.push(ValidationError::CategoryValueNotSelectable {
                        resource: rid.clone(),
                        value: value.as_str().to_string(),
                    });
                    continue;
                };

                // B4: 1軸1値。同じ軸に複数の値があれば違反
                if !seen_axes.insert(axis.clone()) {
                    errors.push(ValidationError::DuplicateAxis {
                        resource: rid.clone(),
                        axis: axis.as_str().to_string(),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(self)
        } else {
            Err(Errors(errors))
        }
    }
}

impl<RA, CA: Clone + PartialEq> Bipartite<RA, CA> {
    pub fn filter_resources<'a>(
        &'a self,
        selected_tags: &[(Id<Category<CA>>, Id<Category<CA>>)],
    ) -> Vec<&'a Resource<RA, CA>> {
        self.catalog
            .iter()
            .filter(|r| selected_tags.iter().all(|(k, v)| self.tag_matches(r, k, v)))
            .collect()
    }

    fn tag_matches(
        &self,
        r: &Resource<RA, CA>,
        k: &Id<Category<CA>>,
        v: &Id<Category<CA>>,
    ) -> bool {
        let Some(rv) = r.category(&self.taxonomy, k) else {
            return false;
        };
        if rv == v {
            return true;
        }
        self.taxonomy.ancestry(rv).iter().any(|a| a == v)
    }

    /// カテゴリフォレストの non-root ノード（選択可能なカテゴリ値）へのビューを返す。
    pub fn category_view(&self) -> CategoryView<'_, RA, CA> {
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
pub struct ExportData<RA, CA> {
    pub cumulo_version: u32,
    pub exported_at: String,
    #[serde(rename = "store")]
    pub bipartite: Bipartite<RA, CA>,
}

impl<RA, CA> ExportData<RA, CA>
where
    RA: Serialize + DeserializeOwned,
    CA: Serialize + DeserializeOwned,
{
    pub fn new(bipartite: Bipartite<RA, CA>, exported_at: impl Into<String>) -> Self {
        ExportData {
            cumulo_version: CURRENT_VERSION,
            exported_at: exported_at.into(),
            bipartite,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn parse(json: &str) -> Result<Bipartite<RA, CA>, ParseError> {
        let data: ExportData<RA, CA> =
            serde_json::from_str(json).map_err(|e| ParseError::Serde(e.to_string()))?;
        if data.cumulo_version != CURRENT_VERSION {
            return Err(ParseError::UnsupportedVersion(data.cumulo_version));
        }
        data.bipartite.validated().map_err(ParseError::Invalid)
    }
}

/// Bipartite のカテゴリフォレストに対するフィルタ可能なビュー。
pub struct CategoryView<'a, RA, CA> {
    pub bipartite: &'a Bipartite<RA, CA>,
    pub view: Vec<&'a Category<CA>>,
}

impl<'a, RA, CA> CategoryView<'a, RA, CA> {
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
    use crate::category::{tests::test_forest, Category, Taxonomy};
    use crate::id::Id;

    fn cid(s: &str) -> Id<Category<()>> {
        s.try_into().unwrap()
    }

    fn rid(s: &str) -> Id<Resource<(), ()>> {
        s.try_into().unwrap()
    }

    #[test]
    fn filter_selects_by_ancestry() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            catalog: Catalog(vec![
                Resource {
                    id: rid("a"),
                    label: None,
                    parent: None,
                    categories: vec![cid("bigquery")],
                    attribute: (),
                },
                Resource {
                    id: rid("b"),
                    label: None,
                    parent: None,
                    categories: vec![cid("s3")],
                    attribute: (),
                },
                Resource {
                    id: rid("c"),
                    label: None,
                    parent: None,
                    categories: vec![cid("bigtable")],
                    attribute: (),
                },
            ]),
        };
        let got = bipartite.filter_resources(&[(cid("platform"), cid("gcp"))]);
        assert!(got.iter().any(|r| r.id.as_str() == "a"));
        assert!(got.iter().any(|r| r.id.as_str() == "c"));
        assert!(!got.iter().any(|r| r.id.as_str() == "b"));
    }

    #[test]
    fn roundtrip() {
        let bipartite: Bipartite<(), ()> = Bipartite {
            catalog: Catalog(vec![Resource {
                id: rid("r1"),
                label: Some("BigQuery (prod)".into()),
                parent: None,
                categories: vec![cid("bigquery"), cid("prod")],
                attribute: (),
            }]),
            taxonomy: Taxonomy(vec![
                Category {
                    id: cid("platform"),
                    label: "プラットフォーム".into(),
                    parent: None,
                    attribute: (),
                },
                Category {
                    id: cid("bigquery"),
                    label: "BigQuery".into(),
                    parent: Some(cid("platform")),
                    attribute: (),
                },
                Category {
                    id: cid("env"),
                    label: "環境".into(),
                    parent: None,
                    attribute: (),
                },
                Category {
                    id: cid("prod"),
                    label: "prod".into(),
                    parent: Some(cid("env")),
                    attribute: (),
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
            "store": { "catalog": [], "taxonomy": [] }
        })
        .to_string();
        // バージョン不正は ParseError::UnsupportedVersion になるはず
        assert!(ExportData::<(), ()>::parse(&json).is_err());
    }

    #[test]
    fn unknown_version_is_unsupported_version_error() {
        use crate::error::ParseError;
        let json = serde_json::json!({
            "cumulo_version": 99,
            "exported_at": "2026-06-10T00:00:00.000Z",
            "store": { "catalog": [], "taxonomy": [] }
        })
        .to_string();
        let err = ExportData::<(), ()>::parse(&json).unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedVersion(99)));
    }

    #[test]
    fn malformed_json_gives_serde_error() {
        use crate::error::ParseError;
        let err = ExportData::<(), ()>::parse("not json").unwrap_err();
        assert!(matches!(err, ParseError::Serde(_)));
    }

    #[test]
    fn structurally_invalid_json_gives_invalid_error() {
        use crate::error::ParseError;
        // JSON としては正しいが構造不正: taxonomy にない axis をキーに使う dangling parent あり
        let json = serde_json::json!({
            "cumulo_version": 1,
            "exported_at": "2026-06-10T00:00:00.000Z",
            "store": {
                "catalog": [{
                    "id": "r1",
                    "categories": ["nowhere"]
                }],
                "taxonomy": []
            }
        })
        .to_string();
        let err = ExportData::<(), ()>::parse(&json).unwrap_err();
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn dangling_parent_in_taxonomy_gives_invalid_error() {
        use crate::error::ParseError;
        let json = serde_json::json!({
            "cumulo_version": 1,
            "exported_at": "2026-06-10T00:00:00.000Z",
            "store": {
                "catalog": [],
                "taxonomy": [{ "id": "child", "label": "Child", "parent": "ghost" }]
            }
        })
        .to_string();
        let err = ExportData::<(), ()>::parse(&json).unwrap_err();
        assert!(matches!(err, ParseError::Invalid(_)));
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
            catalog: Catalog(vec![]),
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
            catalog: Catalog(vec![]),
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
            catalog: Catalog(vec![]),
        };
        let all = bipartite.category_view().query("").view;
        let all_non_roots = bipartite
            .taxonomy
            .iter()
            .filter(|a| a.parent.is_some())
            .count();
        assert_eq!(all.len(), all_non_roots);
    }

    // --- validate() のテスト ---

    fn valid_bipartite() -> Bipartite<(), ()> {
        // taxonomy: platform(root) > bigquery, bigtable
        //           env(root) > prod
        // catalog: r1 が platform=bigquery, env=prod という正しい参照を持つ
        Bipartite {
            taxonomy: Taxonomy(vec![
                Category { id: cid("platform"), label: "Platform".into(), parent: None, attribute: () },
                Category { id: cid("bigquery"), label: "BigQuery".into(), parent: Some(cid("platform")), attribute: () },
                Category { id: cid("bigtable"), label: "Bigtable".into(), parent: Some(cid("platform")), attribute: () },
                Category { id: cid("env"), label: "Env".into(), parent: None, attribute: () },
                Category { id: cid("prod"), label: "prod".into(), parent: Some(cid("env")), attribute: () },
            ]),
            catalog: Catalog(vec![Resource {
                id: rid("r1"),
                label: None,
                parent: None,
                categories: vec![cid("bigquery"), cid("prod")],
                attribute: (),
            }]),
        }
    }

    // 正常系 — エラーなし
    #[test]
    fn valid_bipartite_has_no_validation_errors() {
        assert!(valid_bipartite().validate().is_ok());
    }

    // Catalog の森エラーが ValidationError::Catalog にラップされる
    #[test]
    fn catalog_forest_error_is_wrapped() {
        use crate::error::{ForestError, ValidationError};
        let mut b = valid_bipartite();
        b.catalog.push(Resource {
            id: rid("r1"), // duplicate
            label: None,
            parent: None,
            categories: vec![],
            attribute: (),
        });
        let errs = b.validate().unwrap_err();
        assert!(errs.contains(&ValidationError::Catalog(ForestError::DuplicateId {
            id: "r1".into()
        })));
    }

    // Taxonomy の森エラーが ValidationError::Taxonomy にラップされる
    #[test]
    fn taxonomy_forest_error_is_wrapped() {
        use crate::error::{ForestError, ValidationError};
        let mut b = valid_bipartite();
        b.taxonomy.push(Category {
            id: cid("bigquery"), // duplicate
            label: "dup".into(),
            parent: Some(cid("platform")),
            attribute: (),
        });
        let errs = b.validate().unwrap_err();
        assert!(errs.contains(&ValidationError::Taxonomy(ForestError::DuplicateId {
            id: "bigquery".into()
        })));
    }

    // B2: value が taxonomy に存在しない場合は CategoryValueMissing
    #[test]
    fn b2_missing_value_is_detected() {
        use crate::error::ValidationError;
        let mut b = valid_bipartite();
        b.catalog[0].categories.push(cid("staging")); // staging は存在しない
        let errs = b.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            ValidationError::CategoryValueMissing { resource, value }
            if resource == "r1" && value == "staging"
        )));
    }

    // B3: value が軸の根そのものなら CategoryValueNotSelectable（非根でなければならない）
    #[test]
    fn b3_root_value_is_not_selectable() {
        use crate::error::ValidationError;
        let mut b = valid_bipartite();
        b.catalog[0].categories.push(cid("platform")); // platform は根
        let errs = b.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            ValidationError::CategoryValueNotSelectable { resource, value }
            if resource == "r1" && value == "platform"
        )));
    }

    // B4: 同一軸に複数の値があれば DuplicateAxis
    #[test]
    fn b4_duplicate_axis_is_detected() {
        use crate::error::ValidationError;
        let mut b = valid_bipartite();
        // r1 は既に bigquery（軸 platform）を持つ。bigtable も軸 platform なので重複
        b.catalog[0].categories.push(cid("bigtable"));
        let errs = b.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            ValidationError::DuplicateAxis { resource, axis }
            if resource == "r1" && axis == "platform"
        )));
    }

    // categories が空のリソースは正常
    #[test]
    fn resource_with_no_categories_is_valid() {
        let b: Bipartite<(), ()> = Bipartite {
            taxonomy: Taxonomy(vec![
                Category { id: cid("axis"), label: "Axis".into(), parent: None, attribute: () },
            ]),
            catalog: Catalog(vec![Resource {
                id: rid("r1"),
                label: None,
                parent: None,
                categories: vec![],
                attribute: (),
            }]),
        };
        assert!(b.validate().is_ok());
    }

    // --- Bipartite::try_new のテスト ---

    fn valid_taxonomy() -> Taxonomy<()> {
        Taxonomy(vec![
            Category { id: cid("platform"), label: "Platform".into(), parent: None, attribute: () },
            Category { id: cid("bigquery"), label: "BigQuery".into(), parent: Some(cid("platform")), attribute: () },
        ])
    }

    fn valid_catalog() -> Catalog<(), ()> {
        Catalog(vec![Resource {
            id: rid("r1"),
            label: None,
            parent: None,
            categories: vec![cid("bigquery")],
            attribute: (),
        }])
    }

    #[test]
    fn try_new_returns_ok_for_valid_bipartite() {
        assert!(Bipartite::try_new(valid_catalog(), valid_taxonomy()).is_ok());
    }

    #[test]
    fn try_new_returns_err_for_invalid_category_value() {
        use crate::error::ValidationError;
        // "platform" は軸の根なのでカテゴリ値として選べない
        let catalog = Catalog(vec![Resource {
            id: rid("r1"),
            label: None,
            parent: None,
            categories: vec![cid("platform")],
            attribute: (),
        }]);
        let err = Bipartite::try_new(catalog, valid_taxonomy()).unwrap_err();
        assert!(err.iter().any(|e| matches!(e, ValidationError::CategoryValueNotSelectable { resource, value }
            if resource == "r1" && value == "platform")));
    }

    #[test]
    fn try_new_returns_err_for_catalog_forest_error() {
        use crate::error::ValidationError;
        // catalog に重複 id
        let catalog = Catalog(vec![
            Resource { id: rid("r1"), label: None, parent: None, categories: vec![], attribute: () },
            Resource { id: rid("r1"), label: None, parent: None, categories: vec![], attribute: () },
        ]);
        let err = Bipartite::try_new(catalog, valid_taxonomy()).unwrap_err();
        assert!(err.iter().any(|e| matches!(e, ValidationError::Catalog(_))));
    }

    #[test]
    fn try_new_returns_ok_for_empty_bipartite() {
        assert!(Bipartite::<(), ()>::try_new(Catalog(vec![]), Taxonomy(vec![])).is_ok());
    }
}
