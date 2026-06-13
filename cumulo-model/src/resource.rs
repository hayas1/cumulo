use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::category::{Category, Taxonomy};
use crate::id::Id;

/// `#[serde(bound)]` で境界を明示し、flatten が attribute: V から生成する V: Default 境界を除去する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound(
    serialize = "V: Serialize, A: Serialize",
    deserialize = "V: Deserialize<'de>, A: Deserialize<'de>"
))]
pub struct Resource<V, A> {
    pub id: Id<Resource<V, A>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// parent が None のリソースが catalog の is-a 森の根となる（Taxonomy と対称）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Resource<V, A>>>,
    /// キーは軸の根id。値はその軸内のノードid。
    pub categories: HashMap<Id<Category<A>>, Id<Category<A>>>,
    #[serde(flatten)]
    pub attribute: V,
}

impl<V: Default, A> Default for Resource<V, A> {
    fn default() -> Self {
        Resource {
            id: Id::<Resource<V, A>>::default(),
            label: None,
            parent: None,
            categories: HashMap::new(),
            attribute: V::default(),
        }
    }
}

impl<V, A: Clone> Resource<V, A> {
    pub fn display_label(&self, forest: &Taxonomy<A>) -> String {
        if let Some(l) = &self.label {
            if !l.is_empty() {
                return l.clone();
            }
        }
        let mut parts: Vec<String> = self
            .categories
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

    pub fn category(&self, root_id: &Id<Category<A>>) -> Option<&Id<Category<A>>> {
        self.categories.get(root_id)
    }
}

/// parent リンクで森を構成する resource の is-a 森（Taxonomy と対称）。
/// parent が None のリソースが根となる。今はガワで、編集系は今後 Taxonomy から移植する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Catalog<V, A>(pub Vec<Resource<V, A>>);

impl<V, A> Default for Catalog<V, A> {
    fn default() -> Self {
        Catalog(Vec::new())
    }
}

impl<V, A> std::ops::Deref for Catalog<V, A> {
    type Target = Vec<Resource<V, A>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V, A> std::ops::DerefMut for Catalog<V, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<V, A> Catalog<V, A> {
    pub fn roots(&self) -> Vec<&Resource<V, A>> {
        self.iter().filter(|n| n.parent.is_none()).collect()
    }

    pub fn children_of(&self, parent_id: &Id<Resource<V, A>>) -> Vec<&Resource<V, A>> {
        self.iter()
            .filter(|n| n.parent.as_ref() == Some(parent_id))
            .collect()
    }

    pub fn node(&self, id: &Id<Resource<V, A>>) -> Option<&Resource<V, A>> {
        self.iter().find(|n| &n.id == id)
    }

    /// 根 (parent==None) の id は含めない（Taxonomy::ancestry と同じ規約）。
    pub fn ancestry(&self, id: &Id<Resource<V, A>>) -> Vec<Id<Resource<V, A>>> {
        let mut chain = Vec::new();
        let mut cur = Some(id.clone());
        while let Some(c) = cur {
            if chain.contains(&c) {
                break;
            }
            let parent = self
                .iter()
                .find(|n| n.id == c)
                .and_then(|n| n.parent.clone());
            if parent.is_none() {
                break;
            }
            chain.push(c);
            cur = parent;
        }
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_catalog() -> Catalog<(), ()> {
        // gcp > bigquery
        //     > bigtable
        Catalog(vec![
            Resource {
                id: "gcp".into(),
                parent: None,
                ..Default::default()
            },
            Resource {
                id: "bigquery".into(),
                parent: Some("gcp".into()),
                ..Default::default()
            },
            Resource {
                id: "bigtable".into(),
                parent: Some("gcp".into()),
                ..Default::default()
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
            .children_of(&"gcp".into())
            .iter()
            .map(|r| r.id.as_str().to_string())
            .collect();
        kids.sort();
        assert_eq!(kids, vec!["bigquery", "bigtable"]);
    }

    #[test]
    fn ancestry_walks_to_root_exclusive() {
        let c = test_catalog();
        assert_eq!(c.ancestry(&"bigquery".into()), vec!["bigquery".into()]);
        assert_eq!(
            c.ancestry(&"gcp".into()),
            Vec::<Id<Resource<(), ()>>>::new()
        );
    }
}
