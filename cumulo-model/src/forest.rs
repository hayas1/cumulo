use std::collections::HashSet;

use crate::error::{Errors, ForestError};
use crate::id::Id;

pub trait ForestNode: Sized {
    fn id(&self) -> &Id<Self>;
    fn parent(&self) -> Option<&Id<Self>>;
    fn set_parent(&mut self, parent: Option<Id<Self>>);
}

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

    fn validate(&self) -> Result<&Self, Errors<ForestError>> {
        let nodes = self.nodes();
        let mut errors = Vec::new();

        for n in nodes {
            if let Err(error) = n.id().validate() {
                errors.push(ForestError::InvalidId {
                    id: n.id().as_str().to_string(),
                    error,
                });
            }
        }

        let mut seen_ids = HashSet::new();
        for n in nodes {
            let s = n.id().as_str().to_string();
            if !seen_ids.insert(s.clone()) {
                errors.push(ForestError::DuplicateId { id: s });
            }
        }

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

    fn validated(self) -> Result<Self, Errors<ForestError>>
    where
        Self: Sized,
    {
        self.validate()?;
        Ok(self)
    }
}

pub trait ForestMut: Forest {
    fn nodes_mut(&mut self) -> &mut Vec<Self::Node>;

    fn delete_promote(&mut self, node_id: &Id<Self::Node>) {
        let parent = self.node(node_id).and_then(|n| n.parent().cloned());
        for child in self.nodes_mut().iter_mut() {
            if child.parent() == Some(node_id) {
                child.set_parent(parent.clone());
            }
        }
        self.nodes_mut().retain(|n| n.id() != node_id);
    }

    fn delete_subtree(&mut self, node_id: &Id<Self::Node>) {
        let doomed = self.collect_descendants(node_id);
        self.nodes_mut().retain(|n| !doomed.contains(n.id()));
    }

    fn reparent(
        &mut self,
        dragged: &Id<Self::Node>,
        new_parent: Option<Id<Self::Node>>,
    ) -> Result<(), ForestError> {
        if let Some(np) = &new_parent {
            if np == dragged || self.ancestry_contains(np, dragged) {
                return Err(ForestError::Cycle {
                    id: dragged.as_str().to_string(),
                });
            }
        }
        if let Some(n) = self.nodes_mut().iter_mut().find(|n| n.id() == dragged) {
            n.set_parent(new_parent);
        }
        Ok(())
    }

    fn move_relative(
        &mut self,
        dragged: &Id<Self::Node>,
        target: &Id<Self::Node>,
        after: bool,
    ) -> Result<(), ForestError> {
        if dragged == target {
            return Ok(());
        }
        let new_parent = self.node(target).and_then(|n| n.parent().cloned());
        if let Some(np) = &new_parent {
            if self.ancestry_contains(np, dragged) {
                return Err(ForestError::Cycle {
                    id: dragged.as_str().to_string(),
                });
            }
        }
        let nodes = self.nodes_mut();
        let Some(dpos) = nodes.iter().position(|n| n.id() == dragged) else {
            return Ok(());
        };
        let mut node = nodes.remove(dpos);
        node.set_parent(new_parent);
        let tpos = nodes
            .iter()
            .position(|n| n.id() == target)
            .unwrap_or(nodes.len());
        let insert_at = if after { tpos + 1 } else { tpos };
        let len = nodes.len();
        nodes.insert(insert_at.min(len), node);
        Ok(())
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
            id: Id::new_unchecked(id),
            label: id.into(),
            parent: parent.map(Id::new_unchecked),
            attribute: (),
        }
    }

    #[test]
    fn valid_forest_has_no_errors() {
        let t = Taxonomy(vec![cat("root", None), cat("child", Some("root"))]);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn duplicate_id_is_detected() {
        let t = Taxonomy(vec![cat("root", None), cat("root", None)]);
        let errs = t.validate().unwrap_err();
        assert!(errs.contains(&ForestError::DuplicateId { id: "root".into() }));
    }

    #[test]
    fn empty_id_is_detected() {
        let t = Taxonomy(vec![cat("", None)]);
        let errs = t.validate().unwrap_err();
        assert!(errs.contains(&ForestError::InvalidId {
            id: "".into(),
            error: IdError::Empty,
        }));
    }

    #[test]
    fn dangling_parent_is_detected() {
        let t = Taxonomy(vec![cat("child", Some("ghost"))]);
        let errs = t.validate().unwrap_err();
        assert!(errs.contains(&ForestError::DanglingParent {
            id: "child".into(),
            parent: "ghost".into(),
        }));
    }

    #[test]
    fn self_loop_is_detected_as_cycle() {
        let t = Taxonomy(vec![cat("a", Some("a"))]);
        let errs = t.validate().unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, ForestError::Cycle { id } if id == "a")));
    }

    #[test]
    fn two_node_cycle_is_detected() {
        let t = Taxonomy(vec![cat("a", Some("b")), cat("b", Some("a"))]);
        let errs = t.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, ForestError::Cycle { .. })));
    }

    #[test]
    fn empty_forest_is_valid() {
        let t: Taxonomy<()> = Taxonomy(vec![]);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn validated_returns_owned_self_when_valid() {
        let t = Taxonomy(vec![cat("root", None)]);
        let got = t.validated().expect("valid forest");
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn validated_returns_err_when_invalid() {
        let t = Taxonomy(vec![cat("a", Some("a"))]);
        assert!(t.validated().is_err());
    }

    #[test]
    fn multiple_errors_are_collected() {
        let t = Taxonomy(vec![
            cat("", None),
            cat("dup", None),
            cat("dup", None),
            cat("x", Some("ghost")),
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
