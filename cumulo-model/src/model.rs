use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub use crate::id::Id;

/// UI などが追加するビジュアル属性を持たない既定値。
/// `#[serde(flatten)]` で展開されるので JSON に余分なフィールドは追加されない。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct NoValue {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Entity<V = NoValue> {
    pub id: Id<Entity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// キーは軸の根id。値はその軸内のノードid。
    pub attributes: HashMap<Id<Attribute>, Id<Attribute>>,
    #[serde(flatten)]
    pub value: V,
}

impl<V: Default> Default for Entity<V> {
    fn default() -> Self {
        Entity {
            id: Id::<Entity>::default(),
            label: None,
            attributes: HashMap::new(),
            value: V::default(),
        }
    }
}

impl<V> Entity<V> {
    pub fn display_label<A: Clone>(&self, forest: &AttributeForest<A>) -> String {
        if let Some(l) = &self.label {
            if !l.is_empty() {
                return l.clone();
            }
        }
        let mut parts: Vec<String> = self
            .attributes
            .values()
            .filter_map(|v| forest.node(v))
            .map(|n| {
                if n.label.is_empty() {
                    n.id.to_string()
                } else {
                    n.label.clone()
                }
            })
            .collect();
        parts.sort();
        if parts.is_empty() {
            "(名前なし)".to_string()
        } else {
            parts.join(" / ")
        }
    }

    pub fn attribute(&self, root_id: &Id<Attribute>) -> Option<&Id<Attribute>> {
        self.attributes.get(root_id)
    }
}

/// parent が None のノードが軸の根（＝属性キー）となる。
/// エンティティの value は { 根id → ノードid } で表現する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Attribute<A = NoValue> {
    pub id: Id<Attribute>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Attribute>>,
    /// `A = NoValue` のとき flatten は何も追加しない。
    /// web 層は `A = AttributeValue { color }` を指定して color を同じ JSON レベルに展開する。
    #[serde(flatten)]
    pub value: A,
}

/// parent リンクで森を構成する。parent が None のノードが軸の根となる。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(transparent)]
pub struct AttributeForest<A = NoValue>(pub Vec<Attribute<A>>);

impl<A> std::ops::Deref for AttributeForest<A> {
    type Target = Vec<Attribute<A>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A> std::ops::DerefMut for AttributeForest<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<A> AttributeForest<A> {
    pub fn roots(&self) -> Vec<&Attribute<A>> {
        self.iter().filter(|n| n.parent.is_none()).collect()
    }

    pub fn children_of(&self, parent_id: &Id<Attribute>) -> Vec<&Attribute<A>> {
        self.iter()
            .filter(|n| n.parent.as_ref() == Some(parent_id))
            .collect()
    }

    /// 軸の根 (parent==None) の id は含めない（根はキーであり値ではない）。
    pub fn ancestry(&self, id: &Id<Attribute>) -> Vec<Id<Attribute>> {
        let mut chain = Vec::new();
        let mut cur = Some(id.clone());
        while let Some(c) = cur {
            if chain.contains(&c) {
                break;
            }
            let found = self.iter().find(|n| n.id == c);
            let parent = found.and_then(|n| n.parent.clone());
            if parent.is_none() {
                break;
            }
            chain.push(c);
            cur = parent;
        }
        chain
    }

    pub fn root_of(&self, id: &Id<Attribute>) -> Option<Id<Attribute>> {
        let mut cur = id.clone();
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(cur.clone()) {
                return None;
            }
            match self.iter().find(|n| n.id == cur) {
                None => return None,
                Some(n) => match &n.parent {
                    None => return None,
                    Some(p) => {
                        if self
                            .iter()
                            .find(|n| &n.id == p)
                            .is_some_and(|n| n.parent.is_none())
                        {
                            return Some(p.clone());
                        }
                        cur = p.clone();
                    }
                },
            }
        }
    }

    pub fn node(&self, id: &Id<Attribute>) -> Option<&Attribute<A>> {
        self.iter().find(|n| &n.id == id)
    }

    pub fn ancestry_contains(&self, start: &Id<Attribute>, target: &Id<Attribute>) -> bool {
        let mut cur = Some(start.clone());
        let mut seen = HashSet::new();
        while let Some(c) = cur {
            if &c == target {
                return true;
            }
            if !seen.insert(c.clone()) {
                break;
            }
            cur = self
                .iter()
                .find(|n| n.id == c)
                .and_then(|n| n.parent.clone());
        }
        false
    }

    pub fn collect_descendants(&self, root: &Id<Attribute>) -> HashSet<Id<Attribute>> {
        let mut out = HashSet::new();
        self.collect_descendants_rec(root, &mut out);
        out
    }

    fn collect_descendants_rec(&self, id: &Id<Attribute>, out: &mut HashSet<Id<Attribute>>) {
        if !out.insert(id.clone()) {
            return;
        }
        for child in self.children_of(id) {
            self.collect_descendants_rec(&child.id, out);
        }
    }

    pub fn dfs_order(
        &self,
        root_id: &Id<Attribute>,
        collapsed: &HashSet<Id<Attribute>>,
    ) -> Vec<(Id<Attribute>, usize, bool)> {
        let mut out = Vec::new();
        self.dfs_order_rec(root_id, 0, collapsed, &mut out);
        out
    }

    fn dfs_order_rec(
        &self,
        parent_id: &Id<Attribute>,
        depth: usize,
        collapsed: &HashSet<Id<Attribute>>,
        out: &mut Vec<(Id<Attribute>, usize, bool)>,
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
        parent_id: &Id<Attribute>,
        depth: usize,
        counts: &HashMap<Id<Attribute>, usize>,
        out: &mut Vec<(Id<Attribute>, String, usize, usize)>,
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

    pub fn reparent(&mut self, dragged: &Id<Attribute>, new_parent: Option<Id<Attribute>>) {
        if let Some(np) = &new_parent {
            if np == dragged || self.ancestry_contains(np, dragged) {
                return;
            }
        }
        if let Some(n) = self.iter_mut().find(|n| &n.id == dragged) {
            n.parent = new_parent;
        }
    }

    pub fn move_relative(&mut self, dragged: &Id<Attribute>, target: &Id<Attribute>, after: bool) {
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

    pub fn delete_promote(&mut self, node_id: &Id<Attribute>) {
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

    pub fn delete_subtree(&mut self, node_id: &Id<Attribute>) {
        let doomed = self.collect_descendants(node_id);
        self.retain(|n| !doomed.contains(n.id.as_str()));
    }

    pub fn rename_node(
        &mut self,
        old_id: &Id<Attribute>,
        new_id: Id<Attribute>,
        label: &str,
        value: A,
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
            n.value = value;
        }
    }

    pub fn subtree_flat<'a>(
        &'a self,
        root_id: &'a Id<Attribute>,
    ) -> Vec<(&'a Attribute<A>, usize, bool, &'a Id<Attribute>)> {
        let mut out = Vec::new();
        self.subtree_flat_rec(root_id, 0, &mut out);
        out
    }

    fn subtree_flat_rec<'a>(
        &'a self,
        parent_id: &'a Id<Attribute>,
        depth: usize,
        out: &mut Vec<(&'a Attribute<A>, usize, bool, &'a Id<Attribute>)>,
    ) {
        for child in self.children_of(parent_id) {
            let has_children = !self.children_of(&child.id).is_empty();
            out.push((child, depth, has_children, parent_id));
            self.subtree_flat_rec(&child.id, depth + 1, out);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bipartite<RV = NoValue, DV = NoValue> {
    pub entities: Vec<Entity<RV>>,
    pub attributes: AttributeForest<DV>,
}

impl<RV, DV> Bipartite<RV, DV> {
    pub fn filter_entities<'a>(
        &'a self,
        selected_tags: &[(Id<Attribute>, Id<Attribute>)],
    ) -> Vec<&'a Entity<RV>> {
        self.entities
            .iter()
            .filter(|r| selected_tags.iter().all(|(k, v)| self.tag_matches(r, k, v)))
            .collect()
    }

    fn tag_matches(&self, r: &Entity<RV>, k: &Id<Attribute>, v: &Id<Attribute>) -> bool {
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
        selected: &[(Id<Attribute>, Id<Attribute>)],
    ) -> Vec<(Id<Attribute>, Id<Attribute>)> {
        let filtered = self.filter_entities(selected);
        let selected_set: HashSet<(&str, &str)> = selected
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let mut tags: HashSet<(Id<Attribute>, Id<Attribute>)> = HashSet::new();
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

        let mut tags_vec: Vec<(Id<Attribute>, Id<Attribute>)> = tags.into_iter().collect();
        tags_vec.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        tags_vec
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn test_forest() -> AttributeForest {
        // platform > cloud > gcp > bigquery / bigtable
        //                  > aws > s3
        AttributeForest(vec![
            Attribute {
                id: "platform".into(),
                label: "Platform".into(),
                parent: None,
                value: NoValue {},
            },
            Attribute {
                id: "cloud".into(),
                label: "Cloud".into(),
                parent: Some("platform".into()),
                value: NoValue {},
            },
            Attribute {
                id: "gcp".into(),
                label: "GCP".into(),
                parent: Some("cloud".into()),
                value: NoValue {},
            },
            Attribute {
                id: "bigquery".into(),
                label: "BigQuery".into(),
                parent: Some("gcp".into()),
                value: NoValue {},
            },
            Attribute {
                id: "bigtable".into(),
                label: "Bigtable".into(),
                parent: Some("gcp".into()),
                value: NoValue {},
            },
            Attribute {
                id: "aws".into(),
                label: "AWS".into(),
                parent: Some("cloud".into()),
                value: NoValue {},
            },
            Attribute {
                id: "s3".into(),
                label: "S3".into(),
                parent: Some("aws".into()),
                value: NoValue {},
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
            vec![Id::<Attribute>::from("cloud")]
        );
        assert_eq!(f.ancestry(&"unknown".into()), Vec::<Id<Attribute>>::new());
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

    #[test]
    fn filter_selects_by_ancestry() {
        use std::collections::HashMap;
        let f = test_forest();
        let bipartite = Bipartite {
            attributes: f,
            entities: vec![
                Entity {
                    id: "a".into(),
                    label: None,
                    attributes: HashMap::from([("platform".into(), "bigquery".into())]),
                    value: NoValue {},
                },
                Entity {
                    id: "b".into(),
                    label: None,
                    attributes: HashMap::from([("platform".into(), "s3".into())]),
                    value: NoValue {},
                },
                Entity {
                    id: "c".into(),
                    label: None,
                    attributes: HashMap::from([("platform".into(), "bigtable".into())]),
                    value: NoValue {},
                },
            ],
        };
        let got = bipartite.filter_entities(&[("platform".into(), "gcp".into())]);
        assert!(got.iter().any(|r| r.id.as_str() == "a"));
        assert!(got.iter().any(|r| r.id.as_str() == "c"));
        assert!(!got.iter().any(|r| r.id.as_str() == "b"));
    }
}
