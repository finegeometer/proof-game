use super::*;
use crate::render::{bezier, g, handler};
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

        let mut subtrees = bumpalo::collections::Vec::new_in(cx.bump);

        for &node in children {
            let (subtree, x) = self.subtree(cx, node, x, y - 2., y_min);
            xs.push(x);
            subtrees.push(subtree);
        }

        let mut builder = g(cx.bump);

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
                circle = circle.on(
                    "click",
                    handler(move |_| crate::Msg::GotoCase(CaseId(node))),
                )
            }

            let n = subtrees.len();
            subtrees.push(circle.finish());
            subtrees.swap(0, n);

            (
                g(cx.bump).children(subtrees).finish(),
                std::mem::replace(x, *x + 2.),
            )
        } else {
            let x0 = xs.iter().copied().sum::<f64>() / (xs.len() as f64);

            let mut d = bumpalo::collections::String::new_in(cx.bump);
            for x in xs {
                bezier::path([x0, y], [0., -0.5], [0., -0.5], [x, y - 2.], &mut d)
            }
            let d = d.into_bump_str();

            let n = subtrees.len();
            subtrees.push(
                path(cx.bump)
                    .attributes([attr("class", "wire border"), attr("d", d)])
                    .finish(),
            );
            subtrees.swap(0, n);

            let n = subtrees.len();
            subtrees.push(
                path(cx.bump)
                    .attributes([attr("class", "wire"), attr("d", d)])
                    .finish(),
            );
            subtrees.swap(1, n);

            (g(cx.bump).children(subtrees).finish(), x0)
        }
    }

    pub fn render<'a>(&self, cx: &mut dodrio::RenderContext<'a>) -> dodrio::Node<'a> {
        let mut x = 0.;
        let mut y_min = 0.;

        svg(cx.bump)
            .child(self.subtree(cx, 0, &mut x, 0., &mut y_min).0)
            .attributes([
                attr("id", "class-tree"),
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
