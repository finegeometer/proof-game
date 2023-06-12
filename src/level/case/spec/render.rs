use crate::render::g;
use dodrio::bumpalo;

use super::{
    super::render::{render_node, render_wire},
    *,
};

impl LevelSpec {
    pub fn render<'a>(&self, cx: &mut dodrio::RenderContext<'a>) -> [dodrio::Node<'a>; 2] {
        [
            // Wires
            {
                use bumpalo::collections::Vec;

                #[allow(clippy::type_complexity)]
                let mut wire_data: Vec<([f64; 2], Vec<[f64; 2]>, Vec<[f64; 2]>)> =
                    Vec::from_iter_in(
                        self.nodes.iter().map(|(_, position)| {
                            (*position, Vec::new_in(cx.bump), Vec::new_in(cx.bump))
                        }),
                        cx.bump,
                    );

                for (expression, position) in self.nodes.iter() {
                    let inputs = expression.inputs();
                    let x = (inputs.len() as f64 - 1.) / 2.;
                    for (ix, &input) in inputs.iter().enumerate() {
                        wire_data[input].1.push(*position);
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
                        *position,
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
