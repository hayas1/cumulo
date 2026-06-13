use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::category::{Category, Taxonomy};
use crate::id::Id;

/// `#[serde(bound)]` で境界を明示し、flatten が attribute: RA から生成する RA: Default 境界を除去する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound(
    serialize = "RA: Serialize, CA: Serialize",
    deserialize = "RA: Deserialize<'de>, CA: Deserialize<'de>"
))]
pub struct Resource<RA, CA> {
    pub id: Id<Resource<RA, CA>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// parent が None のリソースが catalog の is-a 森の根となる（Taxonomy と対称）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Resource<RA, CA>>>,
    /// キーは軸の根id。値はその軸内のノードid。
    pub categories: HashMap<Id<Category<CA>>, Id<Category<CA>>>,
    #[serde(flatten)]
    pub attribute: RA,
}

impl<RA: Default, CA> Default for Resource<RA, CA> {
    fn default() -> Self {
        Resource {
            id: Id::<Resource<RA, CA>>::default(),
            label: None,
            parent: None,
            categories: HashMap::new(),
            attribute: RA::default(),
        }
    }
}

impl<RA, CA: Clone> Resource<RA, CA> {
    pub fn display_label(&self, forest: &Taxonomy<CA>) -> String {
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

    pub fn category(&self, root_id: &Id<Category<CA>>) -> Option<&Id<Category<CA>>> {
        self.categories.get(root_id)
    }
}

/// parent リンクで森を構成する resource の is-a 森（Taxonomy と対称）。
/// parent が None のリソースが根となる。今はガワで、編集系は今後 Taxonomy から移植する。
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

impl<RA, CA> Catalog<RA, CA> {
    pub fn roots(&self) -> Vec<&Resource<RA, CA>> {
        self.iter().filter(|n| n.parent.is_none()).collect()
    }

    pub fn children_of(&self, parent_id: &Id<Resource<RA, CA>>) -> Vec<&Resource<RA, CA>> {
        self.iter()
            .filter(|n| n.parent.as_ref() == Some(parent_id))
            .collect()
    }

    pub fn node(&self, id: &Id<Resource<RA, CA>>) -> Option<&Resource<RA, CA>> {
        self.iter().find(|n| &n.id == id)
    }

    /// 根 (parent==None) の id は含めない（Taxonomy::ancestry と同じ規約）。
    pub fn ancestry(&self, id: &Id<Resource<RA, CA>>) -> Vec<Id<Resource<RA, CA>>> {
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
