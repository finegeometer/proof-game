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
