use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{Errors, ForestError};
use crate::forest::{Forest, ForestNode};
use crate::id::Id;

/// カテゴリ木の各ノード。categories は値 id のリストで、軸（根）は root_of で導出する。
/// 根を含むすべてのノードが値になりうる（一様化方針）。
/// `#[serde(bound)]` でデシリアライズ境界を明示し、flatten が生成する CA: Default 境界を除去する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound(serialize = "CA: Serialize", deserialize = "CA: Deserialize<'de>"))]
pub struct Category<CA> {
    pub id: Id<Category<CA>>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Category<CA>>>,
    /// web 層は `CA = CategoryAttribute { color }` を指定して color を同じ JSON レベルに展開する。
    #[serde(flatten)]
    pub attribute: CA,
}

impl<CA> ForestNode for Category<CA> {
    fn id(&self) -> &Id<Self> {
        &self.id
    }
    fn parent(&self) -> Option<&Id<Self>> {
        self.parent.as_ref()
    }
}

/// parent リンクで森を構成する。parent が None のノードが軸の根となる。
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

    /// 深さ優先で子孫を列挙し、counts > 0 のノードのみ (id, label, depth, count) を out に追加する。
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

    pub fn reparent(&mut self, dragged: &Id<Category<CA>>, new_parent: Option<Id<Category<CA>>>) {
        if let Some(np) = &new_parent {
            if np == dragged || self.ancestry_contains(np, dragged) {
                return;
            }
        }
        if let Some(n) = self.iter_mut().find(|n| &n.id == dragged) {
            n.parent = new_parent;
        }
    }

    pub fn move_relative(
        &mut self,
        dragged: &Id<Category<CA>>,
        target: &Id<Category<CA>>,
        after: bool,
    ) {
        if dragged == target {
            return;
        }
        let new_parent = self
            .iter()
            .find(|n| &n.id == target)
            .and_then(|n| n.parent.clone());
        if let Some(np) = &new_parent {
            if self.ancestry_contains(np, dragged) {
                return;
            }
        }
        let Some(dpos) = self.iter().position(|n| &n.id == dragged) else {
            return;
        };
        let mut node = self.remove(dpos);
        node.parent = new_parent;
        let tpos = self
            .iter()
            .position(|n| &n.id == target)
            .unwrap_or(self.len());
        let insert_at = if after { tpos + 1 } else { tpos };
        let len = self.len();
        self.insert(insert_at.min(len), node);
    }

    pub fn delete_promote(&mut self, node_id: &Id<Category<CA>>) {
        let parent = self
            .iter()
            .find(|n| &n.id == node_id)
            .and_then(|n| n.parent.clone());
        for child in self.iter_mut() {
            if child.parent.as_ref() == Some(node_id) {
                child.parent = parent.clone();
            }
        }
        self.retain(|n| &n.id != node_id);
    }

    pub fn delete_subtree(&mut self, node_id: &Id<Category<CA>>) {
        let doomed = self.collect_descendants(node_id);
        self.retain(|n| !doomed.contains(n.id.as_str()));
    }

    pub fn rename_node(
        &mut self,
        old_id: &Id<Category<CA>>,
        new_id: Id<Category<CA>>,
        label: &str,
        attribute: CA,
    ) {
        if old_id != &new_id {
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
    }
}

impl<CA> Taxonomy<CA> {
    /// 森の構造整合性を検証してから構築する。検証を通った場合のみ Ok を返す。
    pub fn try_new(nodes: Vec<Category<CA>>) -> Result<Self, Errors<ForestError>> {
        Taxonomy(nodes).validated()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn id(s: &str) -> Id<Category<()>> {
        s.try_into().unwrap()
    }

    pub fn test_forest() -> Taxonomy<()> {
        // platform > cloud > gcp > bigquery / bigtable
        //                  > aws > s3
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
    fn ancestry_walks_to_root_inclusive() {
        let f = test_forest();
        // 根も値になりうるため、ancestry は根を含めて返す
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
        // root_of は存在するノードに対して total（根自身も Some を返す）
        assert_eq!(f.root_of(&id("bigquery")), Some(id("platform")));
        assert_eq!(f.root_of(&id("gcp")), Some(id("platform")));
        assert_eq!(f.root_of(&id("cloud")), Some(id("platform")));
        assert_eq!(f.root_of(&id("platform")), Some(id("platform")));
        // 存在しない id は None
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

    // --- try_new のテスト ---

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
}
