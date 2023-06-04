// At some time in the future, rethink the file location. Maybe once the sandbox exists?

use serde::Deserialize;

use super::*;

#[derive(Deserialize)]
pub struct LevelData<'a> {
    #[serde(borrow)]
    nodes: Vec<(&'a str, Vec<usize>, [f64; 2])>,
    hypotheses: Vec<usize>,
    conclusion: usize,
    text_box: Option<&'a str>,
}

impl<'a> LevelData<'a> {
    pub fn load(&self) -> Result<CaseTree, ()> {
        let mut case = Case::new();
        let mut wires = Vec::with_capacity(self.nodes.len());
        for (op, inputs, position) in &self.nodes {
            if inputs.iter().any(|idx| *idx >= wires.len()) {
                return Err(());
            }
            let mut inputs = inputs.iter().map(|idx| wires[*idx]);
            let expression = {
                match *op {
                    "∧" => Expression::And(inputs.collect()),
                    "∨" => Expression::Or(inputs.collect()),
                    "⇒" => Expression::Implies({
                        let out = [inputs.next().ok_or(())?, inputs.next().ok_or(())?];
                        if inputs.next().is_some() {
                            return Err(());
                        };
                        out
                    }),
                    "=" => Expression::Equal({
                        let out = [inputs.next().ok_or(())?, inputs.next().ok_or(())?];
                        if inputs.next().is_some() {
                            return Err(());
                        };
                        out
                    }),
                    _ => Expression::Other((*op).into()),
                }
            };
            let node = case.make_node(expression, *position);
            wires.push(case.node_output(node));
        }
        for idx in self.hypotheses.iter() {
            case.set_proven(
                *wires.get(*idx).ok_or(())?,
                ValidityReason::new("By assumption."),
            )
        }
        case.set_goal(wires.get(self.conclusion).copied().ok_or(())?);
        Ok(CaseTree::new(case, self.text_box.map(|s| s.to_owned())))
    }
}
