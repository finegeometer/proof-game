pub mod bezier;

use dodrio::{builder::ElementBuilder, bumpalo};
use wasm_bindgen::JsCast;

pub fn g(
    bump: &bumpalo::Bump,
) -> ElementBuilder<
    bumpalo::collections::Vec<dodrio::Listener>,
    bumpalo::collections::Vec<dodrio::Attribute>,
    bumpalo::collections::Vec<dodrio::Node>,
> {
    let builder = ElementBuilder::new(bump, "g");
    builder.namespace(Some("http://www.w3.org/2000/svg"))
}

pub fn text_(
    bump: &bumpalo::Bump,
) -> ElementBuilder<
    bumpalo::collections::Vec<dodrio::Listener>,
    bumpalo::collections::Vec<dodrio::Attribute>,
    bumpalo::collections::Vec<dodrio::Node>,
> {
    let builder = ElementBuilder::new(bump, "text");
    builder.namespace(Some("http://www.w3.org/2000/svg"))
}

// https://stackoverflow.com/a/42711775
pub fn to_svg_coords(e: web_sys::MouseEvent, id: &str) -> (f64, f64) {
    let svg: web_sys::SvgsvgElement = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap();

    let pt: web_sys::SvgPoint = svg.create_svg_point();
    pt.set_x(e.client_x() as f32);
    pt.set_y(e.client_y() as f32);
    let out = pt.matrix_transform(&svg.get_screen_ctm().unwrap().inverse().unwrap());
    (out.x() as f64, out.y() as f64)
}

pub(crate) fn handler(
    msg: impl 'static + Fn(web_sys::Event) -> crate::Msg,
) -> impl 'static + Fn(&mut dyn dodrio::RootRender, dodrio::VdomWeak, web_sys::Event) {
    move |root, _, e| {
        e.stop_propagation();
        e.prevent_default();
        root.unwrap_mut::<super::Model>()
            .send_msg
            .send_blocking(msg(e))
            .unwrap();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PanZoom {
    pub svg_corners: ([f64; 2], [f64; 2]),
}

impl PanZoom {
    pub fn center([x, y]: [f64; 2], r: f64) -> Self {
        Self {
            svg_corners: ([x - r, y - r], [x + r, y + r]),
        }
    }
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.svg_corners.0[0] -= dx;
        self.svg_corners.1[0] -= dx;
        self.svg_corners.0[1] -= dy;
        self.svg_corners.1[1] -= dy;
    }

    pub fn zoom(&mut self, x: f64, y: f64, scale_factor: f64) {
        self.svg_corners.0[0] = (self.svg_corners.0[0] - x) * scale_factor + x;
        self.svg_corners.1[0] = (self.svg_corners.1[0] - x) * scale_factor + x;
        self.svg_corners.0[1] = (self.svg_corners.0[1] - y) * scale_factor + y;
        self.svg_corners.1[1] = (self.svg_corners.1[1] - y) * scale_factor + y;
    }

    pub fn viewbox<'bump>(&self, bump: &'bump bumpalo::Bump) -> dodrio::Attribute<'bump> {
        dodrio::builder::attr(
            "viewBox",
            bumpalo::format!(in bump,
                "{} {} {} {}",
                self.svg_corners.0[0],
                self.svg_corners.0[1],
                self.svg_corners.1[0] - self.svg_corners.0[0],
                self.svg_corners.1[1] - self.svg_corners.0[1]
            )
            .into_bump_str(),
        )
    }
}
