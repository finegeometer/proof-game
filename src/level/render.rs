use super::*;
use crate::game_data::Unlocks;
use crate::render::handler;
use dodrio::builder::*;
use dodrio::bumpalo;

impl State {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        current_level: usize,
        next_level: Option<usize>,
    ) -> [dodrio::Node<'a>; 2] {
        let (case, complete) = self.case_tree.current_case();

        let mut col0 = div(cx.bump).attributes([attr("id", "col0")]);

        // Main Screen
        col0 = col0.child(case.render(
            self.pan_zoom,
            cx,
            self.unlocks,
            complete,
            match self.drag {
                Some(DragState {
                    object: DragObject::Node(node),
                    ..
                }) => Some(node),
                _ => None,
            },
        ));

        // Text Box
        if let Some(text_box) = &self.text_box {
            col0 = col0.child(
                div(cx.bump)
                    .attributes([attr("id", "text-box"), attr("class", "background disabled")])
                    .children([text(
                        bumpalo::collections::String::from_str_in(text_box, cx.bump)
                            .into_bump_str(),
                    )])
                    .finish(),
            );
        }

        let mut col1 = div(cx.bump).attributes([attr("id", "col1")]);

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
        if self.case_tree.all_complete() {
            if let Some(next_level) = next_level {
                col1 = col1.child(
                    div(cx.bump)
                        .attributes([attr("id", "next-level"), attr("class", "button")])
                        .on("click", handler(move |_| crate::Msg::LoadLevel(next_level)))
                        .children([text("Next Level!")])
                        .finish(),
                );
            } else {
                col1 = col1.child(
                    div(cx.bump)
                        .attributes([attr("id", "next-level"), attr("class", "button")])
                        .on(
                            "click",
                            handler(move |_| crate::Msg::LoadMap { recenter: true }),
                        )
                        .children([text("Select a Level!")])
                        .finish(),
                );
            }
        }

        // Reset Level
        col1 = col1.child(
            div(cx.bump)
                .attributes([attr("id", "reset"), attr("class", "button")])
                .on(
                    "click",
                    handler(move |_| crate::Msg::LoadLevel(current_level)),
                )
                .children([text("Reset")])
                .finish(),
        );

        // World Map
        col1 = col1.child(
            div(cx.bump)
                .attributes([attr("id", "return-to-map"), attr("class", "button")])
                .on(
                    "click",
                    handler(move |_| crate::Msg::LoadMap { recenter: false }),
                )
                .children([text("Return to Map")])
                .finish(),
        );

        [col0.finish(), col1.finish()]
    }
}
