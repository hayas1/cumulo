use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::attribute::{Attribute, AttributeForest};
use crate::id::Id;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Entity<V, A> {
    pub id: Id<Entity<V, A>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// キーは軸の根id。値はその軸内のノードid。
    pub attributes: HashMap<Id<Attribute<A>>, Id<Attribute<A>>>,
    #[serde(flatten)]
    pub value: V,
}

impl<V: Default, A> Default for Entity<V, A> {
    fn default() -> Self {
        Entity {
            id: Id::<Entity<V, A>>::default(),
            label: None,
            attributes: HashMap::new(),
            value: V::default(),
        }
    }
}

impl<V, A: Clone> Entity<V, A> {
    pub fn display_label(&self, forest: &AttributeForest<A>) -> String {
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

    pub fn attribute(&self, root_id: &Id<Attribute<A>>) -> Option<&Id<Attribute<A>>> {
        self.attributes.get(root_id)
    }
}
