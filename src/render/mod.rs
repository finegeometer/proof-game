mod bezier;

use dodrio::{builder::ElementBuilder, bumpalo};
use wasm_bindgen::JsCast;

fn g(
    bump: &bumpalo::Bump,
) -> ElementBuilder<
    bumpalo::collections::Vec<dodrio::Listener>,
    bumpalo::collections::Vec<dodrio::Attribute>,
    bumpalo::collections::Vec<dodrio::Node>,
> {
    let builder = ElementBuilder::new(bump, "g");
    builder.namespace(Some("http://www.w3.org/2000/svg"))
}

fn text_(
    bump: &bumpalo::Bump,
) -> ElementBuilder<
    bumpalo::collections::Vec<dodrio::Listener>,
    bumpalo::collections::Vec<dodrio::Attribute>,
    bumpalo::collections::Vec<dodrio::Node>,
> {
    let builder = ElementBuilder::new(bump, "text");
    builder.namespace(Some("http://www.w3.org/2000/svg"))
}

// https://stackoverflow.com/a/42711775
fn to_svg_coords(e: web_sys::MouseEvent, id: &str) -> (f64, f64) {
    let svg: web_sys::SvgsvgElement = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap();

    let pt: web_sys::SvgPoint = svg.create_svg_point();
    pt.set_x(e.client_x() as f32);
    pt.set_y(e.client_y() as f32);
    let out = pt.matrix_transform(&svg.get_screen_ctm().unwrap().inverse().unwrap());
    (out.x() as f64, out.y() as f64)
}

impl super::Node {
    fn render<'a>(
        self,
        case: &super::Case,
        cx: &mut dodrio::RenderContext<'a>,
    ) -> dodrio::Node<'a> {
        use dodrio::builder::*;

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
                            if case.node_has_interaction(self) {
                                "node hoverable"
                            } else {
                                "node"
                            },
                        ),
                    ])
                    .on("mousedown", move |root, vdom, e| {
                        let model = root.unwrap_mut::<super::Model>();
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                        model.update(super::Msg::MouseDown(x, y, super::DragObject::Node(self)));
                        vdom.schedule_render();
                    })
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
        unlocks: crate::UnlockState,
    ) -> [dodrio::Node<'a>; 2] {
        use bumpalo::collections::Vec;
        use dodrio::builder::*;

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

        let hoverable = if unlocks >= crate::UnlockState::Lemmas && case.wire_has_interaction(self)
        {
            " hoverable"
        } else {
            ""
        };

        let closure =
            move |root: &mut dyn dodrio::RootRender, vdom: dodrio::VdomWeak, e: web_sys::Event| {
                let model = root.unwrap_mut::<super::Model>();
                let (x, y) = to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                model.update(super::Msg::MouseDown(x, y, super::DragObject::Wire(self)));
                vdom.schedule_render();
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
                ])
                .on("mousedown", closure)
                .finish(),
            path(cx.bump)
                .attributes([
                    attr(
                        "class",
                        bumpalo::format!(in cx.bump, "wire{}{}", extra_classes, hoverable)
                            .into_bump_str(),
                    ),
                    attr("d", d),
                ])
                .on("mousedown", closure)
                .finish(),
        ]
    }
}

impl super::Case {
    fn render<'a>(
        &self,
        svg_corners: ([f64; 2], [f64; 2]),
        cx: &mut dodrio::RenderContext<'a>,
        unlocks: crate::UnlockState,
    ) -> dodrio::Node<'a> {
        use dodrio::builder::*;

        svg(cx.bump)
            .attributes([
                attr("id", "game"),
                attr("class", "background"),
                attr("preserveAspectRatio", "xMinYMin slice"),
                attr("font-size", "0.75"),
                attr("style", "top: 2%; height: 86%; left: 9%; width: 82%;"),
                attr(
                    "viewBox",
                    bumpalo::format!(in cx.bump,
                        "{} {} {} {}",
                        svg_corners.0[0],
                        svg_corners.0[1],
                        svg_corners.1[0] - svg_corners.0[0],
                        svg_corners.1[1] - svg_corners.0[1]
                    )
                    .into_bump_str(),
                ),
            ])
            .listeners([
                on(cx.bump, "mousedown", move |root, vdom, e| {
                    let model = root.unwrap_mut::<super::Model>();
                    let (x, y) =
                        to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                    model.update(super::Msg::MouseDown(x, y, super::DragObject::Background));
                    vdom.schedule_render();
                }),
                on(cx.bump, "mouseup", move |root, vdom, e| {
                    let model = root.unwrap_mut::<super::Model>();
                    let (x, y) =
                        to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                    model.update(super::Msg::MouseUp(x, y));
                    vdom.schedule_render();
                }),
                on(cx.bump, "mousemove", move |root, vdom, e| {
                    let model = root.unwrap_mut::<super::Model>();
                    let (x, y) =
                        to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                    model.update(super::Msg::MouseMove(x, y));
                    vdom.schedule_render();
                }),
                on(cx.bump, "wheel", move |root, vdom, e| {
                    let e = e.dyn_into::<web_sys::WheelEvent>().unwrap();
                    let model = root.unwrap_mut::<super::Model>();
                    let wheel = e.delta_y();
                    let (x, y) = to_svg_coords(e.into(), "game");
                    model.update(super::Msg::MouseWheel(x, y, wheel));
                    vdom.schedule_render();
                }),
            ])
            .children([
                // Wires
                {
                    let mut builder = g(cx.bump);
                    for (wire, outputs) in self.wires() {
                        for svg_node in wire.render(&outputs, self, cx, unlocks) {
                            builder = builder.child(svg_node);
                        }
                    }
                    builder.finish()
                },
                // Nodes
                {
                    let mut builder = g(cx.bump);
                    for node in self.nodes() {
                        builder = builder.child(node.render(self, cx));
                    }
                    builder.finish()
                },
            ])
            .finish()
    }
}

impl super::CaseTree {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        unlocks: crate::UnlockState,
    ) -> dodrio::Node<'a> {
        use dodrio::builder::*;

        let mut builder = div(cx.bump);

        if let Some(case) = self.current_case() {
            builder = builder.child(case.render(self.svg_corners, cx, unlocks))
        }

        // Case Selection
        builder = builder
            .child(
                div(cx.bump)
                    .attributes([
                        attr("style", "top: 2%; height: 96%; left: 2%; width: 5%;"),
                        attr(
                            "class",
                            if self.cases_left() == 0 {
                                "background disabled button"
                            } else {
                                "background hoverable button"
                            },
                        ),
                    ])
                    .on("click", move |root, vdom, _| {
                        let model = root.unwrap_mut::<super::Model>();
                        model.update(super::Msg::PrevCase);
                        vdom.schedule_render();
                    })
                    .children([text("◀")])
                    .finish(),
            )
            .child(
                div(cx.bump)
                    .attributes([
                        attr("style", "top: 2%; height: 96%; left: 93%; width: 5%;"),
                        attr(
                            "class",
                            if self.cases_right() == 0 {
                                "background disabled button"
                            } else {
                                "background hoverable button"
                            },
                        ),
                    ])
                    .on("click", move |root, vdom, _| {
                        let model = root.unwrap_mut::<super::Model>();
                        model.update(super::Msg::NextCase);
                        vdom.schedule_render();
                    })
                    .children([text("▶")])
                    .finish(),
            );

        // Reset Level
        builder = builder.child(
            div(cx.bump)
                .attributes([
                    attr("class", "resetButton button"),
                    attr("style", "top: 88%; height: 10%; left: 81%; width: 10%;"),
                ])
                .on("click", move |root, vdom, _| {
                    let model = root.unwrap_mut::<super::Model>();
                    model.update(super::Msg::ResetLevel);
                    vdom.schedule_render();
                })
                .children([text("Reset")])
                .finish(),
        );

        // Next Level
        if self.current_case().is_none() && self.cases_left() == 0 && self.cases_right() == 0 {
            builder = builder.child(
                div(cx.bump)
                    .attributes([attr("class", "nextLevel button")])
                    .on("click", move |root, vdom, _| {
                        let model = root.unwrap_mut::<super::Model>();
                        model.update(super::Msg::NextLevel);
                        vdom.schedule_render();
                    })
                    .children([text("Next Level!")])
                    .finish(),
            );
        }

        if let Some(text_box) = &self.text_box {
            builder = builder.child(
                div(cx.bump)
                    .attributes([
                        attr("class", "background disabled"),
                        attr("style", "top: 92%; height: 6%; left: 9%; width: 82%; text-align: center; vertical-align: middle;"),
                    ])
                    .children([text(
                        bumpalo::collections::String::from_str_in(text_box, cx.bump)
                            .into_bump_str(),
                    )])
                    .finish(),
            );
        }

        builder.finish()
    }
}
