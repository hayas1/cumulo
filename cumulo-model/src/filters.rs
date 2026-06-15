use indexmap::IndexMap;

use crate::category::Category;
use crate::id::Id;

/// 値が属する木の根 → 選択値 の対応。カテゴリ木に対する絞り込み選択を表す。
/// 根（faceting でいう軸）1つにつき値は1つに保ち、挿入順を維持する（UI のピル表示順が安定する）。
pub struct Filters<CA>(IndexMap<Id<Category<CA>>, Id<Category<CA>>>);

// Id<Category<CA>> は CA に依らず Clone/Eq/Hash なので、CA に余計な境界を課さないよう手実装する。
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

    /// 指定した根の選択値。
    pub fn get(&self, root: &Id<Category<CA>>) -> Option<&Id<Category<CA>>> {
        self.0.get(root)
    }

    /// その根の値を設定する（既存値は置換、挿入位置は保持）。
    pub fn set(&mut self, root: Id<Category<CA>>, value: Id<Category<CA>>) {
        self.0.insert(root, value);
    }

    /// 同じ (root, value) があれば解除、なければ設定（根1つにつき値1つで置換）。
    pub fn toggle(&mut self, root: Id<Category<CA>>, value: Id<Category<CA>>) {
        if self.0.get(&root) == Some(&value) {
            self.0.shift_remove(&root);
        } else {
            self.0.insert(root, value);
        }
    }

    /// 指定した根のフィルタを外す。
    pub fn remove_root(&mut self, root: &Id<Category<CA>>) {
        self.0.shift_remove(root);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// 指定した根を除いたコピーを返す。その根の候補件数を「他の根で絞った母集団」で数えるのに使う。
    pub fn without_root(&self, root: &Id<Category<CA>>) -> Self {
        let mut copy = self.clone();
        copy.remove_root(root);
        copy
    }

    /// (根, 値) を挿入順に走査する。
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
    fn set_and_get() {
        let mut f = Filters::new();
        f.set(cid("platform"), cid("gcp"));
        assert_eq!(f.get(&cid("platform")), Some(&cid("gcp")));
        assert_eq!(f.get(&cid("env")), None);
    }

    // 1軸1値: 同じ軸に set すると値が置換される
    #[test]
    fn set_replaces_value_on_same_axis() {
        let mut f = Filters::new();
        f.set(cid("platform"), cid("gcp"));
        f.set(cid("platform"), cid("aws"));
        assert_eq!(f.get(&cid("platform")), Some(&cid("aws")));
        assert_eq!(f.iter().count(), 1);
    }

    // toggle: 同値なら解除、別値なら置換
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

    // iter は挿入順を保つ（置換しても位置は変わらない）
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
        // 元は変わらない
        assert_eq!(f.get(&cid("platform")), Some(&cid("gcp")));
    }
}
