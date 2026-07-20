use serde::{Deserialize, Serialize};

use crate::category::{Category, Taxonomy};
use crate::error::{Errors, ForestError};
use crate::forest::{Forest, ForestMut, ForestNode};
use crate::id::Id;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound(
    serialize = "RA: Serialize, CA: Serialize",
    deserialize = "RA: Deserialize<'de>, CA: Deserialize<'de>"
))]
pub struct Resource<RA, CA> {
    pub id: Id<Resource<RA, CA>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Resource<RA, CA>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<Id<Category<CA>>>,
    #[serde(flatten)]
    pub attribute: RA,
}

pub type RootedNode<CA> = (Id<Category<CA>>, Id<Category<CA>>);

impl<RA, CA: Clone> Resource<RA, CA> {
    pub fn resolved_label(&self, forest: &Taxonomy<CA>) -> Option<String> {
        if let Some(l) = &self.label {
            if !l.is_empty() {
                return Some(l.clone());
            }
        }
        let mut parts: Vec<String> = self
            .categories
            .iter()
            .filter_map(|v| forest.node(v))
            .map(|n| n.display_label().to_string())
            .collect();
        parts.sort();
        (!parts.is_empty()).then(|| parts.join(" / "))
    }

    pub fn rooted_nodes(&self, taxonomy: &Taxonomy<CA>) -> Vec<RootedNode<CA>> {
        let mut pairs: Vec<_> = self
            .categories
            .iter()
            .map(|v| (taxonomy.root_of(v).unwrap_or_else(|| v.clone()), v.clone()))
            .collect();
        pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
        pairs
    }

    pub fn category<'a>(
        &'a self,
        taxonomy: &Taxonomy<CA>,
        root_id: &Id<Category<CA>>,
    ) -> Option<&'a Id<Category<CA>>> {
        self.categories
            .iter()
            .find(|c| taxonomy.root_of(c).as_ref() == Some(root_id))
    }
}

impl<RA, CA> ForestNode for Resource<RA, CA> {
    fn id(&self) -> &Id<Self> {
        &self.id
    }
    fn parent(&self) -> Option<&Id<Self>> {
        self.parent.as_ref()
    }
    fn set_parent(&mut self, parent: Option<Id<Self>>) {
        self.parent = parent;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Catalog<RA, CA>(pub Vec<Resource<RA, CA>>);

impl<RA, CA> Default for Catalog<RA, CA> {
    fn default() -> Self {
        Catalog(Vec::new())
    }
}

impl<RA, CA> std::ops::Deref for Catalog<RA, CA> {
    type Target = Vec<Resource<RA, CA>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<RA, CA> std::ops::DerefMut for Catalog<RA, CA> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<RA, CA> Forest for Catalog<RA, CA> {
    type Node = Resource<RA, CA>;
    fn nodes(&self) -> &[Resource<RA, CA>] {
        &self.0
    }
}

impl<RA, CA> ForestMut for Catalog<RA, CA> {
    fn nodes_mut(&mut self) -> &mut Vec<Resource<RA, CA>> {
        &mut self.0
    }
}

impl<RA, CA> Catalog<RA, CA> {
    pub fn try_new(nodes: Vec<Resource<RA, CA>>) -> Result<Self, Errors<ForestError>> {
        Catalog(nodes).validated()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::{Category, Taxonomy};

    fn id(s: &str) -> Id<Resource<(), ()>> {
        s.try_into().unwrap()
    }

    fn cid(s: &str) -> Id<crate::category::Category<()>> {
        s.try_into().unwrap()
    }

    #[test]
    fn resolved_label_uses_explicit_label_when_present() {
        let tax: Taxonomy<()> = Taxonomy(vec![]);
        let r = Resource {
            id: id("r1"),
            label: Some("My Label".into()),
            parent: None,
            categories: Vec::new(),
            attribute: (),
        };
        assert_eq!(r.resolved_label(&tax), Some("My Label".to_string()));
    }

    #[test]
    fn resolved_label_composes_category_labels_sorted() {
        let tax: Taxonomy<()> = Taxonomy(vec![
            Category {
                id: cid("bigquery"),
                label: "BigQuery".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: cid("prod"),
                label: "prod".into(),
                parent: None,
                attribute: (),
            },
        ]);
        let r = Resource {
            id: id("r1"),
            label: None,
            parent: None,
            categories: vec![cid("prod"), cid("bigquery")],
            attribute: (),
        };
        assert_eq!(r.resolved_label(&tax), Some("BigQuery / prod".to_string()));
    }

    #[test]
    fn resolved_label_is_none_when_no_label_and_no_categories() {
        let tax: Taxonomy<()> = Taxonomy(vec![]);
        let r = Resource {
            id: id("r1"),
            label: None,
            parent: None,
            categories: Vec::new(),
            attribute: (),
        };
        assert_eq!(r.resolved_label(&tax), None);
    }

    #[test]
    fn resolved_label_treats_empty_label_as_absent() {
        let tax: Taxonomy<()> = Taxonomy(vec![]);
        let r = Resource {
            id: id("r1"),
            label: Some(String::new()),
            parent: None,
            categories: Vec::new(),
            attribute: (),
        };
        assert_eq!(r.resolved_label(&tax), None);
    }

    #[test]
    fn rooted_nodes_pairs_each_node_with_its_root_sorted() {
        let tax: Taxonomy<()> = Taxonomy(vec![
            Category {
                id: cid("platform"),
                label: "P".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: cid("bigquery"),
                label: "BQ".into(),
                parent: Some(cid("platform")),
                attribute: (),
            },
            Category {
                id: cid("env"),
                label: "E".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: cid("prod"),
                label: "prod".into(),
                parent: Some(cid("env")),
                attribute: (),
            },
        ]);
        let r = Resource {
            id: id("r1"),
            label: None,
            parent: None,
            categories: vec![cid("bigquery"), cid("prod")],
            attribute: (),
        };
        assert_eq!(
            r.rooted_nodes(&tax),
            vec![
                (cid("env"), cid("prod")),
                (cid("platform"), cid("bigquery"))
            ]
        );
    }

    #[test]
    fn delete_promote_lifts_children_to_grandparent() {
        let mut c = test_catalog();
        c.delete_promote(&id("gcp"));
        assert!(c.node(&id("gcp")).is_none());
        assert_eq!(c.node(&id("bigquery")).unwrap().parent, None);
        assert_eq!(c.node(&id("bigtable")).unwrap().parent, None);
    }

    #[test]
    fn delete_subtree_removes_node_and_descendants() {
        let mut c = test_catalog();
        c.delete_subtree(&id("gcp"));
        assert!(c.is_empty());
    }

    #[test]
    fn reparent_moves_resource_under_new_parent() {
        let mut c = test_catalog();
        c.reparent(&id("bigquery"), None).unwrap();
        assert_eq!(c.node(&id("bigquery")).unwrap().parent, None);
    }

    #[test]
    fn reparent_under_descendant_is_rejected_as_cycle() {
        let mut c = test_catalog();
        let err = c.reparent(&id("gcp"), Some(id("bigquery"))).unwrap_err();
        assert!(matches!(err, ForestError::Cycle { id } if id == "gcp"));
        assert_eq!(c.node(&id("gcp")).unwrap().parent, None);
    }

    #[test]
    fn move_relative_reorders_and_reparents_resource() {
        let mut c = test_catalog();
        c.move_relative(&id("bigtable"), &id("bigquery"), false)
            .unwrap();
        assert_eq!(c.node(&id("bigtable")).unwrap().parent, Some(id("gcp")));
    }

    fn test_catalog() -> Catalog<(), ()> {
        Catalog(vec![
            Resource {
                id: id("gcp"),
                label: None,
                parent: None,
                categories: Vec::new(),
                attribute: (),
            },
            Resource {
                id: id("bigquery"),
                label: None,
                parent: Some(id("gcp")),
                categories: Vec::new(),
                attribute: (),
            },
            Resource {
                id: id("bigtable"),
                label: None,
                parent: Some(id("gcp")),
                categories: Vec::new(),
                attribute: (),
            },
        ])
    }

    #[test]
    fn roots_are_parentless() {
        let c = test_catalog();
        let roots: Vec<_> = c
            .roots()
            .iter()
            .map(|r| r.id.as_str().to_string())
            .collect();
        assert_eq!(roots, vec!["gcp"]);
    }

    #[test]
    fn children_of_lists_direct_children() {
        let c = test_catalog();
        let mut kids: Vec<_> = c
            .children_of(&id("gcp"))
            .iter()
            .map(|r| r.id.as_str().to_string())
            .collect();
        kids.sort();
        assert_eq!(kids, vec!["bigquery", "bigtable"]);
    }

    #[test]
    fn ancestry_walks_to_root_inclusive() {
        let c = test_catalog();
        assert_eq!(c.ancestry(&id("bigquery")), vec![id("bigquery"), id("gcp")]);
        assert_eq!(c.ancestry(&id("gcp")), vec![id("gcp")]);
    }

    #[test]
    fn try_new_returns_ok_for_valid_nodes() {
        let nodes = vec![
            Resource {
                id: id("root"),
                label: None,
                parent: None,
                categories: Vec::new(),
                attribute: (),
            },
            Resource {
                id: id("child"),
                label: None,
                parent: Some(id("root")),
                categories: Vec::new(),
                attribute: (),
            },
        ];
        assert!(Catalog::try_new(nodes).is_ok());
    }

    #[test]
    fn try_new_returns_err_for_duplicate_ids() {
        use crate::error::ForestError;
        let nodes = vec![
            Resource {
                id: id("dup"),
                label: None,
                parent: None,
                categories: Vec::new(),
                attribute: (),
            },
            Resource {
                id: id("dup"),
                label: None,
                parent: None,
                categories: Vec::new(),
                attribute: (),
            },
        ];
        let err = Catalog::<(), ()>::try_new(nodes).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, ForestError::DuplicateId { id } if id == "dup")));
    }

    #[test]
    fn try_new_returns_err_for_dangling_parent() {
        use crate::error::ForestError;
        let nodes = vec![Resource {
            id: id("r1"),
            label: None,
            parent: Some(id("ghost")),
            categories: Vec::new(),
            attribute: (),
        }];
        let err = Catalog::<(), ()>::try_new(nodes).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, ForestError::DanglingParent { id, parent }
            if id == "r1" && parent == "ghost")));
    }

    #[test]
    fn try_new_empty_is_ok() {
        assert!(Catalog::<(), ()>::try_new(vec![]).is_ok());
    }
}
