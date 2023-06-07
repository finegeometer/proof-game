use super::*;
use crate::{game_data::GameData, render::*};
use dodrio::{builder::*, bumpalo};
use wasm_bindgen::JsCast;

impl State {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        game_data: &GameData,
        global_state: &crate::GlobalState,
    ) -> dodrio::Node<'a> {
        let mut builder = svg(cx.bump)
            .attributes([
                attr("id", "map"),
                attr("class", "background"),
                attr("preserveAspectRatio", "xMidYMid meet"),
                attr("font-size", "0.75"),
                global_state.map_panzoom.viewbox(cx.bump),
            ])
            .listeners([
                on(
                    cx.bump,
                    "mousedown",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "map");
                        crate::Msg::WorldMap(Msg::MouseDown(x, y))
                    }),
                ),
                on(
                    cx.bump,
                    "mouseup",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "map");
                        crate::Msg::WorldMap(Msg::MouseUp(x, y))
                    }),
                ),
                on(
                    cx.bump,
                    "mousemove",
                    handler(move |e| {
                        let (x, y) =
                            to_svg_coords(e.dyn_into::<web_sys::MouseEvent>().unwrap(), "map");
                        crate::Msg::WorldMap(Msg::MouseMove(x, y))
                    }),
                ),
                on(
                    cx.bump,
                    "wheel",
                    handler(move |e| {
                        let e = e.dyn_into::<web_sys::WheelEvent>().unwrap();
                        let wheel = e.delta_y();
                        let (x, y) = to_svg_coords(e.into(), "map");
                        crate::Msg::WorldMap(Msg::MouseWheel(x, y, wheel))
                    }),
                ),
            ]);

        let mut d = bumpalo::collections::String::new_in(cx.bump);
        for level in 0..game_data.num_levels() {
            for prereq in game_data.prereqs(level) {
                bezier::path(
                    game_data.map_position(prereq),
                    game_data.bezier_vector(prereq),
                    game_data.bezier_vector(level),
                    game_data.map_position(level),
                    &mut d,
                )
            }
        }
        let d = d.into_bump_str();

        builder = builder
            .child(
                path(cx.bump)
                    .attributes([attr("class", "wire border"), attr("d", d)])
                    .finish(),
            )
            .child(
                path(cx.bump)
                    .attributes([attr("class", "wire"), attr("d", d)])
                    .finish(),
            );

        for level in 0..game_data.num_levels() {
            let prereqs_complete = game_data
                .prereqs(level)
                .all(|prereq| global_state.completed[prereq]);

            let mut circle = circle(cx.bump).attributes([
                attr("r", "0.5"),
                attr(
                    "cx",
                    bumpalo::format!(in cx.bump, "{}", &game_data.map_position(level)[0])
                        .into_bump_str(),
                ),
                attr(
                    "cy",
                    bumpalo::format!(in cx.bump, "{}", &game_data.map_position(level)[1])
                        .into_bump_str(),
                ),
                attr(
                    "class",
                    if global_state.completed[level] {
                        "node hoverable known"
                    } else if prereqs_complete {
                        "node hoverable goal"
                    } else {
                        "node"
                    },
                ),
            ]);

            if prereqs_complete {
                circle = circle.on("click", handler(move |_| crate::Msg::LoadLevel(level)));
            }

            builder = builder.child(circle.finish());
        }

        builder.finish()
    }
}
