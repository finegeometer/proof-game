mod render;

use super::{
    super::expression::{Expression, Var},
    Case, ValidityReason,
};

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
            if !expression.tycheck(|node| nodes[*node].0.ty()) {
                anyhow::bail!("Node {} fails typechecking.", n)
            }
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
            if nodes[ix].0.ty() != super::Type::TruthValue {
                anyhow::bail!("Hypothesis {} is not a truth value.", ix);
            }
            if ix >= nodes.len() {
                anyhow::bail!("Hypothesis index too large. ({} >= {})", ix, nodes.len())
            }
        }

        if nodes[conclusion].0.ty() != super::Type::TruthValue {
            anyhow::bail!("Conclusion is not a truth value.");
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

    pub fn vars(&self) -> impl '_ + Iterator<Item = Var> {
        self.nodes.iter().filter_map(|(e, _)| {
            if let Expression::Variable(v) = e {
                Some(v.clone())
            } else {
                None
            }
        })
    }

    pub fn add_to_case_tree(
        self,
        case_tree: &mut super::super::case_tree::CaseTree,
        var: impl Fn(&Var) -> super::Node,
        offset: [f64; 2],
    ) {
        let mut wires = Vec::with_capacity(self.nodes.len());

        let mut case = case_tree.case(case_tree.current).0.clone();

        // Create Nodes
        for (expression, position) in self.nodes {
            let node = if let Expression::Variable(v) = &expression {
                var(v)
            } else {
                case.make_node(
                    expression.map(|ix| wires[ix]),
                    [position[0] + offset[0], position[1] + offset[1]],
                )
            };
            wires.push(case.node_output(node));
        }

        // Hypotheses
        let mut subcases = self
            .hypotheses
            .into_iter()
            .map(|h| {
                let mut case = case.clone();
                case.set_goal(wires[h]);
                case
            })
            .collect::<Vec<_>>();

        // Conclusion
        case.set_proven(
            wires[self.conclusion],
            ValidityReason::new("Application of a previously proven theorem."),
        );
        subcases.push(case);

        // Case Split
        case_tree.case_split(subcases);
    }
}
