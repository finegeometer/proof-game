use crate::{game_data::Unlocks, level, render::*};
use dodrio::{builder::*, bumpalo};
use wasm_bindgen::JsCast;

pub(super) fn render_node<'a>(
    cx: &mut dodrio::RenderContext<'a>,
    pos: [f64; 2],
    label: &'a str,
    events: Option<super::Node>,
    hoverable: bool,
) -> dodrio::Node<'a> {
    let [x, y] = pos;
    let x = bumpalo::format!(in cx.bump, "{}", x).into_bump_str();
    let y = bumpalo::format!(in cx.bump, "{}", y).into_bump_str();

    let mut circle = circle(cx.bump).attributes([
        attr("r", "0.5"),
        attr("cx", x),
        attr("cy", y),
        attr("class", if hoverable { "node hoverable" } else { "node" }),
        attr(
            "pointer-events",
            if events.is_some() { "auto" } else { "none" },
        ),
    ]);
    if let Some(node) = events {
        circle = circle
            .on(
                "mousedown",
                handler(move |e| {
                    let (x, y) =
                        to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                    crate::Msg::Level(level::Msg::MouseDown(x, y, level::DragObject::Node(node)))
                }),
            )
            .on(
                "mouseup",
                handler(move |e| {
                    let (x, y) =
                        to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                    crate::Msg::Level(level::Msg::MouseUp(x, y, Some(node)))
                }),
            );
    }

    g(cx.bump)
        .children([
            circle.finish(),
            text_(cx.bump)
                .attributes([
                    attr("text-anchor", "middle"),
                    attr("dominant-baseline", "middle"),
                    attr("pointer-events", "none"),
                    attr("x", x),
                    attr("y", y),
                ])
                .children([text(label)])
                .finish(),
        ])
        .finish()
}

/// `status` must be "" or " known" or " goal".
pub(super) fn render_wire<'a>(
    cx: &mut dodrio::RenderContext<'a>,
    inputs: &[[f64; 2]],
    outputs: &[[f64; 2]],
    output_vectors: &[[f64; 2]],
    status: &str,
    events: Option<super::Wire>,
    hoverable: bool,
) -> [dodrio::Node<'a>; 2] {
    const WIRE_STIFFNESS: f64 = 0.75;
    let mut outputs = outputs;
    let mut output_vectors = output_vectors;

    let input_avg = bezier::average(inputs);
    let input_vector = [0., WIRE_STIFFNESS];

    debug_assert_eq!(outputs.len(), output_vectors.len());

    let tmp;
    if outputs.is_empty() {
        tmp = [[input_avg[0], input_avg[1] + 3. * WIRE_STIFFNESS]];
        outputs = &tmp;
        output_vectors = &[[0., WIRE_STIFFNESS]]
    }

    let output_avg = bezier::average(outputs);
    let output_vector_avg = bezier::average(output_vectors);

    let (mid, mid_vector) = bezier::split(
        input_avg,
        input_vector,
        [
            output_vector_avg[0] * WIRE_STIFFNESS,
            output_vector_avg[1] * WIRE_STIFFNESS,
        ],
        output_avg,
    );

    let mut d = bumpalo::collections::String::new_in(cx.bump);
    for &input in inputs {
        bezier::path(input, input_vector, mid_vector, mid, &mut d);
    }
    for (&output, &[x, y]) in outputs.iter().zip(output_vectors) {
        bezier::path(
            mid,
            mid_vector,
            [x * WIRE_STIFFNESS, y * WIRE_STIFFNESS],
            output,
            &mut d,
        );
    }
    let d = d.into_bump_str();

    let mut out0 = path(cx.bump).attributes([
        attr(
            "class",
            bumpalo::format!(in cx.bump, "wire border{}", status).into_bump_str(),
        ),
        attr("d", d),
    ]);
    let mut out1 = path(cx.bump).attributes([
        attr(
            "class",
            bumpalo::format!(in cx.bump, "wire{}{}", status, if hoverable {" hoverable"} else {""})
                .into_bump_str(),
        ),
        attr("d", d),
    ]);

    if let Some(wire) = events {
        let closure = move |e: web_sys::Event| {
            let (x, y) = to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
            crate::Msg::Level(level::Msg::MouseDown(x, y, level::DragObject::Wire(wire)))
        };
        out0 = out0.on("mousedown", handler(closure));
        out1 = out1.on("mousedown", handler(closure));
    }

    [out0.finish(), out1.finish()]
}

impl super::Case {
    pub fn render<'a>(
        &self,
        pan_zoom: PanZoom,
        cx: &mut dodrio::RenderContext<'a>,
        unlocks: Unlocks,
        complete: bool,
        dragging: Option<super::Node>,
        axiom: bool,
    ) -> dodrio::Node<'a> {
        svg(cx.bump)
            .attributes([
                attr("id", "game"),
                attr(
                    "class",
                    if axiom {
                        "background disabled"
                    } else if complete {
                        "background complete"
                    } else {
                        "background"
                    },
                ),
                attr("preserveAspectRatio", "xMidYMid meet"),
                attr("font-size", "0.75"),
                pan_zoom.viewbox(cx.bump),
            ])
            .listeners([
                on(
                    cx.bump,
                    "mousedown",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                        crate::Msg::Level(level::Msg::MouseDown(
                            x,
                            y,
                            level::DragObject::Background,
                        ))
                    }),
                ),
                on(
                    cx.bump,
                    "mouseup",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                        crate::Msg::Level(level::Msg::MouseUp(x, y, None))
                    }),
                ),
                on(
                    cx.bump,
                    "mousemove",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                        crate::Msg::Level(level::Msg::MouseMove(x, y))
                    }),
                ),
                on(
                    cx.bump,
                    "wheel",
                    handler(move |e| {
                        let e = e.dyn_into::<web_sys::WheelEvent>().unwrap();
                        let wheel = e.delta_y();
                        let (x, y) = to_svg_coords(e.into(), "game");
                        crate::Msg::Level(level::Msg::MouseWheel(x, y, wheel))
                    }),
                ),
            ])
            .children([
                // Wires
                {
                    let mut builder = g(cx.bump);
                    for (wire, outputs) in self.wires() {
                        use bumpalo::collections::Vec;

                        for svg_node in render_wire(
                            cx,
                            &Vec::from_iter_in(
                                self.wire_inputs(wire).map(|node| self.position(node)),
                                cx.bump,
                            ),
                            &Vec::from_iter_in(
                                outputs.iter().map(|&(node, _)| self.position(node)),
                                cx.bump,
                            ),
                            &Vec::from_iter_in(
                                outputs.iter().map(|&(node, idx)| {
                                    [
                                        -(idx as f64
                                            - (self.node_expression(node).inputs().len() as f64
                                                - 1.)
                                                / 2.),
                                        1.,
                                    ]
                                }),
                                cx.bump,
                            ),
                            if self.proven(wire) {
                                " known"
                            } else if self.wire_eq(wire, self.goal()) {
                                " goal"
                            } else {
                                ""
                            },
                            (!axiom && dragging.is_none()).then_some(wire),
                            !axiom
                                && !complete
                                && unlocks >= Unlocks::LEMMAS
                                && dragging.is_none()
                                && self.wire_has_interaction(wire),
                        ) {
                            builder = builder.child(svg_node);
                        }
                    }
                    builder.finish()
                },
                // Nodes
                {
                    let mut builder = g(cx.bump);
                    for node in self.nodes() {
                        builder = builder.child(render_node(
                            cx,
                            self.position(node),
                            bumpalo::collections::String::from_str_in(
                                self.node_expression(node).text(),
                                cx.bump,
                            )
                            .into_bump_str(),
                            (!axiom && dragging != Some(node)).then_some(node),
                            !axiom
                                && !complete
                                && dragging.is_none()
                                && self.node_has_interaction(node),
                        ));
                    }
                    builder.finish()
                },
            ])
            .finish()
    }
}
