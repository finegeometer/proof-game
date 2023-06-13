use crate::render::g;
use dodrio::bumpalo;

use super::{
    super::render::{render_node, render_wire},
    *,
};

impl LevelSpec {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        offset: [f64; 2],
        mut var_position: impl FnMut(&str) -> Option<[f64; 2]>,
    ) -> [dodrio::Node<'a>; 2] {
        let mut correct_position = |expression: &Expression<usize>, position: &[f64; 2]| {
            if let Expression::Variable(v) = expression {
                if let Some(pos) = var_position(v) {
                    return pos;
                }
            }
            [position[0] + offset[0], position[1] + offset[1]]
        };

        [
            // Wires
            {
                use bumpalo::collections::Vec;

                #[allow(clippy::type_complexity)]
                let mut wire_data: Vec<([f64; 2], Vec<[f64; 2]>, Vec<[f64; 2]>)> =
                    Vec::from_iter_in(
                        self.nodes.iter().map(|(expression, position)| {
                            (
                                correct_position(expression, position),
                                Vec::new_in(cx.bump),
                                Vec::new_in(cx.bump),
                            )
                        }),
                        cx.bump,
                    );

                for (expression, position) in self.nodes.iter() {
                    let inputs = expression.inputs();
                    let x = (inputs.len() as f64 - 1.) / 2.;
                    for (ix, &input) in inputs.iter().enumerate() {
                        wire_data[input]
                            .1
                            .push(correct_position(expression, position));
                        wire_data[input].2.push([-(ix as f64 - x), 1.]);
                    }
                }

                let mut builder = g(cx.bump);
                for (node, (input, outputs, output_vectors)) in wire_data.into_iter().enumerate() {
                    for svg_node in render_wire(
                        cx,
                        &[input],
                        &outputs,
                        &output_vectors,
                        if self.hypotheses.contains(&node) {
                            " known"
                        } else if self.conclusion == node {
                            " goal"
                        } else {
                            ""
                        },
                        None,
                        false,
                    ) {
                        builder = builder.child(svg_node);
                    }
                }

                builder.finish()
            },
            // Nodes
            {
                let mut builder = g(cx.bump);
                for (expression, position) in self.nodes.iter() {
                    builder = builder.child(render_node(
                        cx,
                        correct_position(expression, position),
                        bumpalo::collections::String::from_str_in(expression.text(), cx.bump)
                            .into_bump_str(),
                        None,
                        false,
                    ));
                }
                builder.finish()
            },
        ]
    }
}
