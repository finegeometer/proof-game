mod render;

use super::case::*;

use smallvec::SmallVec;

pub struct CaseTree {
    nodes: Vec<CaseNode>,
    pub current: CaseId,
    free_list: SmallVec<[usize; 2]>,
}

// The currently active cases are those where `!complete && children.is_empty()`
struct CaseNode {
    case: Case,
    complete: bool,
    parent: usize,
    children: SmallVec<[usize; 2]>,
}

#[derive(Debug, Clone, Copy)]
pub struct CaseId(usize);

impl CaseNode {
    fn new(case: Case, parent: usize) -> Self {
        CaseNode {
            complete: case.proven(case.goal()),
            case,
            parent,
            children: SmallVec::new(),
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

    /// Edit the current case, possibly splitting it into several in the process.
    pub fn edit_case(&mut self, fs: impl IntoIterator<Item = impl FnOnce(&mut Case)>) {
        let mut fs = fs.into_iter();

        let Some(f0) = fs.next() else {
                self.mark_complete(self.current.0);
                return;
            };

        let Some(f1) = fs.next() else {
                let case = &mut self.nodes[self.current.0].case;
                f0(case);
                if case.proven(case.goal()) {
                    self.mark_complete(self.current.0);
                }
                return;
            };

        let mut incomplete_child = None;

        for f in [f0, f1].into_iter().chain(fs) {
            let mut case = self.nodes[self.current.0].case.clone();
            f(&mut case);
            let child = self.create_case(case, self.current.0);
            self.nodes[self.current.0].children.push(child);
            incomplete_child = incomplete_child.or((!self.nodes[child].complete).then_some(child));
        }

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
        let mut work = std::mem::replace(&mut self.nodes[case.0].children, SmallVec::new());
        while let Some(node) = work.pop() {
            self.free_list.push(node);
            work.append(&mut self.nodes[node].children);
        }
        self.current = case;
    }
}
