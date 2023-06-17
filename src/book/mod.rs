use crate::render::*;
use dodrio::{builder::*, bumpalo, Cached, Render};

pub struct Book {
    pages: Vec<Cached<Page>>,
    current_page: usize,
}

impl Book {
    pub fn new() -> Self {
        Self {
            pages: [
                Op(Conjunction),
                Op(Disjunction),
                Op(Implication),
                Op(Equality),
            ]
            .into_iter()
            .map(Cached::new)
            .collect(),
            current_page: 0,
        }
    }
    pub fn update(&mut self, Msg::GotoPage(page): Msg) {
        self.current_page = page;

        // Note: This is technically scrolling the old page.
        // But I created this code in response to the VDOM library reusing the scroll position,
        // so this is an effective solution.
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("book-content")
            .unwrap()
            .set_scroll_top(0);
    }
}

#[derive(Debug)]
pub enum Msg {
    GotoPage(usize),
}

impl<'a> Render<'a> for Book {
    fn render(&self, cx: &mut dodrio::RenderContext<'a>) -> dodrio::Node<'a> {
        let current_page = self.current_page;

        div(cx.bump)
            .attributes([attr("id", "book"), attr("class", "background")])
            .children([
                if current_page > 0 {
                    div(cx.bump)
                        .attributes([attr("class", "button")])
                        .children([text("◄")])
                        .on(
                            "click",
                            handler(move |_| crate::Msg::Book(Msg::GotoPage(current_page - 1))),
                        )
                        .finish()
                } else {
                    div(cx.bump)
                        .attributes([attr("class", "button disabled")])
                        .children([text("◄")])
                        .finish()
                },
                self.pages[current_page].render(cx),
                if current_page < self.pages.len() - 1 {
                    div(cx.bump)
                        .attributes([attr("class", "button")])
                        .children([text("►")])
                        .on(
                            "click",
                            handler(move |_| crate::Msg::Book(Msg::GotoPage(current_page + 1))),
                        )
                        .finish()
                } else {
                    div(cx.bump)
                        .attributes([attr("class", "button disabled")])
                        .children([text("►")])
                        .finish()
                },
            ])
            .finish()
    }
}

enum Page {
    Op(OpPage),
}

impl Default for Page {
    fn default() -> Self {
        Op(Conjunction)
    }
}
use Page::*;

enum OpPage {
    Conjunction,
    Disjunction,
    Implication,
    Equality,
}
use OpPage::*;

impl<'a> Render<'a> for Page {
    fn render(&self, cx: &mut dodrio::RenderContext<'a>) -> dodrio::Node<'a> {
        let mut builder = div(cx.bump)
            .attributes([attr("id", "book-content")])
            .child(h1(cx.bump).children([text(self.title())]).finish())
            .child(markup(cx, self.description()))
            .child(match self {
                Op(page) => div(cx.bump)
                    .attributes([attr("class", "book-rules")])
                    .children([
                        div(cx.bump)
                            .attributes([attr("class", "book-rule")])
                            .children([
                                markup(cx, page.rule_description(RuleKind::Intro)),
                                page.rule_picture(cx, RuleKind::Intro),
                            ])
                            .finish(),
                        div(cx.bump)
                            .attributes([attr("class", "book-rule")])
                            .children([
                                markup(cx, page.rule_description(RuleKind::Elim)),
                                page.rule_picture(cx, RuleKind::Elim),
                            ])
                            .finish(),
                    ])
                    .finish(),
            });
        for paragraph in self.notes() {
            builder = builder.child(markup(cx, paragraph));
        }
        builder.finish()
    }
}

#[derive(Clone, Copy)]
enum RuleKind {
    Intro,
    Elim,
}

impl Page {
    #[rustfmt::skip]
    fn title(&self) -> &'static str {
        match self {
            Op(Conjunction) => "Conjunction",
            Op(Disjunction) => "Disjunction",
            Op(Implication) => "Implication",
            Op(Equality)    => "Equality"   ,
        }
    }

    #[rustfmt::skip]
    fn description(&self) -> &'static str {
        match self {
            Op(Conjunction) => "The conjunction *a ∧ b* is read \"*a* and *b*\". As that suggests, *a ∧ b* is the statement that *a* and *b* are both true.",
            Op(Disjunction) => "The disjunction *a ∨ b* is read \"*a* or *b*\". It's true whenever either *a* is, or *b* is, or both.",
            Op(Implication) => "The implication *a ⇒ b* is read \"*a* implies *b*\". It says that *if* you knew *a*, you would know *b* as well.",
            Op(Equality) => "The equality *a = b* is read \"*a* equals *b*\". It says that *a* and *b* represent the same thing.",
        }
    }

    fn notes(&self) -> &'static [&'static str] {
        match self {
            Op(Conjunction) => &[
                "More generally, you can take a conjunction of three propositions, or seven, or umpteen million. Or even zero!",
                "For the conjunction to hold, all of the individual propositions must be true. In the \"zero propositions\" case, that's trivial, so the conjunction is always true."
            ],
            Op(Disjunction) => &[
                "Just like conjunctions, you can take a disjunction of any number of propositions.",
                "For the disjunction to hold, at least one of the individual propositions must be true. In the \"zero propositions\" case, that can't happen, so the disjunction is always false.",
                "Note: When you use a disjunction, you end up with one case per individual proposition. For a disjunction of zero propositions, that means there's nothing left to do! This is the *principle of explosion*; if you can prove *False*, you can prove anything."
            ],
            Op(Implication) => &[],
            Op(Equality) => &[],
        }
    }
}

impl OpPage {
    fn rule_description(&self, rule_kind: RuleKind) -> &'static str {
        match (self,rule_kind) {
            (Conjunction, RuleKind::Intro) => "If you know *a*, and you know *b*, you can prove *a ∧ b*.",
            (Conjunction, RuleKind::Elim) => "If you know *a ∧ b*, you can prove *a* and *b*.",
            (Disjunction, RuleKind::Intro) => "If you know *a*, you can prove *a ∨ b*. Same if you know *b*.",
            (Disjunction, RuleKind::Elim) => "If you know *a ∨ b*, there are two cases. Either *a* holds, or *b* does. Handle both cases to complete the proof!",
            (Implication, RuleKind::Intro) => "To prove *a ⇒ b*, show that you could prove *b* if you knew *a*.",
            (Implication, RuleKind::Elim) => "If you know *a ⇒ b*, and you know *a*, then you can prove *b*.",
            (Equality, RuleKind::Intro) => "You can prove *a = a*.",
            (Equality, RuleKind::Elim) => "If *a = b*, *a* and *b* are interchangable. Anything that's true about one is true about the other.",
        }
    }

    fn rule_picture<'a>(
        &self,
        cx: &mut dodrio::RenderContext<'a>,
        rule_kind: RuleKind,
    ) -> dodrio::Node<'a> {
        match (self, rule_kind) {
            (Conjunction, RuleKind::Intro) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: row; align-items: center;",
                )])
                .children([
                    example(cx, "∧", "known", "known", "", false),
                    arrow(cx.bump),
                    example(cx, "∧", "known", "known", "known", false),
                ])
                .finish(),
            (Conjunction, RuleKind::Elim) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: row; align-items: center;",
                )])
                .children([
                    example(cx, "∧", "", "", "known", false),
                    arrow(cx.bump),
                    example(cx, "∧", "known", "known", "known", false),
                ])
                .finish(),

            (Disjunction, RuleKind::Intro) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: column;",
                )])
                .children([
                    div(cx.bump)
                        .attributes([attr(
                            "style",
                            "display: flex; flex-direction: row; align-items: center; padding-bottom: 1em",
                        )])
                        .children([
                            example(cx, "∨", "known", "", "", false),
                            arrow(cx.bump),
                            example(cx, "∨", "known", "", "known", false),
                        ])
                        .finish(),
                    div(cx.bump)
                        .attributes([attr(
                            "style",
                            "display: flex; flex-direction: row; align-items: center;",
                        )])
                        .children([
                            example(cx, "∨", "", "known", "", false),
                            arrow(cx.bump),
                            example(cx, "∨", "", "known", "known", false),
                        ])
                        .finish(),
                ])
                .finish(),
            (Disjunction, RuleKind::Elim) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: row; align-items: center;",
                )])
                .children([
                    example(cx, "∨", "", "", "known", false),
                    arrow(cx.bump),
                    div(cx.bump)
                        .attributes([attr(
                            "style",
                            "flex: 1; display: flex; flex-direction: column;",
                        )])
                        .children([
                            example(cx, "∨", "known", "", "known", false),
                            example(cx, "∨", "", "known", "known", false),
                        ])
                        .finish(),
                ])
                .finish(),
            (Implication, RuleKind::Intro) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: row; align-items: center;",
                )])
                .children([
                    example(cx, "⇒", "", "", "goal", false),
                    arrow(cx.bump),
                    example(cx, "⇒", "known", "goal", "", false),
                ])
                .finish(),
            (Implication, RuleKind::Elim) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: row; align-items: center;",
                )])
                .children([
                    example(cx, "⇒", "known", "", "known", false),
                    arrow(cx.bump),
                    example(cx, "⇒", "known", "known", "known", false),
                ])
                .finish(),
            (Equality, RuleKind::Intro) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: row; align-items: center;",
                )])
                .children([
                    example(cx, "=", "", "", "", true),
                    arrow(cx.bump),
                    example(cx, "=", "", "", "known", true),
                ])
                .finish(),
            (Equality, RuleKind::Elim) => div(cx.bump)
                .attributes([attr(
                    "style",
                    "display: flex; flex-direction: row; align-items: center;",
                )])
                .children([
                    example(cx, "=", "", "", "known", false),
                    arrow(cx.bump),
                    example(cx, "=", "", "", "known", true),
                ])
                .finish(),
        }
    }
}

// Utilities

fn arrow(bump: &bumpalo::Bump) -> dodrio::Node {
    div(bump)
        .attributes([attr("style", "padding: 1em")])
        .children([text("→")])
        .finish()
}

fn emph<'a>(cx: &mut dodrio::RenderContext<'a>, contents: &'a str) -> dodrio::Node<'a> {
    em(cx.bump).children([text(contents)]).finish()
}

fn markup<'a>(cx: &mut dodrio::RenderContext<'a>, mut contents: &'a str) -> dodrio::Node<'a> {
    let mut builder = p(cx.bump);

    // State machine!
    enum State {
        Text,
        Emph,
    }
    use State::*;
    let mut state = Text;

    loop {
        match state {
            Text => {
                let mut eof = false;
                let byte = contents.find('*').unwrap_or_else(|| {
                    eof = true;
                    contents.len()
                });
                builder = builder.child(text(&contents[0..byte]));
                if eof {
                    break;
                } else {
                    state = Emph;
                    contents = &contents[byte + 1..];
                }
            }
            Emph => {
                let Some(byte) = contents.find('*') else {panic!("Unterminated emphasis marking")};
                state = Text;
                builder = builder.child(emph(cx, &contents[0..byte]));
                contents = &contents[byte + 1..]
            }
        }
    }

    builder.finish()
}

#[rustfmt::skip]
fn example<'a>(
    cx: &mut dodrio::RenderContext<'a>,
    op: &'a str,
    in1_class: &str,
    in2_class: &str,
    out_class: &str,
    merge: bool,
) -> dodrio::Node<'a> {
    let no_merge = if merge {0.} else {1.};

    let mut in1_d = bumpalo::collections::String::new_in(cx.bump);
    let mut in2_d = bumpalo::collections::String::new_in(cx.bump);
    let mut out_d = bumpalo::collections::String::new_in(cx.bump);
    bezier::path([-no_merge, -2.], [0.,1.], [ 0.5, 1.], [0., 0.], &mut in1_d);
    bezier::path([ no_merge, -2.], [0.,1.], [-0.5, 1.], [0., 0.], &mut in2_d);
    bezier::path([       0.,  0.], [0.,1.], [ 0. , 1.], [0., 2.], &mut out_d);
    let in1_d = in1_d.into_bump_str();
    let in2_d = in2_d.into_bump_str();
    let out_d = out_d.into_bump_str();

    svg(cx.bump)
        .attributes([
            attr("preserveAspectRatio", "xMidYMid meet"),
            attr("font-size", "0.75"),
            PanZoom { svg_corners: ([-2., -2.], [2., 2.]) }.viewbox(cx.bump),
            attr("class", "background"),
        ])
        .children([
            path(cx.bump).attributes([attr("d", in1_d), attr("class", bumpalo::format!(in cx.bump, "wire border {}", in1_class).into_bump_str())]).finish(),
            path(cx.bump).attributes([attr("d", in2_d), attr("class", bumpalo::format!(in cx.bump, "wire border {}", in2_class).into_bump_str())]).finish(),
            path(cx.bump).attributes([attr("d", out_d), attr("class", bumpalo::format!(in cx.bump, "wire border {}", out_class).into_bump_str())]).finish(),
            path(cx.bump).attributes([attr("d", in1_d), attr("class", bumpalo::format!(in cx.bump, "wire        {}", in1_class).into_bump_str())]).finish(),
            path(cx.bump).attributes([attr("d", in2_d), attr("class", bumpalo::format!(in cx.bump, "wire        {}", in2_class).into_bump_str())]).finish(),
            path(cx.bump).attributes([attr("d", out_d), attr("class", bumpalo::format!(in cx.bump, "wire        {}", out_class).into_bump_str())]).finish(),

            circle(cx.bump)
                .attributes([
                    attr("r", "0.5"),
                    attr("cx", "0"),
                    attr("cy", "0"),
                    attr("class", "node"),
                ])
                .finish(),
            text_(cx.bump)
                .attributes([
                    attr("text-anchor", "middle"),
                    attr("dominant-baseline", "middle"),
                    attr("pointer-events", "none"),
                    attr("x", "0"),
                    attr("y", "0"),
                ])
                .children([text(op)])
                .finish(),
        ])
        .finish()
}
