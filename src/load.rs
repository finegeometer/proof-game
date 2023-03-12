// At some time in the future, rething the file location. Maybe once the sandbox exists?

use serde::Deserialize;

use crate::{
    expr_graph::{Page, ValidityReason},
    operation::Operation,
};

#[derive(Deserialize)]
pub struct LevelData {
    nodes: Vec<(Operation, Vec<usize>, (f64, f64))>,
    hypotheses: Vec<usize>,
    conclusion: usize,
}

impl LevelData {
    pub fn load(&self) -> Result<Page, ()> {
        Page::new(|s| {
            let mut wires = Vec::with_capacity(self.nodes.len());
            for (op, children, position) in &self.nodes {
                if children.iter().any(|idx| *idx >= wires.len()) {
                    return Err(());
                }
                wires.push(
                    s.make_node(
                        op.clone(),
                        children.iter().map(|idx| wires[*idx]),
                        *position,
                    )
                    .output(s),
                );
            }
            for idx in self.hypotheses.iter() {
                s.set_wire_status(
                    *wires.get(*idx).ok_or(())?,
                    ValidityReason("By assumption."),
                )
            }
            wires.get(self.conclusion).copied().ok_or(())
        })
    }
}
