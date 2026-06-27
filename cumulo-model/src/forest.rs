use std::collections::HashSet;

use crate::error::{Errors, ForestError};
use crate::id::Id;

/// is-a 森のノード。自身の id と parent（None なら根）を公開する。
/// `Id<Self>` を使うことで、ノード種ごとに異なる ID 型を型レベルで保てる。
pub trait ForestNode: Sized {
    fn id(&self) -> &Id<Self>;
    fn parent(&self) -> Option<&Id<Self>>;
    /// parent リンクの付け替え。繰り上げ削除などの編集系で使う。
    fn set_parent(&mut self, parent: Option<Id<Self>>);
}

/// parent リンクで is-a 森を構成するノード列に共通する読み取りナビゲーション。
/// Catalog / Taxonomy が実装し、これらの走査ロジックを一箇所に集約する。
/// 並べ替えを伴う reparent などは森ごとに事情が異なるため各型側に残し、
/// 森の種類に依らず同一な削除系は [`ForestMut`] に分離する。
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

    /// id から根まで（根を含む）の is-a チェーンを返す。存在しない id は空。
    fn ancestry(&self, id: &Id<Self::Node>) -> Vec<Id<Self::Node>> {
        let mut chain = Vec::new();
        let mut cur = Some(id.clone());
        while let Some(c) = cur {
            if chain.contains(&c) {
                break;
            }
            let Some(node) = self.node(&c) else { break };
            let parent = node.parent().cloned();
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

    /// id を含む is-a チェーンを根（parent==None）まで辿り、その根 id を返す。
    /// id 自身が根なら id 自身を返す。
    fn root_of(&self, id: &Id<Self::Node>) -> Option<Id<Self::Node>> {
        let mut cur = id.clone();
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(cur.clone()) {
                return None;
            }
            match self.node(&cur)?.parent() {
                None => return Some(cur),
                Some(p) => cur = p.clone(),
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

    /// 森の構造的整合性を全件検証し、見つかったエラーを集約して返す。
    /// fail-fast せず全件収集するのは、インポート時に一度で全問題を把握させるため。
    /// attribute ペイロードの検証はモデル層が型を知らないため対象外。
    fn validate(&self) -> Result<&Self, Errors<ForestError>> {
        let nodes = self.nodes();
        let mut errors = Vec::new();

        // A2: id 単体の妥当性は Id::validate に委譲する
        for n in nodes {
            if let Err(error) = n.id().validate() {
                errors.push(ForestError::InvalidId {
                    id: n.id().as_str().to_string(),
                    error,
                });
            }
        }

        // A1: id 一意性
        let mut seen_ids = HashSet::new();
        for n in nodes {
            let s = n.id().as_str().to_string();
            if !seen_ids.insert(s.clone()) {
                errors.push(ForestError::DuplicateId { id: s });
            }
        }

        // A3: parent 存在性（dangling parent）
        let all_ids: HashSet<&str> = nodes.iter().map(|n| n.id().as_str()).collect();
        for n in nodes {
            if let Some(p) = n.parent() {
                if !all_ids.contains(p.as_str()) {
                    errors.push(ForestError::DanglingParent {
                        id: n.id().as_str().to_string(),
                        parent: p.as_str().to_string(),
                    });
                }
            }
        }

        // A4: 非循環（各ノードから parent を辿り自己祖先を検出）
        for n in nodes {
            let start = n.id();
            let mut cur = n.parent().cloned();
            let mut visited = HashSet::new();
            visited.insert(start.as_str().to_string());
            while let Some(p) = cur {
                if !visited.insert(p.as_str().to_string()) {
                    errors.push(ForestError::Cycle {
                        id: start.as_str().to_string(),
                    });
                    break;
                }
                cur = self.node(&p).and_then(|pn| pn.parent().cloned());
            }
        }

        if errors.is_empty() {
            Ok(self)
        } else {
            Err(Errors(errors))
        }
    }

    /// 検証を通れば所有権ごと返す。構築境界で `Self(nodes).validated()?` のように書ける。
    fn validated(self) -> Result<Self, Errors<ForestError>>
    where
        Self: Sized,
    {
        self.validate()?;
        Ok(self)
    }
}

/// 森の種類（Catalog / Taxonomy）に依らず同一な削除系を集約する。
/// parent リンクだけ見ればよく node 種に依存しないため、各型は `nodes_mut` のみ実装する。
pub trait ForestMut: Forest {
    fn nodes_mut(&mut self) -> &mut Vec<Self::Node>;

    /// node を削除し、その子は node の親へ繰り上げる（子の孤児化を防ぐ）。
    fn delete_promote(&mut self, node_id: &Id<Self::Node>) {
        let parent = self.node(node_id).and_then(|n| n.parent().cloned());
        for child in self.nodes_mut().iter_mut() {
            if child.parent() == Some(node_id) {
                child.set_parent(parent.clone());
            }
        }
        self.nodes_mut().retain(|n| n.id() != node_id);
    }

    /// node とその子孫をまとめて削除する。
    fn delete_subtree(&mut self, node_id: &Id<Self::Node>) {
        let doomed = self.collect_descendants(node_id);
        self.nodes_mut().retain(|n| !doomed.contains(n.id()));
    }
}

#[cfg(test)]
mod tests {
    use crate::category::{Category, Taxonomy};
    use crate::error::{ForestError, IdError};
    use crate::id::Id;

    use super::Forest;

    fn cat(id: &str, parent: Option<&str>) -> Category<()> {
        Category {
            // 空 id のテストケース（A2）のため TryFrom ではなく new_unchecked を使う
            id: Id::new_unchecked(id),
            label: id.into(),
            parent: parent.map(Id::new_unchecked),
            attribute: (),
        }
    }

    // A1: 正常系 — id が重複しなければエラーなし
    #[test]
    fn valid_forest_has_no_errors() {
        let t = Taxonomy(vec![cat("root", None), cat("child", Some("root"))]);
        assert!(t.validate().is_ok());
    }

    // A1: id 重複を検出する
    #[test]
    fn duplicate_id_is_detected() {
        let t = Taxonomy(vec![
            cat("root", None),
            cat("root", None), // 重複
        ]);
        let errs = t.validate().unwrap_err();
        assert!(errs.contains(&ForestError::DuplicateId { id: "root".into() }));
    }

    // A2: 空 id を InvalidId として検出する（ルールは Id::validate が持つ）
    #[test]
    fn empty_id_is_detected() {
        let t = Taxonomy(vec![cat("", None)]);
        let errs = t.validate().unwrap_err();
        assert!(errs.contains(&ForestError::InvalidId {
            id: "".into(),
            error: IdError::Empty,
        }));
    }

    // A3: 存在しない parent を dangling として検出する
    #[test]
    fn dangling_parent_is_detected() {
        let t = Taxonomy(vec![cat("child", Some("ghost"))]);
        let errs = t.validate().unwrap_err();
        assert!(errs.contains(&ForestError::DanglingParent {
            id: "child".into(),
            parent: "ghost".into(),
        }));
    }

    // A4: 自己参照（parent が自分自身）でサイクルを検出する
    #[test]
    fn self_loop_is_detected_as_cycle() {
        let t = Taxonomy(vec![cat("a", Some("a"))]);
        let errs = t.validate().unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, ForestError::Cycle { id } if id == "a")));
    }

    // A4: 2 ノード間の循環を検出する
    #[test]
    fn two_node_cycle_is_detected() {
        let t = Taxonomy(vec![cat("a", Some("b")), cat("b", Some("a"))]);
        let errs = t.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, ForestError::Cycle { .. })));
    }

    // 空の森は正常（エラーなし）
    #[test]
    fn empty_forest_is_valid() {
        let t: Taxonomy<()> = Taxonomy(vec![]);
        assert!(t.validate().is_ok());
    }

    // validated は通れば所有権ごと値を返し、不正なら Err
    #[test]
    fn validated_returns_owned_self_when_valid() {
        let t = Taxonomy(vec![cat("root", None)]);
        let got = t.validated().expect("valid forest");
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn validated_returns_err_when_invalid() {
        let t = Taxonomy(vec![cat("a", Some("a"))]); // self-loop
        assert!(t.validated().is_err());
    }

    // 複数エラーが同時に収集される（fail-fast しない）
    #[test]
    fn multiple_errors_are_collected() {
        let t = Taxonomy(vec![
            cat("", None),           // A2: empty id
            cat("dup", None),        // A1: duplicate (下と)
            cat("dup", None),        // A1: duplicate
            cat("x", Some("ghost")), // A3: dangling
        ]);
        let errs = t.validate().unwrap_err();
        assert!(errs.contains(&ForestError::InvalidId {
            id: "".into(),
            error: IdError::Empty,
        }));
        assert!(errs.contains(&ForestError::DuplicateId { id: "dup".into() }));
        assert!(errs.contains(&ForestError::DanglingParent {
            id: "x".into(),
            parent: "ghost".into(),
        }));
    }
}
