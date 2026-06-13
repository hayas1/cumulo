use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// UI などが追加するビジュアル属性を持たない既定値。
/// `#[serde(flatten)]` で展開されるので JSON に余分なフィールドは追加されない。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct NoValue {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// キーは軸の根id。値はその軸内のノードid。
    pub dimensions: HashMap<String, String>,
    pub console_url: String,
    pub created_at: Option<String>,
    /// アクセス頻度（表示サイズに使用）
    pub freq: u32,
}

impl Default for Resource {
    fn default() -> Self {
        Resource {
            id: String::new(),
            label: None,
            dimensions: HashMap::new(),
            console_url: String::new(),
            created_at: None,
            freq: 1,
        }
    }
}

impl Resource {
    pub fn display_label<A: Clone>(&self, forest: &DimensionForest<A>) -> String {
        if let Some(l) = &self.label {
            if !l.is_empty() {
                return l.clone();
            }
        }
        let mut parts: Vec<String> = self
            .dimensions
            .values()
            .filter_map(|v| forest.node(v))
            .map(|n| {
                if n.label.is_empty() {
                    n.id.clone()
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

    pub fn dimension(&self, root_id: &str) -> Option<&str> {
        self.dimensions.get(root_id).map(String::as_str)
    }
}

/// parent が None のノードが軸の根（＝属性キー）となる。
/// リソースの value は { 根id → ノードid } で表現する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DimensionNode<A = NoValue> {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// `A = NoValue` のとき flatten は何も追加しない。
    /// web 層は `A = DimValue { color }` を指定して color を同じ JSON レベルに展開する。
    #[serde(flatten)]
    pub value: A,
}

/// parent リンクで森を構成する。parent が None のノードが軸の根となる。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(transparent)]
pub struct DimensionForest<A = NoValue>(pub Vec<DimensionNode<A>>);

impl<A> std::ops::Deref for DimensionForest<A> {
    type Target = Vec<DimensionNode<A>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A> std::ops::DerefMut for DimensionForest<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<A> DimensionForest<A> {
    pub fn roots(&self) -> Vec<&DimensionNode<A>> {
        self.iter().filter(|n| n.parent.is_none()).collect()
    }

    pub fn children_of(&self, parent_id: &str) -> Vec<&DimensionNode<A>> {
        self.iter()
            .filter(|n| n.parent.as_deref() == Some(parent_id))
            .collect()
    }

    /// 軸の根 (parent==None) の id は含めない（根はキーであり値ではない）。
    pub fn ancestry(&self, id: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur = Some(id.to_string());
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

    pub fn root_of(&self, id: &str) -> Option<String> {
        let mut cur = id.to_string();
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

    pub fn node(&self, id: &str) -> Option<&DimensionNode<A>> {
        self.iter().find(|n| n.id == id)
    }

    pub fn ancestry_contains(&self, start: &str, target: &str) -> bool {
        let mut cur = Some(start.to_string());
        let mut seen = HashSet::new();
        while let Some(c) = cur {
            if c == target {
                return true;
            }
            if !seen.insert(c.clone()) {
                break;
            }
            cur = self.iter().find(|n| n.id == c).and_then(|n| n.parent.clone());
        }
        false
    }

    pub fn collect_descendants(&self, root: &str) -> HashSet<String> {
        let mut out = HashSet::new();
        self.collect_descendants_rec(root, &mut out);
        out
    }

    fn collect_descendants_rec(&self, id: &str, out: &mut HashSet<String>) {
        if !out.insert(id.to_string()) {
            return;
        }
        for child in self.children_of(id) {
            self.collect_descendants_rec(&child.id, out);
        }
    }

    pub fn dfs_order(
        &self,
        root_id: &str,
        collapsed: &HashSet<String>,
    ) -> Vec<(String, usize, bool)> {
        let mut out = Vec::new();
        self.dfs_order_rec(root_id, 0, collapsed, &mut out);
        out
    }

    fn dfs_order_rec(
        &self,
        parent_id: &str,
        depth: usize,
        collapsed: &HashSet<String>,
        out: &mut Vec<(String, usize, bool)>,
    ) {
        for child in self.children_of(parent_id) {
            let has_children = !self.children_of(&child.id).is_empty();
            out.push((child.id.clone(), depth, has_children));
            if has_children && !collapsed.contains(&child.id) {
                self.dfs_order_rec(&child.id, depth + 1, collapsed, out);
            }
        }
    }

    /// 深さ優先で子孫を列挙し、counts > 0 のノードのみ (id, label, depth, count) を out に追加する。
    pub fn dfs_collect_counts(
        &self,
        parent_id: &str,
        depth: usize,
        counts: &HashMap<String, usize>,
        out: &mut Vec<(String, String, usize, usize)>,
    ) {
        for child in self.children_of(parent_id) {
            let cnt = counts.get(&child.id).copied().unwrap_or(0);
            if cnt == 0 {
                continue;
            }
            out.push((child.id.clone(), child.label.clone(), depth, cnt));
            self.dfs_collect_counts(&child.id, depth + 1, counts, out);
        }
    }

    pub fn reparent(&mut self, dragged: &str, new_parent: Option<String>) {
        if let Some(np) = &new_parent {
            if np == dragged || self.ancestry_contains(np, dragged) {
                return;
            }
        }
        if let Some(n) = self.iter_mut().find(|n| n.id == dragged) {
            n.parent = new_parent;
        }
    }

    pub fn move_relative(&mut self, dragged: &str, target: &str, after: bool) {
        if dragged == target {
            return;
        }
        let new_parent = self.iter().find(|n| n.id == target).and_then(|n| n.parent.clone());
        if let Some(np) = &new_parent {
            if self.ancestry_contains(np, dragged) {
                return;
            }
        }
        let Some(dpos) = self.iter().position(|n| n.id == dragged) else {
            return;
        };
        let mut node = self.remove(dpos);
        node.parent = new_parent;
        let tpos = self
            .iter()
            .position(|n| n.id == target)
            .unwrap_or(self.len());
        let insert_at = if after { tpos + 1 } else { tpos };
        let len = self.len();
        self.insert(insert_at.min(len), node);
    }

    pub fn delete_promote(&mut self, node_id: &str) {
        let parent = self
            .iter()
            .find(|n| n.id == node_id)
            .and_then(|n| n.parent.clone());
        for child in self.iter_mut() {
            if child.parent.as_deref() == Some(node_id) {
                child.parent = parent.clone();
            }
        }
        self.retain(|n| n.id != node_id);
    }

    pub fn delete_subtree(&mut self, node_id: &str) {
        let doomed = self.collect_descendants(node_id);
        self.retain(|n| !doomed.contains(&n.id));
    }

    pub fn rename_node(&mut self, old_id: &str, new_id: &str, label: &str, value: A) {
        if old_id != new_id {
            for other in self.iter_mut() {
                if other.parent.as_deref() == Some(old_id) {
                    other.parent = Some(new_id.to_string());
                }
            }
        }
        if let Some(n) = self.iter_mut().find(|n| n.id == old_id) {
            n.id = new_id.to_string();
            n.label = label.to_string();
            n.value = value;
        }
    }

    /// (node, depth, has_children, parent_id) のフラットリストを DFS 順で返す。
    /// resource_form の葉グルーピングなど呼び出し側が parent_id を使いたいケース向け。
    pub fn subtree_flat<'a>(
        &'a self,
        root_id: &'a str,
    ) -> Vec<(&'a DimensionNode<A>, usize, bool, &'a str)> {
        let mut out = Vec::new();
        self.subtree_flat_rec(root_id, 0, &mut out);
        out
    }

    fn subtree_flat_rec<'a>(
        &'a self,
        parent_id: &'a str,
        depth: usize,
        out: &mut Vec<(&'a DimensionNode<A>, usize, bool, &'a str)>,
    ) {
        for child in self.children_of(parent_id) {
            let has_children = !self.children_of(&child.id).is_empty();
            out.push((child, depth, has_children, parent_id));
            self.subtree_flat_rec(&child.id, depth + 1, out);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Bipartite<A = NoValue> {
    pub resources: Vec<Resource>,
    pub dimensions: DimensionForest<A>,
}

impl<A> Bipartite<A> {
    pub fn filter_resources<'a>(
        &'a self,
        selected_tags: &[(String, String)],
    ) -> Vec<&'a Resource> {
        self.resources
            .iter()
            .filter(|r| {
                selected_tags
                    .iter()
                    .all(|(k, v)| self.tag_matches(r, k, v))
            })
            .collect()
    }

    fn tag_matches(&self, r: &Resource, k: &str, v: &str) -> bool {
        let Some(rv) = r.dimensions.get(k) else {
            return false;
        };
        if rv == v {
            return true;
        }
        self.dimensions.ancestry(rv).iter().any(|a| a == v)
    }

    pub fn available_tags(&self, selected: &[(String, String)]) -> Vec<(String, String)> {
        let filtered = self.filter_resources(selected);
        let selected_set: HashSet<(&str, &str)> = selected
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let mut tags: HashSet<(String, String)> = HashSet::new();
        for r in &filtered {
            for (k, v) in &r.dimensions {
                if !selected_set.contains(&(k.as_str(), v.as_str())) {
                    tags.insert((k.clone(), v.clone()));
                }
                for anc in self.dimensions.ancestry(v) {
                    if !selected_set.contains(&(k.as_str(), anc.as_str())) {
                        tags.insert((k.clone(), anc));
                    }
                }
            }
        }

        let mut tags_vec: Vec<(String, String)> = tags.into_iter().collect();
        tags_vec.sort();
        tags_vec
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn test_forest() -> DimensionForest {
        // platform > cloud > gcp > bigquery / bigtable
        //                  > aws > s3
        DimensionForest(vec![
            DimensionNode { id: "platform".into(), label: "Platform".into(), parent: None, value: NoValue {} },
            DimensionNode { id: "cloud".into(), label: "Cloud".into(), parent: Some("platform".into()), value: NoValue {} },
            DimensionNode { id: "gcp".into(), label: "GCP".into(), parent: Some("cloud".into()), value: NoValue {} },
            DimensionNode { id: "bigquery".into(), label: "BigQuery".into(), parent: Some("gcp".into()), value: NoValue {} },
            DimensionNode { id: "bigtable".into(), label: "Bigtable".into(), parent: Some("gcp".into()), value: NoValue {} },
            DimensionNode { id: "aws".into(), label: "AWS".into(), parent: Some("cloud".into()), value: NoValue {} },
            DimensionNode { id: "s3".into(), label: "S3".into(), parent: Some("aws".into()), value: NoValue {} },
        ])
    }

    #[test]
    fn ancestry_walks_to_root_exclusive() {
        let f = test_forest();
        assert_eq!(f.ancestry("bigquery"), vec!["bigquery", "gcp", "cloud"]);
        assert_eq!(f.ancestry("cloud"), vec!["cloud"]);
        assert_eq!(f.ancestry("unknown"), Vec::<String>::new());
    }

    #[test]
    fn ancestry_contains_detects_ancestor() {
        let f = test_forest();
        assert!(f.ancestry_contains("bigquery", "gcp"));
        assert!(f.ancestry_contains("bigquery", "cloud"));
        assert!(!f.ancestry_contains("bigquery", "s3"));
        assert!(!f.ancestry_contains("bigquery", "bigtable"));
    }

    #[test]
    fn collect_descendants_includes_self_and_all_children() {
        let f = test_forest();
        let desc = f.collect_descendants("gcp");
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
            dimensions: f,
            resources: vec![
                Resource { id: "a".into(), label: None, dimensions: HashMap::from([("platform".into(), "bigquery".into())]), console_url: String::new(), created_at: None, freq: 1 },
                Resource { id: "b".into(), label: None, dimensions: HashMap::from([("platform".into(), "s3".into())]), console_url: String::new(), created_at: None, freq: 1 },
                Resource { id: "c".into(), label: None, dimensions: HashMap::from([("platform".into(), "bigtable".into())]), console_url: String::new(), created_at: None, freq: 1 },
            ],
        };
        let got = bipartite.filter_resources(&[("platform".into(), "gcp".into())]);
        let ids: Vec<&str> = got.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"c"));
        assert!(!ids.contains(&"b"));
    }
}
