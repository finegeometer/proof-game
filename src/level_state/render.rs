use super::*;
use crate::render::handler;
use dodrio::builder::*;
use dodrio::bumpalo;

impl LevelState {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        unlocks: crate::UnlockState,
        dragging: Option<crate::Node>,
    ) -> dodrio::Node<'a> {
        let (case, complete) = self.case_tree.current_case();

        let mut col0 = div(cx.bump).attributes([attr("id", "col0")]);

        // Main Screen
        col0 = col0.child(case.render(self.svg_corners, cx, unlocks, complete, dragging));

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
        if unlocks >= crate::UnlockState::CaseTree {
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
            col1 = col1.child(
                div(cx.bump)
                    .attributes([attr("id", "next-level"), attr("class", "button")])
                    .on("click", handler(move |_| crate::Msg::NextLevel))
                    .children([text("Next Level!")])
                    .finish(),
            );
        }

        // Reset Level
        col1 = col1.child(
            div(cx.bump)
                .attributes([attr("id", "reset"), attr("class", "button")])
                .on("click", handler(move |_| crate::Msg::ResetLevel))
                .children([text("Reset")])
                .finish(),
        );

        div(cx.bump)
            .attributes([attr("id", "top")])
            .children([col0.finish(), col1.finish()])
            .listeners([on(cx.bump, "contextmenu", |_, _, e| e.prevent_default())])
            .finish()
    }
}
