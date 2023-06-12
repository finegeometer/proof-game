use super::*;
use crate::game_data::Unlocks;
use crate::render::handler;
use crate::render::to_svg_coords;
use dodrio::builder::*;
use dodrio::bumpalo;

use wasm_bindgen::JsCast;

impl State {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        current_level: usize,
        next_level: Option<usize>,
    ) -> [dodrio::Node<'a>; 2] {
        let (case, complete) = self.case_tree.current_case();

        let mut col0 = div(cx.bump).attributes([attr("class", "col wide")]);
        let mut col1 = div(cx.bump).attributes([attr("class", "col narrow")]);

        // Main Screen
        col0 = col0.child(
            svg(cx.bump)
                .attributes([
                    attr("id", "game"),
                    attr(
                        "class",
                        if self.axiom {
                            "background disabled"
                        } else if complete {
                            "background complete"
                        } else {
                            "background"
                        },
                    ),
                    attr("preserveAspectRatio", "xMidYMid meet"),
                    attr("font-size", "0.75"),
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
                ])
                .children(case.render(
                    cx,
                    self.unlocks,
                    complete,
                    match self.drag {
                        Some(DragState {
                            object: DragObject::Node(node),
                            confirmed_drag: Ok(()),
                            ..
                        }) => Some(node),
                        _ => None,
                    },
                    self.axiom,
                ))
                .finish(),
        );

        // Text Box
        if let Some(text_box) = &self.text_box {
            col0 = col0.child(
                div(cx.bump)
                    .attributes([attr("class", "background disabled text-box")])
                    .children([text(
                        bumpalo::collections::String::from_str_in(text_box, cx.bump)
                            .into_bump_str(),
                    )])
                    .finish(),
            );
        }

        // Case Tree
        if self.unlocks >= Unlocks::CASES {
            col1 = col1.child(self.case_tree.render(cx));
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

            // Apply Theorem
            if self.unlocks >= Unlocks::THEOREM_APPLICATION {
                col1 = col1.child(
                    div(cx.bump)
                        .attributes([attr("class", "button yellow")])
                        .on("click", handler(move |_| crate::Msg::SelectTheorem))
                        .children([text("Apply Theorem")])
                        .finish(),
                );
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
