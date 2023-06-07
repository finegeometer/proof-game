mod render;

use super::case::*;

use smallvec::SmallVec;

pub struct CaseTree {
    nodes: Vec<CaseNode>,
    current: usize,
}

// The currently active cases are those where `!complete && children.is_empty()`
struct CaseNode {
    case: Case,
    complete: bool,
    parent: usize,
    children: SmallVec<[usize; 2]>,
}

#[derive(Debug)]
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
            current: 0,
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

    pub fn current_case(&self) -> (&Case, bool) {
        let CaseNode { case, complete, .. } = &self.nodes[self.current];
        (case, *complete)
    }

    pub fn goto_case(&mut self, id: CaseId) {
        self.current = id.0;
    }

    /// Edit the current case, possibly splitting it into several in the process.
    pub fn edit_case(&mut self, fs: impl IntoIterator<Item = impl FnOnce(&mut Case)>) {
        let old_len = self.nodes.len();

        let mut fs = fs.into_iter();

        let Some(f0) = fs.next() else {
                self.mark_complete(self.current);
                return;
            };

        let Some(f1) = fs.next() else {
                let case = &mut self.nodes[self.current].case;
                f0(case);
                if case.proven(case.goal()) {
                    self.mark_complete(self.current);
                }
                return;
            };

        for f in [f0, f1].into_iter().chain(fs) {
            let mut case = self.nodes[self.current].case.clone();
            f(&mut case);
            self.nodes.push(CaseNode::new(case, self.current));
        }

        self.nodes[self.current].children = (old_len..self.nodes.len()).collect();

        if let Some(child) = (old_len..self.nodes.len()).find(|&child| !self.nodes[child].complete)
        {
            self.current = child;
        } else {
            self.mark_complete(self.current)
        }
    }

    pub fn set_node_position(&mut self, node: Node, position: [f64; 2]) {
        self.nodes[self.current].case.set_position(node, position)
    }

    pub fn all_complete(&self) -> bool {
        self.nodes[0].complete
    }
}
