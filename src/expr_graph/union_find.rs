use super::core::ExprId;
use std::{cell::RefCell, collections::HashMap};

#[derive(Clone)]
pub struct UnionFind {
    tree: RefCell<HashMap<ExprId, Node>>,
    cycles: HashMap<ExprId, ExprId>,
}

#[derive(Clone, Copy)]
enum Node {
    Root { rank: u8, smallest: ExprId },
    Child { parent: ExprId },
}

impl UnionFind {
    pub fn new() -> Self {
        Self {
            tree: RefCell::new(HashMap::new()),
            cycles: HashMap::new(),
        }
    }

    fn find(&self, node: ExprId) -> (ExprId, u8, ExprId) {
        let x = self.tree.borrow().get(&node).copied();
        match x {
            Some(Node::Root { rank, smallest }) => (node, rank, smallest),
            Some(Node::Child { parent }) => {
                let (root, rank, smallest) = self.find(parent);
                self.tree
                    .borrow_mut()
                    .insert(node, Node::Child { parent: root });
                (root, rank, smallest)
            }
            None => {
                self.tree.borrow_mut().insert(
                    node,
                    Node::Root {
                        rank: 0,
                        smallest: node,
                    },
                );
                (node, 0, node)
            }
        }
    }

    pub fn merge(&mut self, n1: ExprId, n2: ExprId) {
        let (r1, rank1, smallest1) = self.find(n1);
        let (r2, rank2, smallest2) = self.find(n2);
        let smallest = smallest1.min(smallest2);

        if r1 == r2 {
            return;
        }

        // Cycle update
        {
            let s1 = *self.cycles.get(&r1).unwrap_or(&r1);
            let s2 = self.cycles.insert(r2, s1).unwrap_or(r2);
            self.cycles.insert(r1, s2);
        }

        let mut tree = self.tree.borrow_mut();
        if rank1 < rank2 {
            tree.insert(r1, Node::Child { parent: r2 });
            tree.insert(
                r2,
                Node::Root {
                    rank: rank2,
                    smallest,
                },
            );
        } else {
            tree.insert(r2, Node::Child { parent: r1 });
            tree.insert(
                r1,
                Node::Root {
                    rank: if rank1 == rank2 { rank1 + 1 } else { rank1 },
                    smallest,
                },
            );
        }
    }

    /// Find the smallest `ExprId` in the same equivalence class as the input.
    pub fn smallest(&self, e: ExprId) -> ExprId {
        self.find(e).2
    }

    /// Iterator over all the nodes in the same equivalence class as `e`.
    pub fn iter_class(&self, e: ExprId) -> impl Iterator<Item = ExprId> + '_ {
        let mut node = e;
        std::iter::once(node).chain(std::iter::from_fn(move || {
            node = *self.cycles.get(&node).unwrap_or(&node);
            if node == e {
                None
            } else {
                Some(node)
            }
        }))
    }
}
