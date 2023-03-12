use std::collections::HashMap;

use wasm_bindgen::{prelude::Closure, JsCast};

use super::bezier::connect_bezier;

const SVG_NAMESPACE: Option<&str> = Some("http://www.w3.org/2000/svg");
const HTML_NAMESPACE: Option<&str> = Some("http://www.w3.org/1999/xhtml");

pub struct State {
    document: web_sys::Document,
    svg: web_sys::SvgsvgElement,
    left_button: web_sys::Element,
    right_button: web_sys::Element,

    svg_x: (f64, f64),
    svg_y: (f64, f64),

    nodes: HashMap<super::super::Node, Node>,
    wires: HashMap<super::super::Wire, Wire>,

    next_level_button: web_sys::Element,
    remaining_pages_text: web_sys::Element,
}

impl State {
    /// The newly initialized state is not fully ready until load_page has been called.
    // Thanks to https://stackoverflow.com/a/5644436 for styling help.
    pub fn new() -> Self {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        let svg = document
            .create_element_ns(SVG_NAMESPACE, "svg")
            .unwrap()
            .dyn_into::<web_sys::SvgsvgElement>()
            .unwrap();
        svg.set_attribute_ns(None, "preserveAspectRatio", "xMinYMin slice")
            .unwrap();
        svg.set_attribute_ns(None, "font-size", "0.75").unwrap();
        body.append_child(&svg).unwrap();

        svg.set_attribute_ns(None, "style", "top: 2%; height: 96%; left: 9%; width: 82%;")
            .unwrap();

        let closure: Closure<dyn Fn(web_sys::MouseEvent)> = {
            let svg = svg.clone();
            Closure::new(move |e: web_sys::MouseEvent| {
                let (x, y) = to_svg_coords(e, &svg);
                crate::handle_msg(crate::Msg::MouseDown(x, y, crate::DragObject::Background))
            })
        };
        svg.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let closure: Closure<dyn Fn(web_sys::MouseEvent)> = {
            let svg = svg.clone();
            Closure::new(move |e: web_sys::MouseEvent| {
                let (x, y) = to_svg_coords(e, &svg);
                crate::handle_msg(crate::Msg::MouseMove(x, y))
            })
        };
        svg.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let closure: Closure<dyn Fn(web_sys::MouseEvent)> = {
            let svg = svg.clone();
            Closure::new(move |e: web_sys::MouseEvent| {
                let (x, y) = to_svg_coords(e, &svg);
                crate::handle_msg(crate::Msg::MouseUp(x, y))
            })
        };
        svg.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let closure: Closure<dyn Fn(web_sys::WheelEvent)> = {
            let svg = svg.clone();
            Closure::new(move |e: web_sys::WheelEvent| {
                let wheel = e.delta_y();
                let (x, y) = to_svg_coords(e.into(), &svg);
                crate::handle_msg(crate::Msg::MouseWheel(x, y, wheel))
            })
        };
        svg.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let left_button = document.create_element_ns(HTML_NAMESPACE, "div").unwrap();
        body.append_child(&left_button).unwrap();

        left_button
            .set_attribute_ns(None, "style", "top: 2%; height: 96%; left: 2%; width: 5%;")
            .unwrap();

        let closure: Closure<dyn Fn()> =
            Closure::new(move || crate::handle_msg(crate::Msg::PrevPage));
        left_button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let right_button = document.create_element_ns(HTML_NAMESPACE, "div").unwrap();
        body.append_child(&right_button).unwrap();

        right_button
            .set_attribute_ns(None, "style", "top: 2%; height: 96%; left: 93%; width: 5%;")
            .unwrap();

        let closure: Closure<dyn Fn()> =
            Closure::new(move || crate::handle_msg(crate::Msg::NextPage));
        right_button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let reset_button = document.create_element_ns(HTML_NAMESPACE, "div").unwrap();
        reset_button
            .set_attribute_ns(None, "class", "resetButton button")
            .unwrap();
        reset_button
            .set_attribute_ns(
                None,
                "style",
                "top: 88%; height: 10%; left: 81%; width: 10%;",
            )
            .unwrap();
        body.append_child(&reset_button).unwrap();

        reset_button.set_text_content(Some("Reset"));

        let closure: Closure<dyn Fn()> =
            Closure::new(move || crate::handle_msg(crate::Msg::ResetLevel));
        reset_button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let next_level_button = document.create_element_ns(HTML_NAMESPACE, "div").unwrap();
        next_level_button
            .set_attribute_ns(None, "class", "nextLevel button")
            .unwrap();
        next_level_button.set_text_content(Some("Next Level"));

        let closure: Closure<dyn Fn()> = {
            let next_level_button = next_level_button.clone();
            Closure::new(move || {
                body.remove_child(&next_level_button).unwrap();
                crate::handle_msg(crate::Msg::NextLevel)
            })
        };
        next_level_button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let remaining_pages_text = document.create_element_ns(SVG_NAMESPACE, "text").unwrap();
        remaining_pages_text
            .set_attribute_ns(None, "text-anchor", "middle")
            .unwrap();
        remaining_pages_text
            .set_attribute_ns(None, "dominant-baseline", "middle")
            .unwrap();
        remaining_pages_text
            .set_attribute_ns(None, "pointer-events", "none")
            .unwrap();

        let out = Self {
            document,
            svg,
            left_button,
            right_button,
            svg_x: (-1., 19.), // NOTE: If I update this, I need to update the viewBox.
            svg_y: (-1., 19.), // NOTE: If I update this, I need to update the viewBox.
            nodes: HashMap::new(),
            wires: HashMap::new(),
            next_level_button,
            remaining_pages_text,
        };
        out.update_view_box();
        out
    }

    pub fn load_page(state: &mut super::super::State) {
        {
            let n = state.pages_left();
            state
                .render
                .left_button
                .set_attribute_ns(
                    None,
                    "class",
                    if n == 0 {
                        "background disabled button"
                    } else {
                        "background hoverable button"
                    },
                )
                .unwrap();

            state.render.left_button.set_text_content(Some("◀"));
        }
        {
            let n = state.pages_right();
            state
                .render
                .right_button
                .set_attribute_ns(
                    None,
                    "class",
                    if n == 0 {
                        "background disabled button"
                    } else {
                        "background hoverable button"
                    },
                )
                .unwrap();
            state.render.right_button.set_text_content(Some("▶"));
        }

        while let Some(last) = state.render.svg.last_child() {
            state.render.svg.remove_child(&last).unwrap();
        }
        state.render.wires.clear();
        state.render.nodes.clear();

        if let Some(page) = &state.page {
            state
                .render
                .svg
                .set_attribute_ns(None, "class", "background")
                .unwrap();

            let wire_group: web_sys::Element = state
                .render
                .document
                .create_element_ns(SVG_NAMESPACE, "g")
                .unwrap();
            let node_group: web_sys::Element = state
                .render
                .document
                .create_element_ns(SVG_NAMESPACE, "g")
                .unwrap();
            state.render.svg.append_child(&wire_group).unwrap();
            state.render.svg.append_child(&node_group).unwrap();

            for wire in page.visible_wires() {
                let wire_svg = Wire::new(
                    wire,
                    &wire_group,
                    match page.wire_status(wire) {
                        Some(()) => " known",
                        None => {
                            if wire == page.goal {
                                " goal"
                            } else {
                                ""
                            }
                        }
                    },
                    page.wire_has_interaction(wire),
                    &state.render,
                );
                wire_svg.set_beziers(page);
                state.render.wires.insert(wire, wire_svg);

                for node in wire.inputs(page) {
                    let node_svg = Node::new(
                        node,
                        &node_group,
                        page.node_has_interaction(node),
                        &state.render,
                    );
                    node_svg.set_position(page);
                    state.render.nodes.insert(node, node_svg);
                }
            }
        } else {
            state
                .render
                .svg
                .set_attribute_ns(None, "class", "background disabled")
                .unwrap();
        }

        let remaining_pages = state.pages_left() + state.pages_right();

        #[allow(clippy::collapsible_else_if)]
        if state.page().is_none() && remaining_pages == 0 {
            if state.render.next_level_button.parent_node().is_none() {
                state
                    .render
                    .document
                    .body()
                    .unwrap()
                    .append_child(&state.render.next_level_button)
                    .unwrap();
            }
        } else {
            if state.render.next_level_button.parent_node().is_some() {
                state
                    .render
                    .document
                    .body()
                    .unwrap()
                    .remove_child(&state.render.next_level_button)
                    .unwrap();
            }
        }

        #[allow(clippy::collapsible_else_if)]
        if state.page().is_none() && remaining_pages != 0 {
            if state.render.remaining_pages_text.parent_node().is_none() {
                state
                    .render
                    .svg
                    .append_child(&state.render.remaining_pages_text)
                    .unwrap();
                if remaining_pages == 1 {
                    state
                        .render
                        .remaining_pages_text
                        .set_text_content(Some("1 page remains."))
                } else {
                    state
                        .render
                        .remaining_pages_text
                        .set_text_content(Some(&format!("{remaining_pages} pages remain.")))
                };
            }
        } else {
            if state.render.remaining_pages_text.parent_node().is_some() {
                state
                    .render
                    .svg
                    .remove_child(&state.render.remaining_pages_text)
                    .unwrap();
            }
        }
    }

    pub fn scroll_background(&mut self, dx: f64, dy: f64) {
        self.svg_x.0 -= dx;
        self.svg_x.1 -= dx;
        self.svg_y.0 -= dy;
        self.svg_y.1 -= dy;
        self.update_view_box();
    }

    pub fn zoom_background(&mut self, x: f64, y: f64, scale_factor: f64) {
        self.svg_x.0 = (self.svg_x.0 - x) * scale_factor + x;
        self.svg_x.1 = (self.svg_x.1 - x) * scale_factor + x;
        self.svg_y.0 = (self.svg_y.0 - y) * scale_factor + y;
        self.svg_y.1 = (self.svg_y.1 - y) * scale_factor + y;
        self.update_view_box();
    }

    fn update_view_box(&self) {
        self.svg
            .set_attribute_ns(
                None,
                "viewBox",
                &format!(
                    "{} {} {} {}",
                    self.svg_x.0,
                    self.svg_y.0,
                    self.svg_x.1 - self.svg_x.0,
                    self.svg_y.1 - self.svg_y.0,
                ),
            )
            .unwrap();
        self.remaining_pages_text
            .set_attribute_ns(
                None,
                "x",
                &format!("{}", (self.svg_x.0 + self.svg_x.1) / 2.),
            )
            .unwrap();
    }

    pub fn set_node_position(&self, node: super::super::Node, page: &super::super::Page) {
        self.nodes.get(&node).unwrap().set_position(page);
        self.wires
            .get(&node.output(page))
            .unwrap()
            .set_beziers(page);
        for wire in node.data().1 {
            self.wires.get(&wire).unwrap().set_beziers(page);
        }
    }
}

// Note: If I ever have a need to delete these, explicitly implement a `delete` function.
// This can be skipped on page reload, because everything's deleted automatically.
struct Node {
    node: super::super::Node,
    circle: web_sys::Element,
    text: web_sys::Element,
}

impl Node {
    fn new(
        node: super::super::Node,
        parent: &web_sys::Element,
        hoverable: bool,
        state: &State,
    ) -> Self {
        let circle: web_sys::Element = state
            .document
            .create_element_ns(SVG_NAMESPACE, "circle")
            .unwrap();
        circle.set_attribute_ns(None, "r", "0.5").unwrap();
        circle
            .set_attribute_ns(
                None,
                "class",
                if hoverable { "node hoverable" } else { "node" },
            )
            .unwrap();
        parent.append_child(&circle).unwrap();

        let closure: Closure<dyn Fn(web_sys::MouseEvent)> = {
            let svg = state.svg.clone();
            Closure::new(move |e: web_sys::MouseEvent| {
                let (x, y) = to_svg_coords(e, &svg);
                crate::handle_msg(crate::Msg::MouseDown(x, y, crate::DragObject::Node(node)))
            })
        };
        circle
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let text: web_sys::Element = state
            .document
            .create_element_ns(SVG_NAMESPACE, "text")
            .unwrap();
        text.set_attribute_ns(None, "text-anchor", "middle")
            .unwrap();
        text.set_attribute_ns(None, "dominant-baseline", "middle")
            .unwrap();
        text.set_attribute_ns(None, "pointer-events", "none")
            .unwrap();
        parent.append_child(&text).unwrap();

        text.set_text_content(Some(&format!("{}", &node.data().0)));

        Self { node, circle, text }
    }

    fn set_position(&self, page: &super::super::Page) {
        let (x, y) = self.node.position(page);
        self.circle
            .set_attribute_ns(None, "cx", &format!("{x}"))
            .unwrap();
        self.circle
            .set_attribute_ns(None, "cy", &format!("{y}"))
            .unwrap();
        self.text
            .set_attribute_ns(None, "x", &format!("{x}"))
            .unwrap();
        self.text
            .set_attribute_ns(None, "y", &format!("{y}"))
            .unwrap();
    }
}

struct Wire {
    wire: super::super::Wire,
    bg: web_sys::Element,
    fg: web_sys::Element,
}

impl Wire {
    fn new(
        wire: super::super::Wire,
        parent: &web_sys::Element,
        extra_classes: &str,
        hoverable: bool,
        state: &State,
    ) -> Self {
        let closure: Closure<dyn Fn(web_sys::MouseEvent)> = {
            let svg = state.svg.clone();
            Closure::new(move |e: web_sys::MouseEvent| {
                let (x, y) = to_svg_coords(e, &svg);
                crate::handle_msg(crate::Msg::MouseDown(x, y, crate::DragObject::Wire(wire)))
            })
        };

        let bg: web_sys::Element = state
            .document
            .create_element_ns(SVG_NAMESPACE, "path")
            .unwrap();
        bg.set_attribute_ns(None, "class", &format!("wire border{extra_classes}"))
            .unwrap();
        parent.append_child(&bg).unwrap();

        bg.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();

        let fg: web_sys::Element = state
            .document
            .create_element_ns(SVG_NAMESPACE, "path")
            .unwrap();
        fg.set_attribute_ns(
            None,
            "class",
            &format!(
                "wire{extra_classes}{}",
                if hoverable { " hoverable" } else { "" }
            ),
        )
        .unwrap();
        parent.append_child(&fg).unwrap();

        fg.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();

        closure.forget();

        Self { wire, bg, fg }
    }

    fn set_beziers(&self, page: &super::super::Page) {
        const WIRE_STIFFNESS: f64 = 0.75;

        let (ab_x, ab_y): (Vec<_>, Vec<_>) = self
            .wire
            .inputs(page)
            .into_iter()
            .map(|node| {
                let (x, y) = node.position(page);
                ((x, x), (y, y + WIRE_STIFFNESS))
            })
            .unzip();
        let outputs = self.wire.outputs(page);
        let (fg_x, fg_y): (Vec<_>, Vec<_>) = if outputs.is_empty() {
            let x_avg = ab_x.iter().map(|&(a, _)| a).sum::<f64>() / (ab_x.len() as f64);
            let y_avg = ab_y.iter().map(|&(a, _)| a).sum::<f64>() / (ab_y.len() as f64);
            (
                vec![(x_avg, x_avg)],
                vec![(y_avg + 2. * WIRE_STIFFNESS, y_avg + 3. * WIRE_STIFFNESS)],
            )
        } else {
            outputs
                .iter()
                .copied()
                .map(|(node, idx)| {
                    let (x, y) = node.position(page);
                    (
                        (
                            x + WIRE_STIFFNESS
                                * (f64::from(idx) - (node.data().1.count() as f64 - 1.) / 2.),
                            x,
                        ),
                        (y - WIRE_STIFFNESS, y),
                    )
                })
                .unzip()
        };

        let (c_x, d_x, e_x) = connect_bezier(ab_x.iter().copied(), fg_x.iter().copied());
        let (c_y, d_y, e_y) = connect_bezier(ab_y.iter().copied(), fg_y.iter().copied());

        use std::fmt::Write;
        let mut path = String::new();
        for ((a_x, b_x), (a_y, b_y)) in ab_x.into_iter().zip(ab_y) {
            write!(
                path,
                "M {a_x} {a_y} C {b_x} {b_y}, {c_x} {c_y}, {d_x} {d_y}"
            )
            .unwrap();
        }
        for ((f_x, g_x), (f_y, g_y)) in fg_x.into_iter().zip(fg_y) {
            write!(
                path,
                "M {d_x} {d_y} C {e_x} {e_y}, {f_x} {f_y}, {g_x} {g_y}"
            )
            .unwrap();
        }
        self.bg.set_attribute_ns(None, "d", &path).unwrap();
        self.fg.set_attribute_ns(None, "d", &path).unwrap();
    }
}

// https://stackoverflow.com/a/42711775
fn to_svg_coords(e: web_sys::MouseEvent, svg: &web_sys::SvgsvgElement) -> (f64, f64) {
    let pt: web_sys::SvgPoint = svg.create_svg_point();
    pt.set_x(e.client_x() as f32);
    pt.set_y(e.client_y() as f32);
    let out = pt.matrix_transform(&svg.get_screen_ctm().unwrap().inverse().unwrap());
    (out.x() as f64, out.y() as f64)
}
