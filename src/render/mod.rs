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
        outputs: impl Iterator<Item = (super::Node, usize)>,
        case: &super::Case,
        cx: &mut dodrio::RenderContext<'a>,
    ) -> [dodrio::Node<'a>; 2] {
        use dodrio::builder::*;

        // Compute Bezier
        let bezier: &'a str = {
            const WIRE_STIFFNESS: f64 = 0.75;

            let ab: Vec<([f64; 2], [f64; 2])> = case
                .wire_inputs(self)
                .map(|node| {
                    let [x, y] = case.position(node);
                    ([x, y], [x, y + WIRE_STIFFNESS])
                })
                .collect();

            let mut fg: Vec<([f64; 2], [f64; 2])> = outputs
                .map(|(node, idx)| {
                    let [x, y] = case.position(node);
                    (
                        [
                            x + WIRE_STIFFNESS
                                * (idx as f64
                                    - (case.node_expression(node).inputs().len() as f64 - 1.) / 2.),
                            y - WIRE_STIFFNESS,
                        ],
                        [x, y],
                    )
                })
                .collect();

            if fg.is_empty() {
                let x_avg = ab.iter().map(|&([a, _], _)| a).sum::<f64>() / (ab.len() as f64);
                let y_avg = ab.iter().map(|&([_, a], _)| a).sum::<f64>() / (ab.len() as f64);
                fg = vec![(
                    [x_avg, y_avg + 2. * WIRE_STIFFNESS],
                    [x_avg, y_avg + 3. * WIRE_STIFFNESS],
                )]
            }

            let ([c_x, c_y], [d_x, d_y], [e_x, e_y]) =
                connect_bezier(ab.iter().copied(), fg.iter().copied());

            use std::fmt::Write;
            let mut path = bumpalo::collections::String::new_in(cx.bump);
            for ([a_x, a_y], [b_x, b_y]) in ab {
                write!(
                    path,
                    "M {a_x} {a_y} C {b_x} {b_y}, {c_x} {c_y}, {d_x} {d_y}"
                )
                .unwrap();
            }
            for ([f_x, f_y], [g_x, g_y]) in fg {
                write!(
                    path,
                    "M {d_x} {d_y} C {e_x} {e_y}, {f_x} {f_y}, {g_x} {g_y}"
                )
                .unwrap();
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

        let hoverable = if case.wire_has_interaction(self) {
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
                    attr("d", bezier),
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
                    attr("d", bezier),
                ])
                .on("mousedown", closure)
                .finish(),
        ]
    }
}

/// Given points `A_i`, `B_i`, `F_j`, and `G_j`,
/// find points `C`, `D`, and `E` such that
/// the bezier splines `A_i B_i C D E F_j G_j` have:
/// 1. Continuous first derivative at `D`
/// 2. Minimum discontinuity in the second derivative at `D`.
/// 3. Consistent with the above, minimum discontinuity in the third derivative at `D`.
fn connect_bezier(
    ab: impl Iterator<Item = ([f64; 2], [f64; 2])>,
    fg: impl Iterator<Item = ([f64; 2], [f64; 2])>,
) -> ([f64; 2], [f64; 2], [f64; 2]) {
    let a: [f64; 2];
    let b: [f64; 2];
    let f: [f64; 2];
    let g: [f64; 2];

    {
        let mut a_sum = [0.; 2];
        let mut b_sum = [0.; 2];
        let mut ab_count = 0;

        for (ai, bi) in ab {
            a_sum[0] += ai[0];
            a_sum[1] += ai[1];
            b_sum[0] += bi[0];
            b_sum[1] += bi[1];
            ab_count += 1;
        }

        let ab_count = f64::from(ab_count);
        a = [a_sum[0] / ab_count, a_sum[1] / ab_count];
        b = [b_sum[0] / ab_count, b_sum[1] / ab_count];
    }

    {
        let mut f_sum = [0.; 2];
        let mut g_sum = [0.; 2];
        let mut fg_count = 0;

        for (fi, gi) in fg {
            f_sum[0] += fi[0];
            f_sum[1] += fi[1];
            g_sum[0] += gi[0];
            g_sum[1] += gi[1];
            fg_count += 1;
        }

        let fg_count = f64::from(fg_count);

        f = [f_sum[0] / fg_count, f_sum[1] / fg_count];
        g = [g_sum[0] / fg_count, g_sum[1] / fg_count];
    }

    // Continuity of first derivative implies `E-D = D-C`.
    // Minimum discontinuity of second derivative further implies `E-D = D-C = (F-B) / 4`.
    // Minimum discontinuity of third derivative further implies `D = ((3B-A) + (3F-G)) / 4`.

    let d = [
        (3. * b[0] - a[0] + 3. * f[0] - g[0]) / 4.,
        (3. * b[1] - a[1] + 3. * f[1] - g[1]) / 4.,
    ];
    let v = [(f[0] - b[0]) / 4., (f[1] - b[1]) / 4.];
    ([d[0] - v[0], d[1] - v[1]], d, [d[0] + v[0], d[1] + v[1]])
}

impl super::Case {
    fn render<'a>(
        &self,
        svg_corners: ([f64; 2], [f64; 2]),
        cx: &mut dodrio::RenderContext<'a>,
    ) -> dodrio::Node<'a> {
        use dodrio::builder::*;

        svg(cx.bump)
            .attributes([
                attr("id", "game"),
                attr("class", "background"),
                attr("preserveAspectRatio", "xMinYMin slice"),
                attr("font-size", "0.75"),
                attr("style", "top: 2%; height: 96%; left: 9%; width: 82%;"),
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
                        for svg_node in wire.render(outputs.into_iter(), self, cx) {
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

impl<'a> dodrio::Render<'a> for super::CaseTree {
    fn render(&self, cx: &mut dodrio::RenderContext<'a>) -> dodrio::Node<'a> {
        use dodrio::builder::*;

        let mut builder = div(cx.bump);

        if let Some(case) = self.current_case() {
            builder = builder.child(case.render(self.svg_corners, cx))
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
                    .attributes([
                        attr("class", "nextLevel button"),
                        // attr("style", "top: 88%; height: 10%; left: 81%; width: 10%;"),
                    ])
                    .on("click", move |root, vdom, _| {
                        let model = root.unwrap_mut::<super::Model>();
                        model.update(super::Msg::NextLevel);
                        vdom.schedule_render();
                    })
                    .children([text("Next Level!")])
                    .finish(),
            );
        }

        builder.finish()
    }
}
