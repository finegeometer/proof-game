use super::*;
use crate::render::text_;
use crate::render::{bezier, g, handler};
use dodrio::builder::*;
use dodrio::bumpalo;

impl CaseTree {
    /// Returns the svg `g` node, the `x` position of the root of this subtree, and whether this subtree contains the current node.
    fn subtree<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        node: usize,
        x: &mut f64,
        y: f64,
        y_min: &mut f64,
        undo_buttons: &mut Vec<(usize, [f64; 2])>,
    ) -> (dodrio::Node<'a>, f64, bool) {
        if y < *y_min {
            *y_min = y;
        }

        match self.nodes[node].children.as_ref().map(SmallVec::as_slice) {
            // Leaf
            None => {
                let mut clickable = false;

                let mut circle = circle(cx.bump).attributes([
                    attr("r", "0.5"),
                    attr("cx", bumpalo::format!(in cx.bump, "{}", *x).into_bump_str()),
                    attr("cy", bumpalo::format!(in cx.bump, "{}", y).into_bump_str()),
                    attr(
                        "class",
                        if node == self.current.0 {
                            "node goal"
                        } else if self.nodes[node].complete {
                            "node known"
                        } else {
                            clickable = true;
                            "node hoverable"
                        },
                    ),
                ]);

                if clickable {
                    circle = circle.on(
                        "click",
                        handler(move |_| {
                            crate::Msg::Level(crate::level::Msg::GotoCase(CaseId(node)))
                        }),
                    )
                }

                (
                    g(cx.bump).children([circle.finish()]).finish(),
                    std::mem::replace(x, *x + 2.),
                    node == self.current.0,
                )
            }
            // Empty Branch
            Some(&[]) => (g(cx.bump).finish(), std::mem::replace(x, *x + 2.), true),
            // Branch
            Some(children) => {
                let mut xs: SmallVec<[(f64, bool); 2]> = SmallVec::with_capacity(children.len());

                let mut subtrees = bumpalo::collections::Vec::new_in(cx.bump);

                let mut contains_current = false;
                for &node in children {
                    let (subtree, x, subtree_contains_current) =
                        self.subtree(cx, node, x, y - 2., y_min, undo_buttons);
                    xs.push((x, self.nodes[node].complete));
                    subtrees.push(subtree);
                    contains_current |= subtree_contains_current;
                }

                let x0 = xs.iter().map(|&(x, _)| x).sum::<f64>() / (xs.len() as f64);

                let mut d_complete = bumpalo::collections::String::new_in(cx.bump);
                let mut d_incomplete = bumpalo::collections::String::new_in(cx.bump);
                for (x, complete) in xs {
                    bezier::path(
                        [x0, y],
                        [0., -0.5],
                        [0., -0.5],
                        [x, y - 2.],
                        if complete {
                            &mut d_complete
                        } else {
                            &mut d_incomplete
                        },
                    )
                }
                let d_complete = d_complete.into_bump_str();
                let d_incomplete = d_incomplete.into_bump_str();

                let n = subtrees.len();
                subtrees.push(
                    path(cx.bump)
                        .attributes([attr("class", "wire known border"), attr("d", d_complete)])
                        .finish(),
                );
                subtrees.swap(0, n);

                let n = subtrees.len();
                subtrees.push(
                    path(cx.bump)
                        .attributes([attr("class", "wire border"), attr("d", d_incomplete)])
                        .finish(),
                );
                subtrees.swap(1, n);

                let n = subtrees.len();
                subtrees.push(
                    path(cx.bump)
                        .attributes([attr("class", "wire known"), attr("d", d_complete)])
                        .finish(),
                );
                subtrees.swap(2, n);

                let n = subtrees.len();
                subtrees.push(
                    path(cx.bump)
                        .attributes([attr("class", "wire"), attr("d", d_incomplete)])
                        .finish(),
                );
                subtrees.swap(3, n);

                if contains_current {
                    undo_buttons.push((node, [x0, y]));
                }

                (g(cx.bump).children(subtrees).finish(), x0, contains_current)
            }
        }
    }

    pub fn render<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        render_undo_buttons: bool,
        axiom: bool,
    ) -> dodrio::Node<'a> {
        let mut x = 0.;
        let mut y_min = 0.;

        let mut undo_buttons = Vec::new();

        let mut svg = svg(cx.bump).child(
            self.subtree(cx, 0, &mut x, 0., &mut y_min, &mut undo_buttons)
                .0,
        );

        if render_undo_buttons {
            svg = svg.child(
                g(cx.bump)
                    .children(bumpalo::collections::Vec::from_iter_in(
                        undo_buttons.into_iter().flat_map(|(id, [x, y])| {
                            let x = bumpalo::format!(in cx.bump, "{}", x).into_bump_str();
                            let y = bumpalo::format!(in cx.bump, "{}", y).into_bump_str();
                            [
                                circle(cx.bump)
                                    .attributes([
                                        attr("r", "0.5"),
                                        attr("cx", x),
                                        attr("cy", y),
                                        attr("class", "node hoverable revert"),
                                    ])
                                    .on(
                                        "mouseover",
                                        handler(move |_| {
                                            crate::Msg::Level(crate::level::Msg::RevertPreview(
                                                CaseId(id),
                                            ))
                                        }),
                                    )
                                    .on(
                                        "click",
                                        handler(move |_| {
                                            crate::Msg::Level(crate::level::Msg::RevertTo(CaseId(
                                                id,
                                            )))
                                        }),
                                    )
                                    .finish(),
                                text_(cx.bump)
                                    .attributes([
                                        attr("text-anchor", "middle"),
                                        attr("dominant-baseline", "middle"),
                                        attr("pointer-events", "none"),
                                        attr("x", x),
                                        attr("y", y),
                                    ])
                                    .children([text("⎌")])
                                    .finish(),
                            ]
                        }),
                        cx.bump,
                    ))
                    .finish(),
            )
        }

        svg.attributes([
            attr("id", "class-tree"),
            attr(
                "class",
                if axiom {
                    "background axiom"
                } else if self.all_complete() {
                    "background complete"
                } else {
                    "background"
                },
            ),
            attr("preserveAspectRatio", "xMidYMax meet"),
            attr("font-size", "0.75"),
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
