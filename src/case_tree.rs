use super::case::*;

use smallvec::SmallVec;

pub struct CaseTree {
    nodes: Vec<CaseNode>,
    current: usize,
}

// The currently active cases are those where `!complete && children.is_empty()`
struct CaseNode {
    case: Case,
    complete: bool,
    parent: usize,
    children: SmallVec<[usize; 2]>,
}

#[derive(Debug)]
pub struct CaseId(usize);

impl CaseNode {
    fn new(case: Case, parent: usize) -> Self {
        CaseNode {
            complete: case.proven(case.goal()),
            case,
            parent,
            children: SmallVec::new(),
        }
    }
}

impl CaseTree {
    pub fn new(case: Case) -> Self {
        Self {
            nodes: vec![CaseNode::new(case, 0)],
            current: 0,
        }
    }

    fn mark_complete(&mut self, mut node: usize) {
        loop {
            self.nodes[node].complete = true;

            if node == 0 {
                break;
            }
            node = self.nodes[node].parent;

            if !self.nodes[node]
                .children
                .iter()
                .all(|&node| self.nodes[node].complete)
            {
                break;
            }
        }
    }

    pub fn current_case(&self) -> (&Case, bool) {
        let CaseNode { case, complete, .. } = &self.nodes[self.current];
        (case, *complete)
    }

    pub fn goto_case(&mut self, id: CaseId) {
        self.current = id.0;
    }

    /// Edit the current case, possibly splitting it into several in the process.
    pub fn edit_case(&mut self, fs: impl IntoIterator<Item = impl FnOnce(&mut Case)>) {
        let old_len = self.nodes.len();

        let mut fs = fs.into_iter();

        let Some(f0) = fs.next() else {
                self.mark_complete(self.current);
                return;
            };

        let Some(f1) = fs.next() else {
                let case = &mut self.nodes[self.current].case;
                f0(case);
                if case.proven(case.goal()) {
                    self.mark_complete(self.current);
                }
                return;
            };

        for f in [f0, f1].into_iter().chain(fs) {
            let mut case = self.nodes[self.current].case.clone();
            f(&mut case);
            self.nodes.push(CaseNode::new(case, self.current));
        }

        self.nodes[self.current].children = (old_len..self.nodes.len()).collect();

        if let Some(child) = (old_len..self.nodes.len()).find(|&child| !self.nodes[child].complete)
        {
            self.current = child;
        } else {
            self.mark_complete(self.current)
        }
    }

    pub fn set_node_position(&mut self, node: Node, position: [f64; 2]) {
        self.nodes[self.current].case.set_position(node, position)
    }

    pub fn all_complete(&self) -> bool {
        self.nodes[0].complete
    }
}

mod render {
    use super::*;
    use crate::render::{bezier, g};
    use dodrio::builder::*;
    use dodrio::bumpalo;

    impl CaseTree {
        fn subtree<'a>(
            &self,
            cx: &mut dodrio::RenderContext<'a>,
            node: usize,
            x: &mut f64,
            y: f64,
            y_min: &mut f64,
        ) -> (dodrio::Node<'a>, f64) {
            if y < *y_min {
                *y_min = y;
            }

            let children = &self.nodes[node].children;
            let mut xs: SmallVec<[f64; 2]> = SmallVec::with_capacity(children.len());

            let mut builder = g(cx.bump);

            for &node in children {
                let (subtree, x) = self.subtree(cx, node, x, y - 2., y_min);
                xs.push(x);
                builder = builder.child(subtree);
            }

            if xs.is_empty() {
                let mut clickable = false;

                let mut circle = circle(cx.bump).attributes([
                    attr("r", "0.5"),
                    attr("cx", bumpalo::format!(in cx.bump, "{}", *x).into_bump_str()),
                    attr("cy", bumpalo::format!(in cx.bump, "{}", y).into_bump_str()),
                    attr(
                        "class",
                        if node == self.current {
                            "node goal"
                        } else if self.nodes[node].complete {
                            "node known"
                        } else if self.nodes[node].children.is_empty() {
                            clickable = true;
                            "node hoverable"
                        } else {
                            "node"
                        },
                    ),
                ]);

                if clickable {
                    circle = circle.on("click", move |root, vdom, _| {
                        let model = root.unwrap_mut::<crate::Model>();
                        model.update(crate::Msg::GotoCase(CaseId(node)));
                        vdom.schedule_render();
                    })
                }

                builder = builder.child(circle.finish());

                (builder.finish(), std::mem::replace(x, *x + 2.))
            } else {
                let x0 = xs.iter().copied().sum::<f64>() / (xs.len() as f64);

                let mut d = bumpalo::collections::String::new_in(cx.bump);
                for x in xs {
                    bezier::path([x0, y], [0., -0.5], [0., -0.5], [x, y - 2.], &mut d)
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

                (builder.finish(), x0)
            }
        }

        pub fn render<'a>(&self, cx: &mut dodrio::RenderContext<'a>) -> dodrio::Node<'a> {
            let mut x = 0.;
            let mut y_min = 0.;

            svg(cx.bump)
                .child(self.subtree(cx, 0, &mut x, 0., &mut y_min).0)
                .attributes([
                    attr(
                        "class",
                        if self.all_complete() {
                            "background complete"
                        } else {
                            "background"
                        },
                    ),
                    attr("preserveAspectRatio", "xMidYMax meet"),
                    attr("font-size", "0.75"),
                    attr("style", "top: 2%; height: 18%; left: 82%; width: 18%;"),
                    attr(
                        "viewBox",
                        bumpalo::format!(in cx.bump,
                            "{} {} {} {}",
                            -1,
                            y_min - 1.,
                            x,
                            1. - y_min
                        )
                        .into_bump_str(),
                    ),
                ])
                .finish()
        }
    }
}
