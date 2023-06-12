use super::*;
use crate::{game_data::GameData, render::*};
use dodrio::{builder::*, bumpalo};
use wasm_bindgen::JsCast;

impl State {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        game_data: &GameData,
        panzoom: &PanZoom,
        save_data: &crate::SaveData,
        is_theorem_select: bool,
    ) -> dodrio::Node<'a> {
        let mut builder = svg(cx.bump)
            .attributes([
                attr("id", "map"),
                attr("class", "background"),
                attr("preserveAspectRatio", "xMidYMid meet"),
                attr("font-size", "0.75"),
                panzoom.viewbox(cx.bump),
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
            for &prereq in &game_data.level(level).prereqs {
                bezier::path(
                    game_data.level(prereq).map_position,
                    game_data.level(prereq).bezier_vector,
                    game_data.level(level).bezier_vector,
                    game_data.level(level).map_position,
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
                .level(level)
                .prereqs
                .iter()
                .all(|&prereq| save_data.completed(prereq));

            let mut circle = circle(cx.bump).attributes([
                attr("r", "0.5"),
                attr(
                    "cx",
                    bumpalo::format!(in cx.bump, "{}", &game_data.level(level).map_position[0])
                        .into_bump_str(),
                ),
                attr(
                    "cy",
                    bumpalo::format!(in cx.bump, "{}", &game_data.level(level).map_position[1])
                        .into_bump_str(),
                ),
                attr(
                    "class",
                    bumpalo::format!(in cx.bump, "node{}{}", if game_data.level(level).axiom {" axiom"} else {""}, if save_data.completed(level) {
                        " hoverable known"
                    } else if !is_theorem_select && prereqs_complete {
                        " hoverable goal"
                    } else {
                        ""
                    }).into_bump_str()
                    ,
                ),
            ]);

            #[allow(clippy::collapsible_else_if)]
            if is_theorem_select {
                if save_data.completed(level) {
                    circle = circle
                        .on(
                            "click",
                            handler(move |_| crate::Msg::SelectedTheorem(level)),
                        )
                        .on(
                            "mouseover",
                            handler(move |_| crate::Msg::PreviewTheorem(level)),
                        );
                }
            } else {
                if prereqs_complete {
                    circle = circle.on("click", handler(move |_| crate::Msg::GotoLevel(level)));
                }
            }

            builder = builder.child(circle.finish());
        }

        builder.finish()
    }
}
