mod render;
mod spec;
mod union_find;

pub use spec::LevelSpec;

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use super::expression::{Expression, Type};
use union_find::UnionFind;

/// This is a safety feature.
/// Whenever a proposition is set to be true or false,
/// the caller must explain why this is a reasonable thing to do.
///
/// This should decrease the chance of the proof game being inconsistent.
///
/// The API pretends we have `pub struct ValidityReason(pub &'static str);`,
/// but the struct does not actually store the string.
pub struct ValidityReason {
    _phantom: (),
}
impl ValidityReason {
    pub fn new(_: &str) -> ValidityReason {
        Self { _phantom: () }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Node(usize);

/// Wires are identified with the nodes they emerge from.
/// By using the `Wire` type, you are signifying that
/// wires emerging from different nodes, but that connect together,
/// are to be treated as the same thing.
#[derive(Debug, Clone, Copy)]
pub struct Wire(Node);

#[derive(Clone)]
pub struct Case {
    nodes: Vec<Data>,
    // Two nodes are in the same equivalence class iff their output wires are connected.
    connections: Rc<UnionFind<Node>>,
    goal: Option<Wire>,

    // Keeps track of which nodes describe identical expressions, even if they're displayed separately.
    egg: RefCell<egg::EGraph<Expression<egg::Id>, ()>>,
    node_to_egg: Vec<egg::Id>,
}

#[derive(Debug, Clone)]
struct Data {
    expression: Expression<Wire>,
    /// Positions that the nodes will be displayed on the screen.
    /// Units are such that the default size of a node is a diameter 1 circle.
    position: [f64; 2],
    /// A boolean can be proven true, or unproven. (A non-boolean is always treated as unproven.)
    proven: bool,
    deleted: bool,
}

impl Case {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Rc::new(UnionFind::new()),
            goal: None,
            egg: RefCell::new(egg::EGraph::new(())),
            node_to_egg: Vec::new(),
        }
    }

    pub fn ty(&self, w: Wire) -> Type {
        self.node_expression(w.0).ty()
    }

    pub fn set_goal(&mut self, goal: Wire) {
        self.goal = Some(goal);
    }

    pub fn goal(&self) -> Wire {
        self.goal
            .expect("Attempt to retrieve goal before setting it.")
    }

    pub fn make_node(&mut self, expression: Expression<Wire>, position: [f64; 2]) -> Node {
        let n = Node(self.nodes.len());
        self.nodes.push(Data {
            expression: expression.clone(),
            position,
            proven: false,
            deleted: false,
        });
        self.node_to_egg.push(
            self.egg
                .borrow_mut()
                .add(expression.map(|node| self.node_to_egg[node.0 .0])),
        );
        n
    }

    pub fn node_output(&self, n: Node) -> Wire {
        Wire(n)
    }

    pub fn node_expression(&self, n: Node) -> &Expression<Wire> {
        &self.nodes[n.0].expression
    }

    pub fn wire_inputs(&self, w: Wire) -> impl Iterator<Item = Node> + '_ {
        self.connections
            .iter_class(w.0)
            .filter(|n| !self.nodes[n.0].deleted)
    }

    pub fn wire_eq(&self, w1: Wire, w2: Wire) -> bool {
        self.connections.eq(w1.0, w2.0)
    }

    /// Test whether the wires describe the same *expression*.
    /// For instance, if there are two copies of `a` on screen,
    /// `wire_equiv` will say they are equal, while `wire_eq` will not.
    pub fn wire_equiv(&self, w1: Wire, w2: Wire) -> bool {
        let mut egg = self.egg.borrow_mut();
        if !egg.clean {
            egg.rebuild();
        }
        assert!(egg.clean);

        egg.find(self.node_to_egg[w1.0 .0]) == egg.find(self.node_to_egg[w2.0 .0])
    }

    pub fn connect(&mut self, w1: Wire, w2: Wire, _why_valid: ValidityReason) {
        // Connecting a proven wire to an unproven one should prove the unproven one.
        match (self.proven(w1), self.proven(w2)) {
            (true, true) => (),
            (false, false) => (),
            (true, false) => {
                for n in self.connections.iter_class(w2.0) {
                    self.nodes[n.0].proven = true;
                }
            }
            (false, true) => {
                for n in self.connections.iter_class(w1.0) {
                    self.nodes[n.0].proven = true;
                }
            }
        }

        Rc::make_mut(&mut self.connections).merge(w1.0, w2.0);

        self.egg
            .borrow_mut()
            .union(self.node_to_egg[w1.0 .0], self.node_to_egg[w2.0 .0]);
    }

    pub fn proven(&self, w: Wire) -> bool {
        self.nodes[w.0 .0].proven
    }

    pub fn set_proven(&mut self, w: Wire, _why_valid: ValidityReason) {
        for node in self.connections.iter_class(w.0) {
            self.nodes[node.0].proven = true;
        }
    }

    pub fn set_deleted(&mut self, node: Node) {
        self.nodes[node.0].deleted = true;
    }

    pub fn position(&self, n: Node) -> [f64; 2] {
        self.nodes[n.0].position
    }

    pub fn set_position(&mut self, n: Node, position: [f64; 2]) {
        self.nodes[n.0].position = position;
    }

    pub fn nodes(&self) -> impl '_ + Iterator<Item = Node> {
        (0..self.nodes.len())
            .map(Node)
            .filter(|n| !self.nodes[n.0].deleted)
    }

    pub fn wires(&self) -> impl Iterator<Item = (Wire, Vec<(Node, usize)>)> {
        let mut wires: BTreeMap<usize, Vec<(Node, usize)>> = BTreeMap::new();
        for node in self.nodes() {
            wires.entry(self.connections.canonical(node).0).or_default();
            for (ix, &wire) in self.node_expression(node).inputs().iter().enumerate() {
                wires
                    .entry(self.connections.canonical(wire.0).0)
                    .or_default()
                    .push((node, ix));
            }
        }
        wires.into_iter().map(|(k, v)| (Wire(Node(k)), v))
    }
}
