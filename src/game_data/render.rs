use super::*;
use crate::render::*;
use dodrio::{builder::*, bumpalo};

impl GameData {
    pub fn world_map<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        svg_corners: ([f64; 2], [f64; 2]),
    ) -> dodrio::Node<'a> {
        svg(cx.bump)
            .attributes([
                attr("id", "map"),
                attr("class", "background"),
                attr("preserveAspectRatio", "xMidYMid meet"),
                attr("font-size", "0.75"),
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
            .children(bumpalo::collections::Vec::from_iter_in(
                self.levels.iter().enumerate().map(|(level_num, level)| {
                    circle(cx.bump)
                        .attributes([
                            attr("r", "0.5"),
                            attr(
                                "cx",
                                bumpalo::format!(in cx.bump, "{}", &level.map_position[0])
                                    .into_bump_str(),
                            ),
                            attr(
                                "cy",
                                bumpalo::format!(in cx.bump, "{}", &level.map_position[1])
                                    .into_bump_str(),
                            ),
                            attr("class", "node hoverable"),
                        ])
                        .listeners([on(
                            cx.bump,
                            "click",
                            handler(move |_| crate::Msg::LoadLevel(level_num)),
                        )])
                        .finish()
                }),
                cx.bump,
            ))
            .finish()
    }
}