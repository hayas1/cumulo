use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::category::{Category, Taxonomy};
use crate::error::{Errors, ForestError, ParseError, ValidationError};
use crate::filters::Filters;
use crate::forest::{Forest, ForestMut, ForestNode};
use crate::id::Id;
use crate::resource::{Catalog, Resource};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bipartite<RA, CA> {
    pub catalog: Catalog<RA, CA>,
    pub taxonomy: Taxonomy<CA>,
}

impl<RA, CA> Bipartite<RA, CA> {
    pub fn try_new(
        catalog: crate::resource::Catalog<RA, CA>,
        taxonomy: crate::category::Taxonomy<CA>,
    ) -> Result<Self, Errors<ValidationError>> {
        Bipartite { catalog, taxonomy }.validated()
    }

    pub fn validated(self) -> Result<Self, Errors<ValidationError>> {
        self.validate()?;
        Ok(self)
    }

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
            let mut seen_axes: std::collections::HashSet<Id<Category<CA>>> =
                std::collections::HashSet::new();
            for value in &resource.categories {
                if self.taxonomy.node(value).is_none() {
                    errors.push(ValidationError::CategoryValueMissing {
                        resource: rid.clone(),
                        value: value.as_str().to_string(),
                    });
                    continue;
                }
                let Some(axis) = self.taxonomy.root_of(value) else {
                    continue;
                };

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

    pub fn resources_with_category(&self, category: &Id<Category<CA>>) -> Vec<&Resource<RA, CA>> {
        self.catalog
            .iter()
            .filter(|r| r.categories.iter().any(|c| c == category))
            .collect()
    }

    pub fn rename_category(
        &mut self,
        old_id: &Id<Category<CA>>,
        new_id: Id<Category<CA>>,
        label: &str,
        attribute: CA,
    ) -> Result<(), ForestError> {
        self.taxonomy
            .rename_node(old_id, new_id.clone(), label, attribute)?;
        if old_id != &new_id {
            for resource in self.catalog.iter_mut() {
                for c in resource.categories.iter_mut() {
                    if c == old_id {
                        *c = new_id.clone();
                    }
                }
            }
        }
        Ok(())
    }

    fn categories_removed_by_delete(
        &self,
        node_id: &Id<Category<CA>>,
        subtree: bool,
    ) -> std::collections::HashSet<Id<Category<CA>>> {
        if subtree {
            self.taxonomy.collect_descendants(node_id)
        } else {
            std::iter::once(node_id.clone()).collect()
        }
    }

    pub fn resources_affected_by_delete(
        &self,
        node_id: &Id<Category<CA>>,
        subtree: bool,
    ) -> Vec<&Resource<RA, CA>> {
        let removed = self.categories_removed_by_delete(node_id, subtree);
        self.catalog
            .iter()
            .filter(|r| r.categories.iter().any(|c| removed.contains(c)))
            .collect()
    }

    pub fn delete_category(&mut self, node_id: &Id<Category<CA>>, subtree: bool) {
        if subtree {
            let removed = self.categories_removed_by_delete(node_id, true);
            self.taxonomy.delete_subtree(node_id);
            for resource in self.catalog.iter_mut() {
                resource.categories.retain(|c| !removed.contains(c));
            }
        } else {
            let parent = self.taxonomy.node(node_id).and_then(|n| n.parent.clone());
            self.taxonomy.delete_promote(node_id);
            for resource in self.catalog.iter_mut() {
                match &parent {
                    Some(parent) => resource.categories.iter_mut().for_each(|c| {
                        if c == node_id {
                            *c = parent.clone();
                        }
                    }),
                    None => resource.categories.retain(|c| c != node_id),
                }
            }
        }
    }
}

impl<RA, CA: Clone + PartialEq> Bipartite<RA, CA> {
    pub fn filtered(&self, filters: &Filters<CA>) -> ResourceSelection<'_, RA, CA> {
        let items = self
            .catalog
            .iter()
            .filter(|r| self.matches_resource(r, filters))
            .collect();
        ResourceSelection { items }
    }

    pub fn subtree_counts(
        &self,
        root: &Id<Category<CA>>,
        base: &Filters<CA>,
    ) -> std::collections::HashMap<Id<Category<CA>>, usize> {
        let mut counts = std::collections::HashMap::new();
        for resource in self.filtered(base).items() {
            if let Some(value) = resource.category(&self.taxonomy, root) {
                for node in self.taxonomy.ancestry(value) {
                    *counts.entry(node).or_default() += 1;
                }
            }
        }
        counts
    }

    pub fn matches(&self, id: &Id<Resource<RA, CA>>, filters: &Filters<CA>) -> bool {
        self.catalog
            .node(id)
            .is_some_and(|r| self.matches_resource(r, filters))
    }

    fn matches_resource(&self, r: &Resource<RA, CA>, filters: &Filters<CA>) -> bool {
        filters.iter().all(|(k, v)| self.tag_matches(r, k, v))
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

    pub fn category_selection(&self) -> CategorySelection<'_, CA> {
        CategorySelection {
            items: self.taxonomy.iter().collect(),
        }
    }

    pub fn tree_pivot<'a>(
        &'a self,
        row_axis: &Id<Category<CA>>,
        col_axis: &Id<Category<CA>>,
        row_expanded: &std::collections::HashSet<Id<Category<CA>>>,
        col_expanded: &std::collections::HashSet<Id<Category<CA>>>,
        base: &Filters<CA>,
    ) -> TreePivot<'a, CA> {
        let rows = self.visible_axis_nodes(row_axis, row_expanded);
        let cols = self.visible_axis_nodes(col_axis, col_expanded);
        let row_root = self.taxonomy.root_or_self(row_axis);
        let col_root = self.taxonomy.root_or_self(col_axis);
        let base = base.without_root(&row_root).without_root(&col_root);

        let mut counts = CellCounts::new();
        for resource in self.catalog.iter() {
            if !self.matches_resource(resource, &base) {
                continue;
            }
            let row_chain = self.axis_chain(resource, row_axis, &row_root);
            let col_chain = self.axis_chain(resource, col_axis, &col_root);
            for row in &row_chain {
                for col in &col_chain {
                    *counts.entry((row.clone(), col.clone())).or_default() += 1;
                }
            }
        }
        TreePivot {
            rows,
            cols,
            row_axis: row_axis.clone(),
            col_axis: col_axis.clone(),
            counts,
        }
    }

    fn visible_axis_nodes<'a>(
        &'a self,
        axis: &Id<Category<CA>>,
        expanded: &std::collections::HashSet<Id<Category<CA>>>,
    ) -> Vec<PivotNode<'a, CA>> {
        let mut out = Vec::new();
        if self.taxonomy.children_of(axis).is_empty() {
            if let Some(node) = self.taxonomy.node(axis) {
                out.push(PivotNode {
                    node,
                    depth: 0,
                    has_children: false,
                });
            }
        } else {
            self.collect_visible(axis, 0, expanded, &mut out);
        }
        out
    }

    fn collect_visible<'a>(
        &'a self,
        parent: &Id<Category<CA>>,
        depth: usize,
        expanded: &std::collections::HashSet<Id<Category<CA>>>,
        out: &mut Vec<PivotNode<'a, CA>>,
    ) {
        for child in self.taxonomy.children_of(parent) {
            let has_children = !self.taxonomy.children_of(&child.id).is_empty();
            out.push(PivotNode {
                node: child,
                depth,
                has_children,
            });
            if has_children && expanded.contains(&child.id) {
                self.collect_visible(&child.id, depth + 1, expanded, out);
            }
        }
    }

    fn axis_chain(
        &self,
        resource: &Resource<RA, CA>,
        axis: &Id<Category<CA>>,
        root: &Id<Category<CA>>,
    ) -> Vec<Id<Category<CA>>> {
        let Some(value) = resource.category(&self.taxonomy, root) else {
            return Vec::new();
        };
        let ancestry = self.taxonomy.ancestry(value);
        match ancestry.iter().position(|node| node == axis) {
            Some(pos) => ancestry[..=pos].to_vec(),
            None => Vec::new(),
        }
    }
}

pub trait Selection {
    type Item: ForestNode;

    fn items(&self) -> &[&Self::Item];

    fn len(&self) -> usize {
        self.items().len()
    }

    fn is_empty(&self) -> bool {
        self.items().is_empty()
    }
}

pub struct ResourceSelection<'a, RA, CA> {
    items: Vec<&'a Resource<RA, CA>>,
}

impl<RA, CA> Selection for ResourceSelection<'_, RA, CA> {
    type Item = Resource<RA, CA>;
    fn items(&self) -> &[&Resource<RA, CA>] {
        &self.items
    }
}

type CellCounts<CA> = std::collections::HashMap<(Id<Category<CA>>, Id<Category<CA>>), usize>;

pub struct PivotNode<'a, CA> {
    pub node: &'a Category<CA>,
    pub depth: usize,
    pub has_children: bool,
}

pub struct TreePivot<'a, CA> {
    pub rows: Vec<PivotNode<'a, CA>>,
    pub cols: Vec<PivotNode<'a, CA>>,
    row_axis: Id<Category<CA>>,
    col_axis: Id<Category<CA>>,
    counts: CellCounts<CA>,
}

impl<CA> TreePivot<'_, CA> {
    pub fn count(&self, row: &Id<Category<CA>>, col: &Id<Category<CA>>) -> usize {
        self.counts
            .get(&(row.clone(), col.clone()))
            .copied()
            .unwrap_or(0)
    }

    pub fn row_total(&self, row: &Id<Category<CA>>) -> usize {
        self.count(row, &self.col_axis)
    }

    pub fn col_total(&self, col: &Id<Category<CA>>) -> usize {
        self.count(&self.row_axis, col)
    }

    pub fn total(&self) -> usize {
        self.count(&self.row_axis, &self.col_axis)
    }
}

pub struct CategorySelection<'a, CA> {
    items: Vec<&'a Category<CA>>,
}

impl<CA> Selection for CategorySelection<'_, CA> {
    type Item = Category<CA>;
    fn items(&self) -> &[&Category<CA>] {
        &self.items
    }
}

impl<'a, CA> CategorySelection<'a, CA> {
    pub fn query(self, q: &str) -> Self {
        if q.is_empty() {
            return self;
        }
        let q_lower = q.to_lowercase();
        let items = self
            .items
            .into_iter()
            .filter(|a| {
                Self::subsequence_matches(&q_lower, &a.id.to_lowercase())
                    || Self::subsequence_matches(&q_lower, &a.label.to_lowercase())
            })
            .collect();
        CategorySelection { items }
    }

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
    fn filter_ands_across_axes() {
        let bipartite = valid_bipartite();
        let both = Filters::from_iter([
            (cid("platform"), cid("bigquery")),
            (cid("env"), cid("prod")),
        ]);
        assert_eq!(bipartite.filtered(&both).len(), 1);
        let unmatched = Filters::from_iter([
            (cid("platform"), cid("bigtable")),
            (cid("env"), cid("prod")),
        ]);
        assert!(bipartite.filtered(&unmatched).is_empty());
    }

    #[test]
    fn filtered_lists_matching_resources() {
        let bipartite = valid_bipartite();
        let view = bipartite.filtered(&Filters::from_iter([(cid("platform"), cid("bigquery"))]));
        assert_eq!(view.len(), 1);
        assert!(view.items().iter().any(|r| r.id == rid("r1")));
        assert!(bipartite
            .filtered(&Filters::from_iter([(cid("platform"), cid("bigtable"))]))
            .is_empty());
    }

    #[test]
    fn matches_tests_single_resource_against_filters() {
        let bipartite = valid_bipartite();
        let f = Filters::from_iter([(cid("platform"), cid("bigquery"))]);
        assert!(bipartite.matches(&rid("r1"), &f));
        let f2 = Filters::from_iter([(cid("platform"), cid("bigtable"))]);
        assert!(!bipartite.matches(&rid("r1"), &f2));
        assert!(!bipartite.matches(&rid("ghost"), &f));
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
        let view = bipartite.filtered(&Filters::from_iter([(cid("platform"), cid("gcp"))]));
        let got = view.items();
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
                    label: "Platform".into(),
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
                    label: "Env".into(),
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
        assert!(CategorySelection::<()>::subsequence_matches(
            "bq", "bigquery"
        ));
        assert!(CategorySelection::<()>::subsequence_matches(
            "gcs",
            "google-cloud-storage"
        ));
    }

    #[test]
    fn substring_matches() {
        assert!(CategorySelection::<()>::subsequence_matches(
            "big", "bigquery"
        ));
        assert!(CategorySelection::<()>::subsequence_matches(
            "query", "bigquery"
        ));
    }

    #[test]
    fn no_match_when_char_missing() {
        assert!(!CategorySelection::<()>::subsequence_matches(
            "bq", "bigtable"
        ));
        assert!(!CategorySelection::<()>::subsequence_matches(
            "bq", "storage"
        ));
    }

    #[test]
    fn order_matters() {
        assert!(!CategorySelection::<()>::subsequence_matches(
            "qb", "bigquery"
        ));
    }

    #[test]
    fn empty_query_matches_any() {
        assert!(CategorySelection::<()>::subsequence_matches("", "bigquery"));
        assert!(CategorySelection::<()>::subsequence_matches("", ""));
    }

    #[test]
    fn category_view_includes_all_nodes() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            catalog: Catalog(vec![]),
        };
        let sel = bipartite.category_selection();
        assert!(sel.items().iter().any(|a| a.parent.is_none()));
        assert_eq!(sel.len(), bipartite.taxonomy.iter().count());
    }

    #[test]
    fn category_view_query_filters_by_id_and_label() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            catalog: Catalog(vec![]),
        };
        let sel = bipartite.category_selection().query("bq");
        assert!(sel.items().iter().any(|a| a.id.as_str() == "bigquery"));
        assert!(!sel.items().iter().any(|a| a.id.as_str() == "s3"));
    }

    #[test]
    fn category_view_empty_query_returns_all_nodes() {
        let f = test_forest();
        let bipartite: Bipartite<(), ()> = Bipartite {
            taxonomy: f,
            catalog: Catalog(vec![]),
        };
        let all_nodes = bipartite.taxonomy.iter().count();
        assert_eq!(bipartite.category_selection().query("").len(), all_nodes);
    }

    fn valid_bipartite() -> Bipartite<(), ()> {
        Bipartite {
            taxonomy: Taxonomy(vec![
                Category {
                    id: cid("platform"),
                    label: "Platform".into(),
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
                    id: cid("bigtable"),
                    label: "Bigtable".into(),
                    parent: Some(cid("platform")),
                    attribute: (),
                },
                Category {
                    id: cid("env"),
                    label: "Env".into(),
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
            catalog: Catalog(vec![Resource {
                id: rid("r1"),
                label: None,
                parent: None,
                categories: vec![cid("bigquery"), cid("prod")],
                attribute: (),
            }]),
        }
    }

    #[test]
    fn valid_bipartite_has_no_validation_errors() {
        assert!(valid_bipartite().validate().is_ok());
    }

    #[test]
    fn catalog_forest_error_is_wrapped() {
        use crate::error::{ForestError, ValidationError};
        let mut b = valid_bipartite();
        b.catalog.push(Resource {
            id: rid("r1"),
            label: None,
            parent: None,
            categories: vec![],
            attribute: (),
        });
        let errs = b.validate().unwrap_err();
        assert!(
            errs.contains(&ValidationError::Catalog(ForestError::DuplicateId {
                id: "r1".into()
            }))
        );
    }

    #[test]
    fn taxonomy_forest_error_is_wrapped() {
        use crate::error::{ForestError, ValidationError};
        let mut b = valid_bipartite();
        b.taxonomy.push(Category {
            id: cid("bigquery"),
            label: "dup".into(),
            parent: Some(cid("platform")),
            attribute: (),
        });
        let errs = b.validate().unwrap_err();
        assert!(
            errs.contains(&ValidationError::Taxonomy(ForestError::DuplicateId {
                id: "bigquery".into()
            }))
        );
    }

    #[test]
    fn b2_missing_value_is_detected() {
        use crate::error::ValidationError;
        let mut b = valid_bipartite();
        b.catalog[0].categories.push(cid("staging"));
        let errs = b.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            ValidationError::CategoryValueMissing { resource, value }
            if resource == "r1" && value == "staging"
        )));
    }

    #[test]
    fn b3_root_value_is_selectable() {
        let b: Bipartite<(), ()> = Bipartite {
            taxonomy: Taxonomy(vec![Category {
                id: cid("axis"),
                label: "Axis".into(),
                parent: None,
                attribute: (),
            }]),
            catalog: Catalog(vec![Resource {
                id: rid("r1"),
                label: None,
                parent: None,
                categories: vec![cid("axis")],
                attribute: (),
            }]),
        };
        assert!(b.validate().is_ok());
    }

    #[test]
    fn b4_duplicate_axis_is_detected() {
        use crate::error::ValidationError;
        let mut b = valid_bipartite();
        b.catalog[0].categories.push(cid("bigtable"));
        let errs = b.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            ValidationError::DuplicateAxis { resource, axis }
            if resource == "r1" && axis == "platform"
        )));
    }

    #[test]
    fn resource_with_no_categories_is_valid() {
        let b: Bipartite<(), ()> = Bipartite {
            taxonomy: Taxonomy(vec![Category {
                id: cid("axis"),
                label: "Axis".into(),
                parent: None,
                attribute: (),
            }]),
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

    fn valid_taxonomy() -> Taxonomy<()> {
        Taxonomy(vec![
            Category {
                id: cid("platform"),
                label: "Platform".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: cid("bigquery"),
                label: "BigQuery".into(),
                parent: Some(cid("platform")),
                attribute: (),
            },
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
    fn try_new_returns_err_for_missing_category_value() {
        use crate::error::ValidationError;
        let catalog = Catalog(vec![Resource {
            id: rid("r1"),
            label: None,
            parent: None,
            categories: vec![cid("ghost")],
            attribute: (),
        }]);
        let err = Bipartite::try_new(catalog, valid_taxonomy()).unwrap_err();
        assert!(err.iter().any(
            |e| matches!(e, ValidationError::CategoryValueMissing { resource, value }
            if resource == "r1" && value == "ghost")
        ));
    }

    #[test]
    fn try_new_returns_err_for_catalog_forest_error() {
        use crate::error::ValidationError;
        let catalog = Catalog(vec![
            Resource {
                id: rid("r1"),
                label: None,
                parent: None,
                categories: vec![],
                attribute: (),
            },
            Resource {
                id: rid("r1"),
                label: None,
                parent: None,
                categories: vec![],
                attribute: (),
            },
        ]);
        let err = Bipartite::try_new(catalog, valid_taxonomy()).unwrap_err();
        assert!(err.iter().any(|e| matches!(e, ValidationError::Catalog(_))));
    }

    #[test]
    fn try_new_returns_ok_for_empty_bipartite() {
        assert!(Bipartite::<(), ()>::try_new(Catalog(vec![]), Taxonomy(vec![])).is_ok());
    }

    #[test]
    fn resources_with_category_lists_referencing_resources() {
        let b = valid_bipartite();
        let users = b.resources_with_category(&cid("bigquery"));
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, rid("r1"));
        assert!(b.resources_with_category(&cid("bigtable")).is_empty());
    }

    #[test]
    fn rename_category_cascades_to_resources() {
        let mut b = valid_bipartite();
        b.rename_category(&cid("bigquery"), cid("bq"), "BQ", ())
            .unwrap();
        assert!(b.taxonomy.node(&cid("bigquery")).is_none());
        assert_eq!(b.taxonomy.node(&cid("bq")).unwrap().label, "BQ");
        assert!(b.catalog[0].categories.contains(&cid("bq")));
        assert!(!b.catalog[0].categories.contains(&cid("bigquery")));
        assert!(b.validate().is_ok());
    }

    #[test]
    fn rename_category_label_only_keeps_references() {
        let mut b = valid_bipartite();
        b.rename_category(&cid("bigquery"), cid("bigquery"), "BigQuery 2", ())
            .unwrap();
        assert_eq!(
            b.taxonomy.node(&cid("bigquery")).unwrap().label,
            "BigQuery 2"
        );
        assert!(b.catalog[0].categories.contains(&cid("bigquery")));
    }

    #[test]
    fn rename_category_to_existing_id_is_rejected_and_leaves_unchanged() {
        use crate::error::ForestError;
        let mut b = valid_bipartite();
        let err = b
            .rename_category(&cid("bigquery"), cid("bigtable"), "x", ())
            .unwrap_err();
        assert!(matches!(err, ForestError::DuplicateId { id } if id == "bigtable"));
        assert!(b.taxonomy.node(&cid("bigquery")).is_some());
        assert!(b.catalog[0].categories.contains(&cid("bigquery")));
    }

    #[test]
    fn delete_category_promote_reassigns_resources_to_parent() {
        let mut b = valid_bipartite();
        b.delete_category(&cid("bigquery"), false);
        assert!(b.taxonomy.node(&cid("bigquery")).is_none());
        assert!(!b.catalog[0].categories.contains(&cid("bigquery")));
        assert!(b.catalog[0].categories.contains(&cid("platform")));
        assert!(b.catalog[0].categories.contains(&cid("prod")));
        assert!(b.validate().is_ok());
    }

    #[test]
    fn delete_category_promote_at_root_removes_the_tag() {
        let mut b: Bipartite<(), ()> = Bipartite {
            taxonomy: Taxonomy(vec![Category {
                id: cid("axis"),
                label: "Axis".into(),
                parent: None,
                attribute: (),
            }]),
            catalog: Catalog(vec![Resource {
                id: rid("r1"),
                label: None,
                parent: None,
                categories: vec![cid("axis")],
                attribute: (),
            }]),
        };
        b.delete_category(&cid("axis"), false);
        assert!(b.taxonomy.node(&cid("axis")).is_none());
        assert!(b.catalog[0].categories.is_empty());
        assert!(b.validate().is_ok());
    }

    #[test]
    fn delete_category_subtree_strips_descendants_from_resources() {
        let mut b = valid_bipartite();
        b.delete_category(&cid("platform"), true);
        assert!(b.taxonomy.node(&cid("platform")).is_none());
        assert!(b.taxonomy.node(&cid("bigquery")).is_none());
        assert!(b.taxonomy.node(&cid("bigtable")).is_none());
        assert_eq!(b.catalog[0].categories, vec![cid("prod")]);
        assert!(b.validate().is_ok());
    }

    #[test]
    fn resources_affected_by_delete_reports_referencing_resources() {
        let b = valid_bipartite();
        let subtree = b.resources_affected_by_delete(&cid("platform"), true);
        assert_eq!(subtree.len(), 1);
        assert_eq!(subtree[0].id, rid("r1"));
        assert!(b
            .resources_affected_by_delete(&cid("platform"), false)
            .is_empty());
        assert!(b
            .resources_affected_by_delete(&cid("bigtable"), false)
            .is_empty());
    }

    fn pivot_bipartite() -> Bipartite<(), ()> {
        fn cat(id: &str, parent: Option<&str>) -> Category<()> {
            Category {
                id: cid(id),
                label: id.into(),
                parent: parent.map(cid),
                attribute: (),
            }
        }
        fn res(id: &str, categories: &[&str]) -> Resource<(), ()> {
            Resource {
                id: rid(id),
                label: None,
                parent: None,
                categories: categories.iter().map(|c| cid(c)).collect(),
                attribute: (),
            }
        }
        Bipartite {
            taxonomy: Taxonomy(vec![
                cat("platform", None),
                cat("gcp", Some("platform")),
                cat("bigquery", Some("gcp")),
                cat("aws", Some("platform")),
                cat("s3", Some("aws")),
                cat("env", None),
                cat("prod", Some("env")),
                cat("dev", Some("env")),
                cat("team", None),
                cat("t1", Some("team")),
                cat("t2", Some("team")),
            ]),
            catalog: Catalog(vec![
                res("r1", &["bigquery", "prod", "t1"]),
                res("r2", &["s3", "prod", "t2"]),
                res("r3", &["gcp", "dev", "t1"]),
                res("r4", &["bigquery", "dev", "t2"]),
            ]),
        }
    }

    fn no_expand() -> std::collections::HashSet<Id<Category<()>>> {
        std::collections::HashSet::new()
    }

    #[test]
    fn subtree_counts_are_ancestry_aware_over_a_dimension() {
        let b = pivot_bipartite();
        let counts = b.subtree_counts(&cid("platform"), &Filters::new());
        assert_eq!(counts.get(&cid("platform")).copied(), Some(4));
        assert_eq!(counts.get(&cid("gcp")).copied(), Some(3));
        assert_eq!(counts.get(&cid("bigquery")).copied(), Some(2));
        assert_eq!(counts.get(&cid("aws")).copied(), Some(1));
        assert_eq!(counts.get(&cid("s3")).copied(), Some(1));
        assert_eq!(counts.get(&cid("dev")).copied(), None);
    }

    #[test]
    fn subtree_counts_respect_the_base_filter() {
        let b = pivot_bipartite();
        let base = Filters::from_iter([(cid("env"), cid("dev"))]);
        let counts = b.subtree_counts(&cid("platform"), &base);
        assert_eq!(counts.get(&cid("platform")).copied(), Some(2));
        assert_eq!(counts.get(&cid("gcp")).copied(), Some(2));
        assert_eq!(counts.get(&cid("aws")).copied(), None);
    }

    #[test]
    fn tree_pivot_rows_and_cols_are_direct_children_of_axes_when_collapsed() {
        let b = pivot_bipartite();
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("env"),
            &no_expand(),
            &no_expand(),
            &Filters::new(),
        );
        let rows: Vec<_> = p
            .rows
            .iter()
            .map(|n| (n.node.id.as_str(), n.depth))
            .collect();
        let cols: Vec<_> = p.cols.iter().map(|n| n.node.id.as_str()).collect();
        assert_eq!(rows, vec![("gcp", 0), ("aws", 0)]);
        assert_eq!(cols, vec!["prod", "dev"]);
    }

    #[test]
    fn tree_pivot_cell_counts_respect_ancestry_and_and() {
        let b = pivot_bipartite();
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("env"),
            &no_expand(),
            &no_expand(),
            &Filters::new(),
        );
        assert_eq!(p.count(&cid("gcp"), &cid("prod")), 1);
        assert_eq!(p.count(&cid("gcp"), &cid("dev")), 2);
        assert_eq!(p.count(&cid("aws"), &cid("prod")), 1);
        assert_eq!(p.count(&cid("aws"), &cid("dev")), 0);
    }

    #[test]
    fn tree_pivot_totals_aggregate_the_axis_subtree() {
        let b = pivot_bipartite();
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("env"),
            &no_expand(),
            &no_expand(),
            &Filters::new(),
        );
        assert_eq!(p.row_total(&cid("gcp")), 3);
        assert_eq!(p.row_total(&cid("aws")), 1);
        assert_eq!(p.col_total(&cid("prod")), 2);
        assert_eq!(p.total(), 4);
    }

    #[test]
    fn tree_pivot_base_filters_on_a_third_axis_narrow_the_grid() {
        let b = pivot_bipartite();
        let base = Filters::from_iter([(cid("team"), cid("t1"))]);
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("env"),
            &no_expand(),
            &no_expand(),
            &base,
        );
        assert_eq!(p.count(&cid("gcp"), &cid("prod")), 1);
        assert_eq!(p.count(&cid("gcp"), &cid("dev")), 1);
        assert_eq!(p.total(), 2);
    }

    #[test]
    fn tree_pivot_ignores_base_entries_on_its_own_axes() {
        let b = pivot_bipartite();
        let base = Filters::from_iter([(cid("platform"), cid("aws"))]);
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("env"),
            &no_expand(),
            &no_expand(),
            &base,
        );
        assert_eq!(p.count(&cid("gcp"), &cid("dev")), 2);
    }

    #[test]
    fn tree_pivot_expands_a_row_into_indented_children() {
        let b = pivot_bipartite();
        let row_expanded = std::collections::HashSet::from([cid("gcp")]);
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("env"),
            &row_expanded,
            &no_expand(),
            &Filters::new(),
        );
        let rows: Vec<_> = p
            .rows
            .iter()
            .map(|n| (n.node.id.as_str(), n.depth))
            .collect();
        assert_eq!(rows, vec![("gcp", 0), ("bigquery", 1), ("aws", 0)]);
        let gcp = p.rows.iter().find(|n| n.node.id == cid("gcp")).unwrap();
        let bigquery = p
            .rows
            .iter()
            .find(|n| n.node.id == cid("bigquery"))
            .unwrap();
        assert!(gcp.has_children);
        assert!(!bigquery.has_children);
    }

    #[test]
    fn tree_pivot_parent_row_total_aggregates_its_children() {
        let b = pivot_bipartite();
        let row_expanded = std::collections::HashSet::from([cid("gcp")]);
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("env"),
            &row_expanded,
            &no_expand(),
            &Filters::new(),
        );
        assert_eq!(p.row_total(&cid("gcp")), 3);
        assert_eq!(p.count(&cid("gcp"), &cid("dev")), 2);
        assert_eq!(p.row_total(&cid("bigquery")), 2);
        assert_eq!(p.count(&cid("bigquery"), &cid("dev")), 1);
    }

    #[test]
    fn tree_pivot_with_a_deep_axis_restricts_to_its_subtree() {
        let b = pivot_bipartite();
        let p = b.tree_pivot(
            &cid("gcp"),
            &cid("env"),
            &no_expand(),
            &no_expand(),
            &Filters::new(),
        );
        let rows: Vec<_> = p.rows.iter().map(|n| n.node.id.as_str()).collect();
        assert_eq!(rows, vec!["bigquery"]);
        assert_eq!(p.count(&cid("bigquery"), &cid("prod")), 1);
        assert_eq!(p.count(&cid("bigquery"), &cid("dev")), 1);
        assert_eq!(p.total(), 3);
    }

    #[test]
    fn tree_pivot_with_a_deep_axis_ignores_base_entries_on_its_root() {
        let b = pivot_bipartite();
        let base = Filters::from_iter([(cid("platform"), cid("aws"))]);
        let p = b.tree_pivot(&cid("gcp"), &cid("env"), &no_expand(), &no_expand(), &base);
        assert_eq!(p.count(&cid("bigquery"), &cid("prod")), 1);
        assert_eq!(p.count(&cid("bigquery"), &cid("dev")), 1);
    }

    #[test]
    fn tree_pivot_against_the_same_axis_is_diagonal() {
        let b = pivot_bipartite();
        let p = b.tree_pivot(
            &cid("platform"),
            &cid("platform"),
            &no_expand(),
            &no_expand(),
            &Filters::new(),
        );
        assert_eq!(p.count(&cid("gcp"), &cid("gcp")), 3);
        assert_eq!(p.count(&cid("aws"), &cid("aws")), 1);
        assert_eq!(p.count(&cid("gcp"), &cid("aws")), 0);
        assert_eq!(p.count(&cid("aws"), &cid("gcp")), 0);
        assert_eq!(p.total(), 4);
    }

    #[test]
    fn tree_pivot_with_a_leaf_row_axis_yields_a_single_row() {
        let b = pivot_bipartite();
        let p = b.tree_pivot(
            &cid("bigquery"),
            &cid("env"),
            &no_expand(),
            &no_expand(),
            &Filters::new(),
        );
        let rows: Vec<_> = p
            .rows
            .iter()
            .map(|n| (n.node.id.as_str(), n.depth, n.has_children))
            .collect();
        assert_eq!(rows, vec![("bigquery", 0, false)]);
        assert_eq!(p.count(&cid("bigquery"), &cid("prod")), 1);
        assert_eq!(p.count(&cid("bigquery"), &cid("dev")), 1);
        assert_eq!(p.row_total(&cid("bigquery")), 2);
        assert_eq!(p.total(), 2);
    }

    #[test]
    fn tree_pivot_with_a_leaf_col_axis_yields_a_single_column() {
        let b = pivot_bipartite();
        let p = b.tree_pivot(
            &cid("env"),
            &cid("bigquery"),
            &no_expand(),
            &no_expand(),
            &Filters::new(),
        );
        let cols: Vec<_> = p
            .cols
            .iter()
            .map(|n| (n.node.id.as_str(), n.depth, n.has_children))
            .collect();
        assert_eq!(cols, vec![("bigquery", 0, false)]);
        assert_eq!(p.count(&cid("prod"), &cid("bigquery")), 1);
        assert_eq!(p.count(&cid("dev"), &cid("bigquery")), 1);
        assert_eq!(p.col_total(&cid("bigquery")), 2);
        assert_eq!(p.total(), 2);
    }
}
