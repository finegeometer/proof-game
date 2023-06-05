// At some time in the future, rethink the file location. Maybe once the sandbox exists?

use serde::Deserialize;

use crate::level_state::LevelState;

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
    pub fn load(&self) -> Result<LevelState, ()> {
        let mut x_min = f64::INFINITY;
        let mut y_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        let mut case = Case::new();
        let mut wires = Vec::with_capacity(self.nodes.len());
        for (op, inputs, position) in &self.nodes {
            x_min = x_min.min(position[0]);
            y_min = y_min.min(position[1]);
            x_max = x_max.max(position[0]);
            y_max = y_max.max(position[1]);

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
        Ok(LevelState::new(
            case,
            ([x_min - 1., y_min - 1.], [x_max + 1., y_max + 3.]),
            self.text_box.map(|s| s.to_owned()),
        ))
    }
}
