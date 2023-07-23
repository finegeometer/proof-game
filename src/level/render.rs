use super::*;
use crate::game_data::Unlocks;
use crate::render::g;
use crate::render::handler;
use crate::render::to_svg_coords;
use dodrio::builder::*;
use dodrio::bumpalo;

use wasm_bindgen::JsCast;

impl State {
    fn main_screen<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
    ) -> dodrio::builder::ElementBuilder<
        'a,
        [dodrio::Listener<'a>; 4],
        [dodrio::Attribute<'a>; 4],
        dodrio::bumpalo::collections::Vec<'a, dodrio::Node<'a>>,
    > {
        let (case, complete) = self.case_tree.case(self.case_tree.current);

        let mut main_screen = svg(cx.bump)
            .attributes([
                attr("id", "game"),
                attr(
                    "class",
                    if self.axiom {
                        "background axiom"
                    } else if self.case_tree.current_case_contradiction() {
                        "background disabled"
                    } else if complete {
                        "background complete"
                    } else {
                        "background"
                    },
                ),
                attr("preserveAspectRatio", "xMidYMid meet"),
                self.pan_zoom.viewbox(cx.bump),
            ])
            .listeners([
                on(
                    cx.bump,
                    "mousedown",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                        crate::Msg::Level(Msg::MouseDown(x, y, DragObject::Background))
                    }),
                ),
                on(
                    cx.bump,
                    "mouseup",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                        crate::Msg::Level(Msg::MouseUp(x, y, None))
                    }),
                ),
                on(
                    cx.bump,
                    "mousemove",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "game");
                        crate::Msg::Level(Msg::MouseMove(x, y))
                    }),
                ),
                on(
                    cx.bump,
                    "wheel",
                    handler(move |e| {
                        let e = e.dyn_into::<web_sys::WheelEvent>().unwrap();
                        let wheel = e.delta_y();
                        let (x, y) = to_svg_coords(e.into(), "game");
                        crate::Msg::Level(Msg::MouseWheel(x, y, wheel))
                    }),
                ),
            ]);

        let [wires0, nodes0] = case.render(
            cx,
            match self.drag {
                Some(DragState {
                    object: DragObject::Node(node),
                    confirmed_drag: Ok(()),
                    ..
                }) => Some(node),
                _ => None,
            },
            true,
            |node| match &self.mode {
                Some(Mode::AssignTheoremVars { current, .. }) => {
                    current.1 == case.ty(case.node_output(node))
                }
                Some(Mode::ChooseTheoremLocation(_)) => false,
                Some(Mode::SelectUndo { .. }) => false,
                None => self.interactable() && case.node_has_interaction(node),
            },
            |wire| {
                self.unlocks >= Unlocks::LEMMAS
                    && self.mode.is_none()
                    && self.interactable()
                    && case.wire_has_interaction(wire)
            },
        );
        main_screen = main_screen.child(wires0).child(nodes0);
        main_screen
    }

    fn preview<'a>(&self, cx: &mut dodrio::RenderContext<'a>, case: &Case) -> dodrio::Node<'a> {
        let [wires0, nodes0] = case.render(
            cx,
            match self.drag {
                Some(DragState {
                    object: DragObject::Node(node),
                    confirmed_drag: Ok(()),
                    ..
                }) => Some(node),
                _ => None,
            },
            false,
            |_| false,
            |_| false,
        );
        svg(cx.bump)
            .attributes([
                attr("id", "game"),
                attr("class", "background disabled"),
                attr("preserveAspectRatio", "xMidYMid meet"),
                self.pan_zoom.viewbox(cx.bump),
            ])
            .child(wires0)
            .child(nodes0)
            .finish()
    }

    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        current_level: usize,
        next_level: Option<usize>,
    ) -> [dodrio::Node<'a>; 2] {
        let mut col0 = div(cx.bump).attributes([attr("class", "col wide")]);
        let mut col1 = div(cx.bump).attributes([attr("class", "col narrow")]);

        // Main Screen
        let main_screen = match &self.mode {
            None => self.main_screen(cx).finish(),
            Some(Mode::ChooseTheoremLocation(spec)) => {
                let [wires1, nodes1] = spec.render(cx, self.last_recorded_mouse_position, |_| None);

                self.main_screen(cx)
                    .child(
                        g(cx.bump)
                            .attributes([attr("style", "opacity: 0.5; pointer-events: none;")])
                            .child(wires1)
                            .child(nodes1)
                            .finish(),
                    )
                    .finish()
            }
            Some(Mode::AssignTheoremVars {
                spec,
                offset,
                chosen,
                current,
                remaining: _,
            }) => {
                let [wires1, nodes1] = spec.render(cx, *offset, |v| {
                    if v == current {
                        Some(self.last_recorded_mouse_position)
                    } else {
                        chosen
                            .get(v)
                            .map(|n| self.case_tree.case(self.case_tree.current).0.position(*n))
                    }
                });

                self.main_screen(cx)
                    .child(
                        g(cx.bump)
                            .attributes([attr("style", "opacity: 0.5; pointer-events: none;")])
                            .child(wires1)
                            .child(nodes1)
                            .finish(),
                    )
                    .finish()
            }
            Some(Mode::SelectUndo { preview }) => self.preview(cx, self.case_tree.case(*preview).0),
        };
        col0 = col0.child({
            let mut tmp = div(cx.bump)
                .attr("style", "display: flex; min-height: 0; position: relative;")
                .child(main_screen);
            if !self.axiom {
                tmp = tmp.child({
                    div(cx.bump)
                        .attributes([attr("class", "trash-can")])
                        .listeners([on(
                            cx.bump,
                            "mouseup",
                            handler(|_| {
                                crate::Msg::Level(Msg::MouseUp(0., 0., Some(DropObject::TrashCan)))
                            }),
                        )])
                        .child(text("🗑"))
                        .finish()
                });
            }
            tmp.finish()
        });

        // Text Box
        if let Some((text_box, link)) = &self.text_box {
            col0 = col0.child(
                div(cx.bump)
                    .attributes([attr("class", "text-box")])
                    .children([
                        text(
                            bumpalo::collections::String::from_str_in(text_box, cx.bump)
                                .into_bump_str(),
                        ),
                        text(" ("),
                        a(cx.bump)
                            .attributes([attr(
                                "href",
                                bumpalo::collections::String::from_str_in(link, cx.bump)
                                    .into_bump_str(),
                            )])
                            .children([text("More info")])
                            .finish(),
                        text(")"),
                    ])
                    .finish(),
            );
        }

        // Case Tree
        if self.unlocks >= Unlocks::CASES {
            col1 = col1.child(self.case_tree.render(
                cx,
                matches!(self.mode, Some(Mode::SelectUndo { .. })),
                self.axiom,
            ));

            if matches!(self.mode, Some(Mode::SelectUndo { .. })) {
                col1 = col1.child(
                    div(cx.bump)
                        .attributes([attr("class", "button red")])
                        .on(
                            "mouseover",
                            handler({
                                let current = self.case_tree.current;
                                move |_| {
                                    crate::Msg::Level(crate::level::Msg::RevertPreview(current))
                                }
                            }),
                        )
                        .on("click", handler(move |_| crate::Msg::Level(Msg::Cancel)))
                        .children([text("Cancel undo.")])
                        .finish(),
                )
            } else {
                let current = self.case_tree.current;
                col1 = col1.child(
                    div(cx.bump)
                        .attributes([attr("class", "button red")])
                        .on(
                            "click",
                            handler(move |_| crate::Msg::Level(Msg::RevertPreview(current))),
                        )
                        .children([text("Undo")])
                        .finish(),
                );
            }
        }

        // Blank Space
        col1 = col1.child(
            div(cx.bump)
                .attributes([attr("style", "flex: 1;")])
                .finish(),
        );

        // Next Level
        if self.axiom || self.case_tree.all_complete() {
            #[rustfmt::skip]
            let (listener, s) = if let Some(next_level) = next_level {(
                on(cx.bump, "click", handler(move |_| crate::Msg::GotoLevel(next_level))),
                "Next Level!",
            )} else {(
                on(cx.bump, "click", handler(move |_| crate::Msg::GotoMap { recenter: true })),
                "Select a Level!",
            )};
            col1 = col1.child(
                div(cx.bump)
                    .attributes([attr("class", "button green")])
                    .listeners([listener])
                    .children([text(if self.axiom { "Continue." } else { s })])
                    .finish(),
            );
        }

        if !self.axiom {
            // Reset Level
            col1 = col1.child(
                div(cx.bump)
                    .attributes([attr("class", "button red")])
                    .on(
                        "click",
                        handler(move |_| crate::Msg::GotoLevel(current_level)),
                    )
                    .children([text("Reset")])
                    .finish(),
            );
        }

        if self.interactable() {
            // Apply Theorem
            if self.unlocks >= Unlocks::THEOREM_APPLICATION {
                if matches!(
                    self.mode,
                    Some(Mode::ChooseTheoremLocation { .. } | Mode::AssignTheoremVars { .. })
                ) {
                    col1 = col1.child(
                        div(cx.bump)
                            .attributes([attr("class", "button yellow")])
                            .on("click", handler(move |_| crate::Msg::Level(Msg::Cancel)))
                            .children([text("Cancel Application")])
                            .finish(),
                    );
                } else {
                    col1 = col1.child(
                        div(cx.bump)
                            .attributes([attr("class", "button yellow")])
                            .on("click", handler(move |_| crate::Msg::SelectTheorem))
                            .children([text("Apply Theorem")])
                            .finish(),
                    );
                }
            }
        }

        // World Map
        col1 = col1.child(
            div(cx.bump)
                .attributes([attr("id", "return-to-map"), attr("class", "button blue")])
                .on(
                    "click",
                    handler(move |_| crate::Msg::GotoMap { recenter: false }),
                )
                .children([text("Return to Map")])
                .finish(),
        );

        [col0.finish(), col1.finish()]
    }
}
