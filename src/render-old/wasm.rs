use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::*;

const SVG_NAMESPACE: Option<&str> = Some("http://www.w3.org/2000/svg");
const HTML_NAMESPACE: Option<&str> = Some("http://www.w3.org/1999/xhtml");

fn document() -> Document {
    window().unwrap().document().unwrap()
}

fn create_svg_elt(ty: &str) -> Element {
    document().create_element_ns(SVG_NAMESPACE, ty).unwrap()
}

fn attr(elt: &Element, a: &str, b: &str) {
    elt.set_attribute_ns(None, a, b).unwrap();
}

fn add_event_listener<F>(elt: &Element, evt: &str, closure: &Closure<F>) {
    elt.add_event_listener_with_callback(evt, closure.as_ref().unchecked_ref())
        .unwrap();
}

// https://stackoverflow.com/a/42711775
fn to_svg_coords(e: MouseEvent, svg: &SvgsvgElement) -> [f64; 2] {
    let pt: SvgPoint = svg.create_svg_point();
    pt.set_x(e.client_x() as f32);
    pt.set_y(e.client_y() as f32);
    let out = pt.matrix_transform(&svg.get_screen_ctm().unwrap().inverse().unwrap());
    [out.x() as f64, out.y() as f64]
}

/// A circle of diameter one, with text inside it.
#[derive(Clone)]
pub struct TextCircle {
    circle: Element,
    text: Element,
}

impl TextCircle {
    pub fn new(str: &str) -> Self {
        let circle = create_svg_elt("circle");
        attr(&circle, "r", "0.5");

        let text = create_svg_elt("text");
        attr(&text, "text-anchor", "middle");
        attr(&text, "dominant-baseline", "middle");
        attr(&text, "pointer-events", "none");

        text.set_text_content(Some(str));

        Self { circle, text }
    }

    fn attach(&self, parent: &Element) {
        parent.append_child(&self.circle).unwrap();
        parent.append_child(&self.text).unwrap();
    }

    fn class(&self, class: &str) {
        attr(&self.circle, "class", &format!("node {class}"));
    }

    fn event_listener<F: wasm_bindgen::closure::WasmClosure>(
        &self,
        evt: &str,
        closure: Closure<F>,
    ) {
        add_event_listener(&self.circle, evt, &closure);
        closure.forget();
    }

    pub fn reposition(&self, [x, y]: [f64; 2]) {
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

/// A path from a collection of inputs to a collection of outputs, connected by bezier curves.
#[derive(Clone)]
pub struct Path {
    bg: Element,
    fg: Element,
}

impl Path {
    pub fn new() -> Self {
        Self {
            bg: create_svg_elt("path"),
            fg: create_svg_elt("path"),
        }
    }

    fn attach(&self, parent: &Element) {
        parent.append_child(&self.bg).unwrap();
        parent.append_child(&self.fg).unwrap();
    }

    pub fn class(&self, class: &str) {
        attr(&self.bg, "class", &format!("wire border {class}"));
        attr(&self.fg, "class", &format!("wire {class}"));
    }

    fn event_listener<F: wasm_bindgen::closure::WasmClosure>(
        &self,
        evt: &str,
        closure: Closure<F>,
    ) {
        add_event_listener(&self.bg, evt, &closure);
        add_event_listener(&self.fg, evt, &closure);
        closure.forget();
    }

    /// Requires: `inputs` nonempty.
    pub fn reposition(&self, inputs: &[([f64; 2], [f64; 2])], outputs: &[([f64; 2], [f64; 2])]) {
        let [mut ax, mut ay, mut bx, mut by, mut fx, mut fy, mut gx, mut gy] = [0.; 8];

        let mut input_len = 0;
        for i in inputs.iter() {
            ax += i.0[0];
            ay += i.0[1];
            bx += i.1[0];
            by += i.1[1];
            input_len += 1;
        }
        let input_len = input_len as f64;
        ax /= input_len;
        ay /= input_len;
        bx /= input_len;
        by /= input_len;

        let mut output_len = 0;
        for o in outputs.iter() {
            fx += o.0[0];
            fy += o.0[1];
            gx += o.1[0];
            gy += o.1[1];
            output_len += 1;
        }
        if output_len == 0 {
            fx = 2. * bx - ax;
            fy = 2. * by - ay;
            gx = 3. * bx - 2. * ax;
            gy = 3. * by - 2. * ay;
        } else {
            let output_len = output_len as f64;
            fx /= output_len;
            fy /= output_len;
            gx /= output_len;
            gy /= output_len;
        }

        // Continuity of first derivative implies `E-D = D-C`.
        // Minimum discontinuity of second derivative further implies `E-D = D-C = (F-B) / 4`.
        // Minimum discontinuity of third derivative further implies `D = ((3B-A) + (3F-G)) / 4`.

        let dx = (3. * bx - ax + 3. * fx - gx) / 4.;
        let dy = (3. * by - ay + 3. * fy - gy) / 4.;
        let vx = (fx - bx) / 4.;
        let vy = (fy - by) / 4.;
        let cx = dx - vx;
        let cy = dy - vy;
        let ex = dx + vx;
        let ey = dy + vy;

        use std::fmt::Write;
        let mut path = String::new();
        for ([ax, ay], [bx, by]) in inputs {
            write!(path, "M {ax} {ay} C {bx} {by}, {cx} {cy}, {dx} {dy}").unwrap();
        }
        for ([fx, fy], [gx, gy]) in inputs {
            write!(path, "M {dx} {dy} C {ex} {ey}, {fx} {fy}, {gx} {gy}").unwrap();
        }
        attr(&self.bg, "d", &path);
        attr(&self.fg, "d", &path);
    }
}
