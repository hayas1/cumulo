use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{Errors, ForestError};
use crate::forest::{Forest, ForestMut, ForestNode};
use crate::id::Id;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound(serialize = "CA: Serialize", deserialize = "CA: Deserialize<'de>"))]
pub struct Category<CA> {
    pub id: Id<Category<CA>>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Category<CA>>>,
    #[serde(flatten)]
    pub attribute: CA,
}

impl<CA> Category<CA> {
    pub fn display_label(&self) -> &str {
        if self.label.is_empty() {
            self.id.as_str()
        } else {
            &self.label
        }
    }
}

impl<CA> ForestNode for Category<CA> {
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
pub struct Taxonomy<CA>(pub Vec<Category<CA>>);

impl<CA> Default for Taxonomy<CA> {
    fn default() -> Self {
        Taxonomy(Vec::new())
    }
}

impl<CA> std::ops::Deref for Taxonomy<CA> {
    type Target = Vec<Category<CA>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<CA> std::ops::DerefMut for Taxonomy<CA> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<CA> Forest for Taxonomy<CA> {
    type Node = Category<CA>;
    fn nodes(&self) -> &[Category<CA>] {
        &self.0
    }
}

impl<CA> ForestMut for Taxonomy<CA> {
    fn nodes_mut(&mut self) -> &mut Vec<Category<CA>> {
        &mut self.0
    }
}

impl<CA> Taxonomy<CA> {
    pub fn dfs_order(
        &self,
        root_id: &Id<Category<CA>>,
        collapsed: &HashSet<Id<Category<CA>>>,
    ) -> Vec<(Id<Category<CA>>, usize, bool)> {
        let mut out = Vec::new();
        self.dfs_order_rec(root_id, 0, collapsed, &mut out);
        out
    }

    fn dfs_order_rec(
        &self,
        parent_id: &Id<Category<CA>>,
        depth: usize,
        collapsed: &HashSet<Id<Category<CA>>>,
        out: &mut Vec<(Id<Category<CA>>, usize, bool)>,
    ) {
        for child in self.children_of(parent_id) {
            let has_children = !self.children_of(&child.id).is_empty();
            out.push((child.id.clone(), depth, has_children));
            if has_children && !collapsed.contains(child.id.as_str()) {
                self.dfs_order_rec(&child.id, depth + 1, collapsed, out);
            }
        }
    }

    pub fn dfs_collect_counts(
        &self,
        parent_id: &Id<Category<CA>>,
        depth: usize,
        counts: &HashMap<Id<Category<CA>>, usize>,
        out: &mut Vec<(Id<Category<CA>>, String, usize, usize)>,
    ) {
        for child in self.children_of(parent_id) {
            let cnt = counts.get(child.id.as_str()).copied().unwrap_or(0);
            if cnt == 0 {
                continue;
            }
            out.push((child.id.clone(), child.label.clone(), depth, cnt));
            self.dfs_collect_counts(&child.id, depth + 1, counts, out);
        }
    }

    pub fn rename_node(
        &mut self,
        old_id: &Id<Category<CA>>,
        new_id: Id<Category<CA>>,
        label: &str,
        attribute: CA,
    ) -> Result<(), ForestError> {
        if old_id != &new_id {
            if self.iter().any(|n| n.id == new_id) {
                return Err(ForestError::DuplicateId {
                    id: new_id.as_str().to_string(),
                });
            }
            for other in self.iter_mut() {
                if other.parent.as_ref() == Some(old_id) {
                    other.parent = Some(new_id.clone());
                }
            }
        }
        if let Some(n) = self.iter_mut().find(|n| &n.id == old_id) {
            n.id = new_id;
            n.label = label.to_string();
            n.attribute = attribute;
        }
        Ok(())
    }
}

impl<CA> Taxonomy<CA> {
    pub fn try_new(nodes: Vec<Category<CA>>) -> Result<Self, Errors<ForestError>> {
        Taxonomy(nodes).validated()
    }

    pub fn label_of(&self, id: &Id<Category<CA>>) -> String {
        self.node(id)
            .map(|n| n.display_label().to_string())
            .unwrap_or_else(|| id.to_string())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn id(s: &str) -> Id<Category<()>> {
        s.try_into().unwrap()
    }

    pub fn test_forest() -> Taxonomy<()> {
        Taxonomy(vec![
            Category {
                id: id("platform"),
                label: "Platform".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: id("cloud"),
                label: "Cloud".into(),
                parent: Some(id("platform")),
                attribute: (),
            },
            Category {
                id: id("gcp"),
                label: "GCP".into(),
                parent: Some(id("cloud")),
                attribute: (),
            },
            Category {
                id: id("bigquery"),
                label: "BigQuery".into(),
                parent: Some(id("gcp")),
                attribute: (),
            },
            Category {
                id: id("bigtable"),
                label: "Bigtable".into(),
                parent: Some(id("gcp")),
                attribute: (),
            },
            Category {
                id: id("aws"),
                label: "AWS".into(),
                parent: Some(id("cloud")),
                attribute: (),
            },
            Category {
                id: id("s3"),
                label: "S3".into(),
                parent: Some(id("aws")),
                attribute: (),
            },
        ])
    }

    #[test]
    fn display_label_returns_label_when_present() {
        let c = Category {
            id: id("gcp"),
            label: "GCP".into(),
            parent: None,
            attribute: (),
        };
        assert_eq!(c.display_label(), "GCP");
    }

    #[test]
    fn display_label_falls_back_to_id_when_label_empty() {
        let c = Category {
            id: id("gcp"),
            label: String::new(),
            parent: None,
            attribute: (),
        };
        assert_eq!(c.display_label(), "gcp");
    }

    #[test]
    fn label_of_resolves_the_node_label() {
        let f = test_forest();
        assert_eq!(f.label_of(&id("bigquery")), "BigQuery");
    }

    #[test]
    fn label_of_falls_back_to_id_for_missing_node() {
        let f = test_forest();
        assert_eq!(f.label_of(&id("ghost")), "ghost");
    }

    #[test]
    fn label_of_falls_back_to_id_for_empty_label() {
        let f = Taxonomy(vec![Category {
            id: id("x"),
            label: String::new(),
            parent: None,
            attribute: (),
        }]);
        assert_eq!(f.label_of(&id("x")), "x");
    }

    #[test]
    fn ancestry_walks_to_root_inclusive() {
        let f = test_forest();
        assert_eq!(
            f.ancestry(&id("bigquery")),
            vec![id("bigquery"), id("gcp"), id("cloud"), id("platform")]
        );
        assert_eq!(f.ancestry(&id("cloud")), vec![id("cloud"), id("platform")]);
        assert_eq!(f.ancestry(&id("unknown")), Vec::<Id<Category<()>>>::new());
    }

    #[test]
    fn root_of_is_total_for_existing_nodes() {
        use crate::forest::Forest;
        let f = test_forest();
        assert_eq!(f.root_of(&id("bigquery")), Some(id("platform")));
        assert_eq!(f.root_of(&id("gcp")), Some(id("platform")));
        assert_eq!(f.root_of(&id("cloud")), Some(id("platform")));
        assert_eq!(f.root_of(&id("platform")), Some(id("platform")));
        assert_eq!(f.root_of(&id("unknown")), None);
    }

    #[test]
    fn ancestry_contains_detects_ancestor() {
        let f = test_forest();
        assert!(f.ancestry_contains(&id("bigquery"), &id("gcp")));
        assert!(f.ancestry_contains(&id("bigquery"), &id("cloud")));
        assert!(!f.ancestry_contains(&id("bigquery"), &id("s3")));
        assert!(!f.ancestry_contains(&id("bigquery"), &id("bigtable")));
    }

    #[test]
    fn collect_descendants_includes_self_and_all_children() {
        let f = test_forest();
        let desc = f.collect_descendants(&id("gcp"));
        assert!(desc.contains("gcp"));
        assert!(desc.contains("bigquery"));
        assert!(desc.contains("bigtable"));
        assert!(!desc.contains("s3"));
    }

    #[test]
    fn try_new_returns_ok_for_valid_nodes() {
        let nodes = vec![
            Category {
                id: id("root"),
                label: "Root".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: id("child"),
                label: "Child".into(),
                parent: Some(id("root")),
                attribute: (),
            },
        ];
        assert!(Taxonomy::try_new(nodes).is_ok());
    }

    #[test]
    fn try_new_returns_err_for_duplicate_ids() {
        use crate::error::ForestError;
        let nodes = vec![
            Category {
                id: id("dup"),
                label: "A".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: id("dup"),
                label: "B".into(),
                parent: None,
                attribute: (),
            },
        ];
        let err = Taxonomy::try_new(nodes).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, ForestError::DuplicateId { id } if id == "dup")));
    }

    #[test]
    fn try_new_returns_err_for_dangling_parent() {
        use crate::error::ForestError;
        let nodes = vec![Category {
            id: id("child"),
            label: "Child".into(),
            parent: Some(id("ghost")),
            attribute: (),
        }];
        let err = Taxonomy::try_new(nodes).unwrap_err();
        assert!(err
            .iter()
            .any(|e| matches!(e, ForestError::DanglingParent { id, parent }
            if id == "child" && parent == "ghost")));
    }

    #[test]
    fn try_new_empty_is_ok() {
        assert!(Taxonomy::<()>::try_new(vec![]).is_ok());
    }

    #[test]
    fn rename_to_fresh_id_updates_node_and_children() {
        let mut t = Taxonomy(vec![
            Category {
                id: id("old"),
                label: "Old".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: id("child"),
                label: "Child".into(),
                parent: Some(id("old")),
                attribute: (),
            },
        ]);
        t.rename_node(&id("old"), id("new"), "New", ()).unwrap();
        assert!(t.node(&id("old")).is_none());
        assert_eq!(t.node(&id("new")).unwrap().label, "New");
        assert_eq!(t.node(&id("child")).unwrap().parent, Some(id("new")));
    }

    #[test]
    fn rename_keeping_id_updates_label_only() {
        let mut t = Taxonomy(vec![Category {
            id: id("a"),
            label: "A".into(),
            parent: None,
            attribute: (),
        }]);
        t.rename_node(&id("a"), id("a"), "A2", ()).unwrap();
        assert_eq!(t.node(&id("a")).unwrap().label, "A2");
    }

    #[test]
    fn rename_to_existing_id_is_rejected() {
        use crate::error::ForestError;
        let mut t = Taxonomy(vec![
            Category {
                id: id("a"),
                label: "A".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: id("b"),
                label: "B".into(),
                parent: None,
                attribute: (),
            },
        ]);
        let err = t.rename_node(&id("a"), id("b"), "A2", ()).unwrap_err();
        assert!(matches!(err, ForestError::DuplicateId { id } if id == "b"));
        assert_eq!(t.node(&id("a")).unwrap().label, "A");
        assert_eq!(t.node(&id("b")).unwrap().label, "B");
    }

    #[test]
    fn reparent_to_valid_parent_succeeds() {
        let mut t = test_forest();
        t.reparent(&id("bigquery"), Some(id("aws"))).unwrap();
        assert_eq!(t.node(&id("bigquery")).unwrap().parent, Some(id("aws")));
    }

    #[test]
    fn reparent_to_root_succeeds() {
        let mut t = test_forest();
        t.reparent(&id("bigquery"), None).unwrap();
        assert_eq!(t.node(&id("bigquery")).unwrap().parent, None);
    }

    #[test]
    fn reparent_under_self_is_rejected_as_cycle() {
        let mut t = test_forest();
        let err = t.reparent(&id("gcp"), Some(id("gcp"))).unwrap_err();
        assert!(matches!(err, ForestError::Cycle { id } if id == "gcp"));
        assert_eq!(t.node(&id("gcp")).unwrap().parent, Some(id("cloud")));
    }

    #[test]
    fn reparent_under_descendant_is_rejected_as_cycle() {
        let mut t = test_forest();
        let err = t.reparent(&id("gcp"), Some(id("bigquery"))).unwrap_err();
        assert!(matches!(err, ForestError::Cycle { id } if id == "gcp"));
        assert_eq!(t.node(&id("gcp")).unwrap().parent, Some(id("cloud")));
    }

    #[test]
    fn move_relative_reorders_under_same_parent_and_succeeds() {
        let mut t = test_forest();
        t.move_relative(&id("bigtable"), &id("bigquery"), false)
            .unwrap();
        assert_eq!(t.node(&id("bigtable")).unwrap().parent, Some(id("gcp")));
    }

    #[test]
    fn move_relative_same_node_is_noop_ok() {
        let mut t = test_forest();
        t.move_relative(&id("gcp"), &id("gcp"), true).unwrap();
        assert_eq!(t.node(&id("gcp")).unwrap().parent, Some(id("cloud")));
    }

    #[test]
    fn move_relative_into_descendant_is_rejected_as_cycle() {
        let mut t = test_forest();
        let err = t
            .move_relative(&id("gcp"), &id("bigquery"), false)
            .unwrap_err();
        assert!(matches!(err, ForestError::Cycle { id } if id == "gcp"));
        assert_eq!(t.node(&id("gcp")).unwrap().parent, Some(id("cloud")));
    }
}
