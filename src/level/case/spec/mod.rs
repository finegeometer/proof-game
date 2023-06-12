mod render;

use super::{super::expression::Expression, Case, ValidityReason};

#[derive(Debug, Clone)]
pub struct LevelSpec {
    /// Invariant: `nodes[n].inputs()[k] < n`.
    nodes: Vec<(Expression<usize>, [f64; 2])>,
    /// Invariant: `hypotheses[k] < nodes.len()`
    hypotheses: Vec<usize>,
    /// Invariant: `conclusion < nodes.len()`
    conclusion: usize,
}

impl LevelSpec {
    /// Validate a specification.
    pub fn new(
        nodes: Vec<(Expression<usize>, [f64; 2])>,
        hypotheses: Vec<usize>,
        conclusion: usize,
    ) -> anyhow::Result<Self> {
        for (n, (expression, _)) in nodes.iter().enumerate() {
            for ix in expression.inputs() {
                match ix.cmp(&n) {
                    std::cmp::Ordering::Less => {}
                    std::cmp::Ordering::Equal => anyhow::bail!("Node {} depends on itself", n),
                    std::cmp::Ordering::Greater => {
                        anyhow::bail!("Node {} depends on later node {}", n, ix)
                    }
                }
            }
        }

        for &ix in &hypotheses {
            if ix >= nodes.len() {
                anyhow::bail!("Hypothesis index too large. ({} >= {})", ix, nodes.len())
            }
        }

        if conclusion >= nodes.len() {
            anyhow::bail!(
                "Conclusion index too large. ({} >= {})",
                conclusion,
                nodes.len()
            )
        }

        Ok(Self {
            nodes,
            hypotheses,
            conclusion,
        })
    }

    pub fn to_case(&self, offset: [f64; 2]) -> Case {
        let mut case = Case::new();
        let mut wires = Vec::with_capacity(self.nodes.len());

        for (expression, position) in &self.nodes {
            let node = case.make_node(
                expression.clone().map(|ix| wires[ix]),
                [position[0] + offset[0], position[1] + offset[1]],
            );
            wires.push(case.node_output(node));
        }

        for ix in self.hypotheses.iter() {
            case.set_proven(wires[*ix], ValidityReason::new("By assumption."));
        }

        case.set_goal(wires[self.conclusion]);

        case
    }

    pub fn vars(&self) -> impl Iterator<Item = &str> {
        self.nodes.iter().filter_map(|(e, _)| {
            if let Expression::Other(s) = e {
                Some(s.as_str())
            } else {
                None
            }
        })
    }

    pub fn set_node_position(&mut self, node: usize, pos: [f64; 2]) {
        self.nodes[node].1 = pos;
    }
}
