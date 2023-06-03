use std::{cell::RefCell, collections::HashMap, hash::Hash};

#[derive(Clone)]
pub struct UnionFind<T> {
    tree: RefCell<HashMap<T, Node<T>>>,
    cycles: HashMap<T, T>,
}

#[derive(Clone, Copy)]
enum Node<T> {
    Root { rank: u8 },
    Child { parent: T },
}

impl<T: Copy + Eq + Hash> UnionFind<T> {
    pub fn new() -> Self {
        Self {
            tree: RefCell::new(HashMap::new()),
            cycles: HashMap::new(),
        }
    }

    fn find(&self, node: T) -> (T, u8) {
        let x = self.tree.borrow().get(&node).copied();
        match x {
            Some(Node::Root { rank }) => (node, rank),
            Some(Node::Child { parent }) => {
                let (root, rank) = self.find(parent);
                self.tree
                    .borrow_mut()
                    .insert(node, Node::Child { parent: root });
                (root, rank)
            }
            None => {
                self.tree.borrow_mut().insert(node, Node::Root { rank: 0 });
                (node, 0)
            }
        }
    }

    pub fn canonical(&self, node: T) -> T {
        self.find(node).0
    }

    pub fn eq(&self, n1: T, n2: T) -> bool {
        self.canonical(n1) == self.canonical(n2)
    }

    pub fn merge(&mut self, n1: T, n2: T) {
        let (r1, rank1) = self.find(n1);
        let (r2, rank2) = self.find(n2);

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
            tree.insert(r2, Node::Root { rank: rank2 });
        } else {
            tree.insert(r2, Node::Child { parent: r1 });
            tree.insert(
                r1,
                Node::Root {
                    rank: if rank1 == rank2 { rank1 + 1 } else { rank1 },
                },
            );
        }
    }

    /// Iterator over all the nodes in the same equivalence class as `e`.
    pub fn iter_class(&self, e: T) -> impl Iterator<Item = T> + '_ {
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
