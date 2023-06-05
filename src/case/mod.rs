mod render;
mod union_find;

use std::{collections::BTreeMap, rc::Rc};

use super::*;
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
}

#[derive(Clone)]
struct Data {
    expression: Expression,
    /// Positions that the nodes will be displayed on the screen.
    /// Units are such that the default size of a node is a diameter 1 circle.
    position: [f64; 2],
    /// A boolean can be proven true, or unproven. (A non-boolean is always treated as unproven.)
    proven: bool,
}

impl Case {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Rc::new(UnionFind::new()),
            goal: None,
        }
    }

    pub fn set_goal(&mut self, goal: Wire) {
        self.goal = Some(goal);
    }

    pub fn goal(&self) -> Wire {
        self.goal
            .expect("Attempt to retrieve goal before setting it.")
    }

    pub fn make_node(&mut self, expression: Expression, position: [f64; 2]) -> Node {
        let n = Node(self.nodes.len());
        self.nodes.push(Data {
            expression,
            position,
            proven: false,
        });
        n
    }

    pub fn node_output(&self, n: Node) -> Wire {
        Wire(n)
    }

    pub fn node_expression(&self, n: Node) -> &Expression {
        &self.nodes[n.0].expression
    }

    pub fn wire_inputs(&self, w: Wire) -> impl Iterator<Item = Node> + '_ {
        self.connections.iter_class(w.0)
    }

    pub fn wire_eq(&self, w1: Wire, w2: Wire) -> bool {
        self.connections.eq(w1.0, w2.0)
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
    }

    pub fn proven(&self, w: Wire) -> bool {
        self.nodes[w.0 .0].proven
    }

    pub fn set_proven(&mut self, w: Wire, _why_valid: ValidityReason) {
        self.nodes[w.0 .0].proven = true;
    }

    pub fn position(&self, n: Node) -> [f64; 2] {
        self.nodes[n.0].position
    }

    pub fn set_position(&mut self, n: Node, position: [f64; 2]) {
        self.nodes[n.0].position = position;
    }

    pub fn nodes(&self) -> impl Iterator<Item = Node> {
        (0..self.nodes.len()).map(Node)
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
