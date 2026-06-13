use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::forest::{Forest, ForestNode};
use crate::id::Id;

/// parent が None のノードが軸の根（＝カテゴリキー）となる。
/// リソースの categories は { 根id → ノードid } で表現する。
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

    pub fn subtree_flat<'a>(
        &'a self,
        root_id: &'a Id<Category<CA>>,
    ) -> Vec<(&'a Category<CA>, usize, bool, &'a Id<Category<CA>>)> {
        let mut out = Vec::new();
        self.subtree_flat_rec(root_id, 0, &mut out);
        out
    }

    fn subtree_flat_rec<'a>(
        &'a self,
        parent_id: &'a Id<Category<CA>>,
        depth: usize,
        out: &mut Vec<(&'a Category<CA>, usize, bool, &'a Id<Category<CA>>)>,
    ) {
        for child in self.children_of(parent_id) {
            let has_children = !self.children_of(&child.id).is_empty();
            out.push((child, depth, has_children, parent_id));
            self.subtree_flat_rec(&child.id, depth + 1, out);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn test_forest() -> Taxonomy<()> {
        // platform > cloud > gcp > bigquery / bigtable
        //                  > aws > s3
        Taxonomy(vec![
            Category {
                id: "platform".into(),
                label: "Platform".into(),
                parent: None,
                attribute: (),
            },
            Category {
                id: "cloud".into(),
                label: "Cloud".into(),
                parent: Some("platform".into()),
                attribute: (),
            },
            Category {
                id: "gcp".into(),
                label: "GCP".into(),
                parent: Some("cloud".into()),
                attribute: (),
            },
            Category {
                id: "bigquery".into(),
                label: "BigQuery".into(),
                parent: Some("gcp".into()),
                attribute: (),
            },
            Category {
                id: "bigtable".into(),
                label: "Bigtable".into(),
                parent: Some("gcp".into()),
                attribute: (),
            },
            Category {
                id: "aws".into(),
                label: "AWS".into(),
                parent: Some("cloud".into()),
                attribute: (),
            },
            Category {
                id: "s3".into(),
                label: "S3".into(),
                parent: Some("aws".into()),
                attribute: (),
            },
        ])
    }

    #[test]
    fn ancestry_walks_to_root_exclusive() {
        let f = test_forest();
        assert_eq!(
            f.ancestry(&"bigquery".into()),
            vec!["bigquery".into(), "gcp".into(), "cloud".into()]
        );
        assert_eq!(
            f.ancestry(&"cloud".into()),
            vec![Id::<Category<()>>::from("cloud")]
        );
        assert_eq!(
            f.ancestry(&"unknown".into()),
            Vec::<Id<Category<()>>>::new()
        );
    }

    #[test]
    fn ancestry_contains_detects_ancestor() {
        let f = test_forest();
        assert!(f.ancestry_contains(&"bigquery".into(), &"gcp".into()));
        assert!(f.ancestry_contains(&"bigquery".into(), &"cloud".into()));
        assert!(!f.ancestry_contains(&"bigquery".into(), &"s3".into()));
        assert!(!f.ancestry_contains(&"bigquery".into(), &"bigtable".into()));
    }

    #[test]
    fn collect_descendants_includes_self_and_all_children() {
        let f = test_forest();
        let desc = f.collect_descendants(&"gcp".into());
        assert!(desc.contains("gcp"));
        assert!(desc.contains("bigquery"));
        assert!(desc.contains("bigtable"));
        assert!(!desc.contains("s3"));
    }
}
