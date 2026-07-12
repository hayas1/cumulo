use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::category::Category;
use crate::id::Id;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Filters<CA>(IndexMap<Id<Category<CA>>, Id<Category<CA>>>);

impl<CA> Clone for Filters<CA> {
    fn clone(&self) -> Self {
        Filters(self.0.clone())
    }
}

impl<CA> Default for Filters<CA> {
    fn default() -> Self {
        Filters(IndexMap::new())
    }
}

impl<CA> std::fmt::Debug for Filters<CA> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<CA> PartialEq for Filters<CA> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<CA> FromIterator<(Id<Category<CA>>, Id<Category<CA>>)> for Filters<CA> {
    fn from_iter<I: IntoIterator<Item = (Id<Category<CA>>, Id<Category<CA>>)>>(iter: I) -> Self {
        Filters(iter.into_iter().collect())
    }
}

impl<CA> Filters<CA> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, root: &Id<Category<CA>>) -> Option<&Id<Category<CA>>> {
        self.0.get(root)
    }

    pub fn set(&mut self, root: Id<Category<CA>>, value: Id<Category<CA>>) {
        self.0.insert(root, value);
    }

    pub fn toggle(&mut self, root: Id<Category<CA>>, value: Id<Category<CA>>) {
        if self.0.get(&root) == Some(&value) {
            self.0.shift_remove(&root);
        } else {
            self.0.insert(root, value);
        }
    }

    pub fn remove_root(&mut self, root: &Id<Category<CA>>) {
        self.0.shift_remove(root);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn without_root(&self, root: &Id<Category<CA>>) -> Self {
        let mut copy = self.clone();
        copy.remove_root(root);
        copy
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id<Category<CA>>, &Id<Category<CA>>)> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(s: &str) -> Id<Category<()>> {
        s.try_into().unwrap()
    }

    #[test]
    fn serde_round_trips_as_map_preserving_order() {
        let f: Filters<()> = [(cid("platform"), cid("gcp")), (cid("env"), cid("prod"))]
            .into_iter()
            .collect();
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(json, r#"{"platform":"gcp","env":"prod"}"#);
        assert_eq!(serde_json::from_str::<Filters<()>>(&json).unwrap(), f);
    }

    #[test]
    fn set_and_get() {
        let mut f = Filters::new();
        f.set(cid("platform"), cid("gcp"));
        assert_eq!(f.get(&cid("platform")), Some(&cid("gcp")));
        assert_eq!(f.get(&cid("env")), None);
    }

    #[test]
    fn set_replaces_value_on_same_axis() {
        let mut f = Filters::new();
        f.set(cid("platform"), cid("gcp"));
        f.set(cid("platform"), cid("aws"));
        assert_eq!(f.get(&cid("platform")), Some(&cid("aws")));
        assert_eq!(f.iter().count(), 1);
    }

    #[test]
    fn toggle_clears_when_same_sets_when_different() {
        let mut f = Filters::new();
        f.toggle(cid("platform"), cid("gcp"));
        assert_eq!(f.get(&cid("platform")), Some(&cid("gcp")));
        f.toggle(cid("platform"), cid("gcp"));
        assert_eq!(f.get(&cid("platform")), None);
        f.toggle(cid("platform"), cid("gcp"));
        f.toggle(cid("platform"), cid("aws"));
        assert_eq!(f.get(&cid("platform")), Some(&cid("aws")));
    }

    #[test]
    fn iter_preserves_insertion_order() {
        let mut f = Filters::new();
        f.set(cid("platform"), cid("gcp"));
        f.set(cid("env"), cid("prod"));
        f.set(cid("platform"), cid("aws"));
        let order: Vec<_> = f.iter().map(|(k, _)| k.as_str().to_string()).collect();
        assert_eq!(order, vec!["platform", "env"]);
    }

    #[test]
    fn remove_root_and_clear() {
        let mut f = Filters::new();
        f.set(cid("platform"), cid("gcp"));
        f.set(cid("env"), cid("prod"));
        f.remove_root(&cid("platform"));
        assert_eq!(f.get(&cid("platform")), None);
        assert_eq!(f.iter().count(), 1);
        f.clear();
        assert!(f.is_empty());
    }

    #[test]
    fn without_root_returns_copy_excluding_root() {
        let f: Filters<()> = [(cid("platform"), cid("gcp")), (cid("env"), cid("prod"))]
            .into_iter()
            .collect();
        let g = f.without_root(&cid("platform"));
        assert_eq!(g.get(&cid("platform")), None);
        assert_eq!(g.get(&cid("env")), Some(&cid("prod")));
        assert_eq!(f.get(&cid("platform")), Some(&cid("gcp")));
    }
}
