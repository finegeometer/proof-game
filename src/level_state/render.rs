use super::*;
use crate::render::handler;
use dodrio::builder::*;
use dodrio::bumpalo;

impl LevelState {
    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        unlocks: crate::UnlockState,
    ) -> dodrio::Node<'a> {
        let mut builder = div(cx.bump);

        let (case, complete) = self.case_tree.current_case();

        builder = builder.child(case.render(self.svg_corners, cx, unlocks, complete));

        if unlocks >= crate::UnlockState::CaseTree {
            builder = builder.child(self.case_tree.render(cx));
        }

        // Reset Level
        builder = builder.child(
            div(cx.bump)
                .attributes([
                    attr("class", "resetButton button"),
                    attr("style", "top: 88%; height: 10%; left: 81%; width: 10%;"),
                ])
                .on("click", handler(move |_| crate::Msg::ResetLevel))
                .children([text("Reset")])
                .finish(),
        );

        // Next Level
        if self.case_tree.all_complete() {
            builder = builder.child(
                div(cx.bump)
                    .attributes([attr("class", "nextLevel button")])
                    .on("click", handler(move |_| crate::Msg::NextLevel))
                    .children([text("Next Level!")])
                    .finish(),
            );
        }

        // Text Box
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
