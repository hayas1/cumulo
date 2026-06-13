use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::id::Id;
use crate::taxonomy::{Category, Taxonomy};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource<V, A> {
    pub id: Id<Resource<V, A>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// キーは軸の根id。値はその軸内のノードid。
    pub categories: HashMap<Id<Category<A>>, Id<Category<A>>>,
    #[serde(flatten)]
    pub value: V,
}

impl<V: Default, A> Default for Resource<V, A> {
    fn default() -> Self {
        Resource {
            id: Id::<Resource<V, A>>::default(),
            label: None,
            categories: HashMap::new(),
            value: V::default(),
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
