use super::case::Case;
use super::case_tree::CaseTree;

pub struct LevelState {
    pub case_tree: CaseTree,
    svg_corners: ([f64; 2], [f64; 2]),
    text_box: Option<String>,
}

impl LevelState {
    pub fn new(case: Case, text_box: Option<String>) -> Self {
        Self {
            case_tree: CaseTree::new(case),
            svg_corners: ([-10., -1.], [10., 19.]),
            text_box,
        }
    }

    pub fn scroll_background(&mut self, dx: f64, dy: f64) {
        self.svg_corners.0[0] -= dx;
        self.svg_corners.1[0] -= dx;
        self.svg_corners.0[1] -= dy;
        self.svg_corners.1[1] -= dy;
    }

    pub fn zoom_background(&mut self, x: f64, y: f64, scale_factor: f64) {
        self.svg_corners.0[0] = (self.svg_corners.0[0] - x) * scale_factor + x;
        self.svg_corners.1[0] = (self.svg_corners.1[0] - x) * scale_factor + x;
        self.svg_corners.0[1] = (self.svg_corners.0[1] - y) * scale_factor + y;
        self.svg_corners.1[1] = (self.svg_corners.1[1] - y) * scale_factor + y;
    }
}

mod render {
    use super::*;
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
                    .on("click", move |root, vdom, _| {
                        let model = root.unwrap_mut::<crate::Model>();
                        model.update(crate::Msg::ResetLevel);
                        vdom.schedule_render();
                    })
                    .children([text("Reset")])
                    .finish(),
            );

            // Next Level
            if self.case_tree.all_complete() {
                builder = builder.child(
                    div(cx.bump)
                        .attributes([attr("class", "nextLevel button")])
                        .on("click", move |root, vdom, _| {
                            let model = root.unwrap_mut::<crate::Model>();
                            model.update(crate::Msg::NextLevel);
                            vdom.schedule_render();
                        })
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
}
