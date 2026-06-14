use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::category::{Category, Taxonomy};
use crate::forest::{Forest, ForestNode};
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

impl<RA, CA> ForestNode for Resource<RA, CA> {
    fn id(&self) -> &Id<Self> {
        &self.id
    }
    fn parent(&self) -> Option<&Id<Self>> {
        self.parent.as_ref()
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

impl<RA, CA> Forest for Catalog<RA, CA> {
    type Node = Resource<RA, CA>;
    fn nodes(&self) -> &[Resource<RA, CA>] {
        &self.0
    }
}

impl<RA, CA> Catalog<RA, CA> {
    /// 森の構造整合性を検証してから構築する。検証を通った場合のみ Ok を返す。
    pub fn try_new(nodes: Vec<Resource<RA, CA>>) -> Result<Self, crate::error::Errors<crate::error::ForestError>> {
        Catalog(nodes).validated()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Id<Resource<(), ()>> {
        s.try_into().unwrap()
    }

    fn test_catalog() -> Catalog<(), ()> {
        // gcp > bigquery
        //     > bigtable
        Catalog(vec![
            Resource {
                id: id("gcp"),
                label: None,
                parent: None,
                categories: HashMap::new(),
                attribute: (),
            },
            Resource {
                id: id("bigquery"),
                label: None,
                parent: Some(id("gcp")),
                categories: HashMap::new(),
                attribute: (),
            },
            Resource {
                id: id("bigtable"),
                label: None,
                parent: Some(id("gcp")),
                categories: HashMap::new(),
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
    fn ancestry_walks_to_root_exclusive() {
        let c = test_catalog();
        assert_eq!(c.ancestry(&id("bigquery")), vec![id("bigquery")]);
        assert_eq!(
            c.ancestry(&id("gcp")),
            Vec::<Id<Resource<(), ()>>>::new()
        );
    }

    // --- Catalog::try_new のテスト ---

    #[test]
    fn try_new_returns_ok_for_valid_nodes() {
        let nodes = vec![
            Resource { id: id("root"), label: None, parent: None, categories: HashMap::new(), attribute: () },
            Resource { id: id("child"), label: None, parent: Some(id("root")), categories: HashMap::new(), attribute: () },
        ];
        assert!(Catalog::try_new(nodes).is_ok());
    }

    #[test]
    fn try_new_returns_err_for_duplicate_ids() {
        use crate::error::ForestError;
        let nodes = vec![
            Resource { id: id("dup"), label: None, parent: None, categories: HashMap::new(), attribute: () },
            Resource { id: id("dup"), label: None, parent: None, categories: HashMap::new(), attribute: () },
        ];
        let err = Catalog::<(), ()>::try_new(nodes).unwrap_err();
        assert!(err.iter().any(|e| matches!(e, ForestError::DuplicateId { id } if id == "dup")));
    }

    #[test]
    fn try_new_returns_err_for_dangling_parent() {
        use crate::error::ForestError;
        let nodes = vec![
            Resource { id: id("r1"), label: None, parent: Some(id("ghost")), categories: HashMap::new(), attribute: () },
        ];
        let err = Catalog::<(), ()>::try_new(nodes).unwrap_err();
        assert!(err.iter().any(|e| matches!(e, ForestError::DanglingParent { id, parent }
            if id == "r1" && parent == "ghost")));
    }

    #[test]
    fn try_new_empty_is_ok() {
        assert!(Catalog::<(), ()>::try_new(vec![]).is_ok());
    }
}
