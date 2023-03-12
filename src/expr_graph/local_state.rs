use std::{collections::HashMap, hash::Hash};

use smallvec::SmallVec;

use super::{
    core::{expr_data, expr_id, ExprData, ExprId},
    union_find,
};

/// This is a safety feature.
/// Whenever a proposition is set to be true or false,
/// the caller must explain why this is a reasonable thing to do.
///
/// This should decrease the chance of the proof game being inconsistent.
pub struct ValidityReason(pub &'static str);

// Should always be in canonical form: the smallest element of the equivalence class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node(ExprId);

// Should always be in canonical form: the node whose children are the canonical forms of the wires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wire(ExprId);

#[derive(Clone)]
pub struct LocalState {
    connections: union_find::UnionFind,
    /// While the tree of expressions is infinite, we only want to display a finite portion.
    /// The keys of this map are the visible wires.
    /// The value associated with a key is the list of visible nodes that point to this wire,
    /// along with the index of the input that does the pointing.
    ///
    /// Note: Why is visibility based on wires?
    /// Because given any two nodes whose outputs are connected, both are visible.
    /// This happens because you can't connect nodes until you've created them.
    ///
    /// Invariants: If a node is visible, so are the things it points to.
    ///
    /// Note: This module only hands out visible nodes.
    /// So assuming a lack of cross-contamination between *different* `LocalState` instances,
    /// we can assume any `Node` we are given is visible.
    visible_parents: HashMap<Wire, SmallVec<[(Node, u32); 2]>>,
    /// A boolean wire may be proven or unknown.
    /// (Non-boolean wires are always treated as unknown.)
    status: HashMap<Wire, ()>,

    /// Positions that nodes will be displayed on the screen.
    /// Units are such that the default size of a node is a diameter 1 circle.
    display_positions: HashMap<Node, (f64, f64)>,
}

impl LocalState {
    pub fn new() -> Self {
        Self {
            connections: union_find::UnionFind::new(),
            visible_parents: HashMap::new(),
            status: HashMap::new(),
            display_positions: HashMap::new(),
        }
    }

    /// Declare two wires to be equal.
    /// Warning: It is up to the caller to ensure this makes sense,
    /// including consideration of whether the expressions are even defined in this scope.
    pub fn connect(&mut self, w1: Wire, w2: Wire, _why_valid: ValidityReason) {
        let mut work = vec![(w1, w2)];
        while let Some((mut w1, mut w2)) = work.pop() {
            // This function is tricky, because it *changes* the canonical forms of nodes and wires.
            // As such, I have to update every `Wire` and `Node` in the state.

            w1 = Wire(self.connections.smallest(w1.0));
            w2 = Wire(self.connections.smallest(w2.0));

            match w1.cmp(&w2) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => continue,
                std::cmp::Ordering::Greater => std::mem::swap(&mut w1, &mut w2),
            }
            // `w1` will be the new canonical form of the wire.

            self.connections.merge(w1.0, w2.0);

            //
            // Handling `status`:
            //

            if let Some(()) = self.status.remove(&w2) {
                self.set_wire_status(
                    w1,
                    ValidityReason(
                        "
We are in the process of connecting `w1` and `w2`.
This means we know that they are equal.
In particular, if `w2` is known to be true, so is `w1`.",
                    ),
                );
            }

            //
            // Handling `visible_parents`:
            //

            let parents2 = self.visible_parents.remove(&w2).unwrap_or(SmallVec::new());
            let parents1 = self.visible_parents.entry(w1).or_default();

            // For every port pointing at `w2`, we need to repoint it at `w1`.
            for (n2, idx) in parents2 {
                // `n2` used to be in canonical form, but is no longer.

                let mut data = expr_data(n2.0).clone();
                for child in data.children.iter_mut() {
                    if *child == w2.0 {
                        *child = w1.0;
                    }
                }
                let n1 = Node(expr_id(&data));

                parents1.push((n1, idx));

                // Now, one final thing:
                // `n2` may have also been in use indirectly; via its output wire.
                // To handle this, we recursively connect `n2`'s output wire to `n1`'s.
                // TODO: Is this guaranteed to terminate?
                work.push((Wire(n1.0), Wire(n2.0)));
            }
        }
    }

    pub fn wire_status(&self, w: Wire) -> Option<()> {
        self.status.get(&w).copied()
    }
    pub fn set_wire_status(&mut self, w: Wire, _why_valid: ValidityReason) {
        self.status.insert(w, ());
    }

    pub fn make_node(
        &mut self,
        op: super::operation::Operation,
        children: impl Iterator<Item = Wire>,
        position: (f64, f64),
    ) -> Node {
        let n = Node::from_data(op, children);
        let w = n.output(self);

        if let std::collections::hash_map::Entry::Vacant(entry) = self.visible_parents.entry(w) {
            entry.insert(SmallVec::new());
        } else {
            return n;
        }

        self.display_positions.insert(n, position);

        for (idx, w2) in n.data().1.enumerate() {
            self
                    .visible_parents
                    .get_mut(&w2)
                    .expect("When a node is created, its children should already be visible, as they were themselves created by `make_node`.")
                    .push((n, idx as u32));
        }

        n
    }

    pub fn visible_wires(&self) -> impl Iterator<Item = Wire> + '_ {
        self.visible_parents.keys().copied()
    }
}

impl Node {
    pub fn output(self, state: &LocalState) -> Wire {
        Wire::new(self.0, state)
    }

    pub fn data(self) -> (super::operation::Operation, impl Iterator<Item = Wire>) {
        let ExprData {
            operation,
            children,
        } = expr_data(self.0);

        (operation, children.into_iter().map(Wire))
    }

    fn from_data(op: super::operation::Operation, children: impl Iterator<Item = Wire>) -> Node {
        Node(expr_id(&ExprData {
            operation: op,
            children: children.map(|x| x.0).collect(),
        }))
    }

    pub fn position(self, state: &LocalState) -> (f64, f64) {
        *state
            .display_positions
            .get(&self)
            .expect("self should have a position, as it should have been created by `make_node`.")
    }

    pub fn set_position(self, state: &mut LocalState, x: f64, y: f64) {
        *state.display_positions.get_mut(&self).expect(
            "self should have a position, as it should have been created by `make_node`.",
        ) = (x, y);
    }
}

impl Wire {
    pub fn new(e: ExprId, state: &LocalState) -> Self {
        Self(state.connections.smallest(e))
    }

    pub fn inputs(self, state: &LocalState) -> Vec<Node> {
        let mut out: Vec<_> = state.connections.iter_class(self.0).map(Node).collect();
        // `iter_class` may return excess non-canonical nodes. Remove those.
        out.retain(|&node| {
            expr_data(node.0)
                .children
                .iter()
                .all(|&child| state.connections.smallest(child) == child)
        });
        out
    }

    pub fn outputs(self, state: &LocalState) -> &[(Node, u32)] {
        if let Some(out) = state.visible_parents.get(&self) {
            out
        } else {
            &[]
        }
    }
}
