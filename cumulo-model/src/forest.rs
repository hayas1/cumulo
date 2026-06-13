use std::collections::HashSet;

use crate::id::Id;

/// is-a 森のノード。自身の id と parent（None なら根）を公開する。
/// `Id<Self>` を使うことで、ノード種ごとに異なる ID 型を型レベルで保てる。
pub trait ForestNode: Sized {
    fn id(&self) -> &Id<Self>;
    fn parent(&self) -> Option<&Id<Self>>;
}

/// parent リンクで is-a 森を構成するノード列に共通する読み取りナビゲーション。
/// Catalog / Taxonomy が実装し、これらの走査ロジックを一箇所に集約する。
/// 編集系（reparent / delete など）は森ごとに事情が異なるため各型側に置く。
pub trait Forest {
    type Node: ForestNode;

    fn nodes(&self) -> &[Self::Node];

    fn roots(&self) -> Vec<&Self::Node> {
        self.nodes()
            .iter()
            .filter(|n| n.parent().is_none())
            .collect()
    }

    fn children_of(&self, parent_id: &Id<Self::Node>) -> Vec<&Self::Node> {
        self.nodes()
            .iter()
            .filter(|n| n.parent() == Some(parent_id))
            .collect()
    }

    fn node(&self, id: &Id<Self::Node>) -> Option<&Self::Node> {
        self.nodes().iter().find(|n| n.id() == id)
    }

    /// 根 (parent==None) の id は含めない（根はキーであり値ではない）。
    fn ancestry(&self, id: &Id<Self::Node>) -> Vec<Id<Self::Node>> {
        let mut chain = Vec::new();
        let mut cur = Some(id.clone());
        while let Some(c) = cur {
            if chain.contains(&c) {
                break;
            }
            let parent = self.node(&c).and_then(|n| n.parent().cloned());
            if parent.is_none() {
                break;
            }
            chain.push(c);
            cur = parent;
        }
        chain
    }

    fn ancestry_contains(&self, start: &Id<Self::Node>, target: &Id<Self::Node>) -> bool {
        let mut cur = Some(start.clone());
        let mut seen = HashSet::new();
        while let Some(c) = cur {
            if &c == target {
                return true;
            }
            if !seen.insert(c.clone()) {
                break;
            }
            cur = self.node(&c).and_then(|n| n.parent().cloned());
        }
        false
    }

    /// id の祖先のうち、軸の根（parent==None のノード）の直下にある根 id を返す。
    /// id 自身が根なら None。
    fn root_of(&self, id: &Id<Self::Node>) -> Option<Id<Self::Node>> {
        let mut cur = id.clone();
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(cur.clone()) {
                return None;
            }
            match self.node(&cur)?.parent() {
                None => return None,
                Some(p) => {
                    if self.node(p).is_some_and(|n| n.parent().is_none()) {
                        return Some(p.clone());
                    }
                    cur = p.clone();
                }
            }
        }
    }

    fn collect_descendants(&self, root: &Id<Self::Node>) -> HashSet<Id<Self::Node>> {
        let mut out = HashSet::new();
        let mut stack = vec![root.clone()];
        while let Some(id) = stack.pop() {
            if !out.insert(id.clone()) {
                continue;
            }
            for child in self.children_of(&id) {
                stack.push(child.id().clone());
            }
        }
        out
    }
}
