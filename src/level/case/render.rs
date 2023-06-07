use crate::{level, render::*};
use dodrio::{builder::*, bumpalo};
use wasm_bindgen::JsCast;

impl super::Node {
    fn render<'a>(
        self,
        case: &super::Case,
        cx: &mut dodrio::RenderContext<'a>,
        enable_hover: bool,
        events: bool,
    ) -> dodrio::Node<'a> {
        let [x, y] = case.position(self);
        let x = bumpalo::format!(in cx.bump, "{}", x).into_bump_str();
        let y = bumpalo::format!(in cx.bump, "{}", y).into_bump_str();

        g(cx.bump)
            .children([
                circle(cx.bump)
                    .attributes([
                        attr("r", "0.5"),
                        attr("cx", x),
                        attr("cy", y),
                        attr(
                            "class",
                            if enable_hover && case.node_has_interaction(self) {
                                "node hoverable"
                            } else {
                                "node"
                            },
                        ),
                        attr("pointer-events", if events { "auto" } else { "none" }),
                    ])
                    .listeners([
                        on(
                            cx.bump,
                            "mousedown",
                            handler(move |e| {
                                let (x, y) = to_svg_coords(
                                    e.dyn_into::<web_sys::MouseEvent>().unwrap(),
                                    "game",
                                );
                                crate::Msg::Level(level::Msg::MouseDown(
                                    x,
                                    y,
                                    level::DragObject::Node(self),
                                ))
                            }),
                        ),
                        on(
                            cx.bump,
                            "mouseup",
                            handler(move |e| {
                                let (x, y) = to_svg_coords(
                                    e.dyn_into::<web_sys::MouseEvent>().unwrap(),
                                    "game",
                                );
                                crate::Msg::Level(level::Msg::MouseUp(x, y, Some(self)))
                            }),
                        ),
                    ])
                    .finish(),
                text_(cx.bump)
                    .attributes([
                        attr("text-anchor", "middle"),
                        attr("dominant-baseline", "middle"),
                        attr("pointer-events", "none"),
                        attr("x", x),
                        attr("y", y),
                    ])
                    .children([text(
                        bumpalo::collections::String::from_str_in(
                            case.node_expression(self).text(),
                            cx.bump,
                        )
                        .into_bump_str(),
                    )])
                    .finish(),
            ])
            .finish()
    }
}

impl super::Wire {
    fn render<'a>(
        self,
        outputs: &[(super::Node, usize)],
        case: &super::Case,
        cx: &mut dodrio::RenderContext<'a>,
        enable_hover: bool,
        events: bool,
    ) -> [dodrio::Node<'a>; 2] {
        use bumpalo::collections::Vec;

        let d: &'a str = {
            const WIRE_STIFFNESS: f64 = 0.75;

            let inputs = Vec::from_iter_in(case.wire_inputs(self), cx.bump);
            let start = Vec::from_iter_in(inputs.iter().map(|&node| case.position(node)), cx.bump);
            let start_vector = [0., WIRE_STIFFNESS];
            let end;
            let end_vector;

            if outputs.is_empty() {
                let start_avg = bezier::average(&start);
                end = bumpalo::vec![in cx.bump; [start_avg[0], start_avg[1] + 3. * WIRE_STIFFNESS]];
                end_vector = bumpalo::vec![in cx.bump; [0., WIRE_STIFFNESS]];
            } else {
                end = Vec::from_iter_in(
                    outputs.iter().map(|&(node, _)| case.position(node)),
                    cx.bump,
                );
                end_vector = Vec::from_iter_in(
                    outputs.iter().map(|&(node, idx)| {
                        [
                            -WIRE_STIFFNESS
                                * (idx as f64
                                    - (case.node_expression(node).inputs().len() as f64 - 1.) / 2.),
                            WIRE_STIFFNESS,
                        ]
                    }),
                    cx.bump,
                );
            }
            let (mid, mid_vector) = bezier::split(
                bezier::average(&start),
                start_vector,
                bezier::average(&end_vector),
                bezier::average(&end),
            );

            let mut path = bumpalo::collections::String::new_in(cx.bump);
            for start in start {
                bezier::path(start, start_vector, mid_vector, mid, &mut path);
            }
            for (end, end_vector) in end.into_iter().zip(end_vector) {
                bezier::path(mid, mid_vector, end_vector, end, &mut path);
            }
            path.into_bump_str()
        };

        let extra_classes = if case.proven(self) {
            " known"
        } else if case.wire_eq(self, case.goal()) {
            " goal"
        } else {
            ""
        };

        let hoverable = if enable_hover && case.wire_has_interaction(self) {
            " hoverable"
        } else {
            ""
        };

        let closure = move |e: web_sys::Event| {
            let (x, y) = to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
            crate::Msg::Level(level::Msg::MouseDown(x, y, level::DragObject::Wire(self)))
        };

        [
            path(cx.bump)
                .attributes([
                    attr(
                        "class",
                        bumpalo::format!(in cx.bump, "wire border{}{}", extra_classes, hoverable)
                            .into_bump_str(),
                    ),
                    attr("d", d),
                    attr("pointer-events", if events { "auto" } else { "none" }),
                ])
                .on("mousedown", handler(closure))
                .finish(),
            path(cx.bump)
                .attributes([
                    attr(
                        "class",
                        bumpalo::format!(in cx.bump, "wire{}{}", extra_classes, hoverable)
                            .into_bump_str(),
                    ),
                    attr("d", d),
                    attr("pointer-events", if events { "auto" } else { "none" }),
                ])
                .on("mousedown", handler(closure))
                .finish(),
        ]
    }
}

impl super::Case {
    pub fn render<'a>(
        &self,
        pan_zoom: PanZoom,
        cx: &mut dodrio::RenderContext<'a>,
        unlocks: crate::UnlockState,
        complete: bool,
        dragging: Option<super::Node>,
    ) -> dodrio::Node<'a> {
        svg(cx.bump)
            .attributes([
                attr("id", "game"),
                attr(
                    "class",
                    if complete {
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
                        for svg_node in wire.render(
                            &outputs,
                            self,
                            cx,
                            !complete && unlocks >= crate::UnlockState::Lemmas,
                            dragging.is_none(),
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
                        builder =
                            builder.child(node.render(self, cx, !complete, dragging != Some(node)));
                    }
                    builder.finish()
                },
            ])
            .finish()
    }
}
