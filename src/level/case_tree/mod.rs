mod render;

use std::ops::{Deref, DerefMut};

use super::case::*;

use smallvec::SmallVec;

pub struct CaseTree {
    nodes: Vec<CaseNode>,
    pub current: CaseId,
    free_list: SmallVec<[usize; 2]>,
}

struct CaseNode {
    case: Case,
    complete: bool,
    parent: usize,
    /// `None` for leaf nodes; `Some` for branches.
    children: Option<SmallVec<[usize; 2]>>,
}

#[derive(Debug, Clone, Copy)]
pub struct CaseId(usize);

impl CaseNode {
    fn new(case: Case, parent: usize) -> Self {
        CaseNode {
            complete: case.proven(case.goal()),
            case,
            parent,
            children: None,
        }
    }
}

impl CaseTree {
    pub fn new(case: Case) -> Self {
        Self {
            nodes: vec![CaseNode::new(case, 0)],
            current: CaseId(0),
            free_list: SmallVec::new(),
        }
    }

    fn mark_complete(&mut self, mut node: usize) {
        loop {
            self.nodes[node].complete = true;

            if node == 0 {
                break;
            }
            node = self.nodes[node].parent;

            if !self.nodes[node]
                .children
                .as_ref()
                .expect("This node is a parent, so it should have children.")
                .iter()
                .all(|&node| self.nodes[node].complete)
            {
                break;
            }
        }
    }

    pub fn case(&self, id: CaseId) -> (&Case, bool) {
        let CaseNode { case, complete, .. } = &self.nodes[id.0];
        (case, *complete)
    }

    pub fn current_case_mut(&mut self) -> CaseRefMut {
        CaseRefMut(self, self.current)
    }

    fn create_case(&mut self, case: Case, parent: usize) -> usize {
        if let Some(node) = self.free_list.pop() {
            self.nodes[node] = CaseNode::new(case, parent);
            node
        } else {
            let node = self.nodes.len();
            self.nodes.push(CaseNode::new(case, parent));
            node
        }
    }

    pub fn case_split(&mut self, subcases: impl IntoIterator<Item = Case>) {
        let mut incomplete_child = None;

        let mut children = SmallVec::new();
        for subcase in subcases {
            let child = self.create_case(subcase, self.current.0);
            children.push(child);
            incomplete_child = incomplete_child.or((!self.nodes[child].complete).then_some(child));
        }
        self.nodes[self.current.0].children = Some(children);

        if let Some(child) = incomplete_child {
            self.current.0 = child;
        } else {
            self.mark_complete(self.current.0)
        }
    }

    pub fn set_node_position(&mut self, node: Node, position: [f64; 2]) {
        self.nodes[self.current.0].case.set_position(node, position)
    }

    pub fn all_complete(&self) -> bool {
        self.nodes[0].complete
    }

    pub fn revert_to(&mut self, case: CaseId) {
        let mut work =
            std::mem::replace(&mut self.nodes[case.0].children, None).unwrap_or_default();
        while let Some(node) = work.pop() {
            self.free_list.push(node);
            if let Some(children) = &mut self.nodes[node].children {
                work.append(children);
            }
        }
        self.current = case;
    }
}

pub struct CaseRefMut<'a>(&'a mut CaseTree, CaseId);

impl Deref for CaseRefMut<'_> {
    type Target = Case;

    fn deref(&self) -> &Self::Target {
        &self.0.nodes[self.1 .0].case
    }
}

impl DerefMut for CaseRefMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.nodes[self.1 .0].case
    }
}

impl Drop for CaseRefMut<'_> {
    fn drop(&mut self) {
        if self.proven(self.goal()) {
            self.0.mark_complete(self.1 .0)
        }
    }
}
