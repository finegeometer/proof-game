mod render;

use crate::render::PanZoom;

pub struct State {
    pan_zoom: PanZoom,
    drag: Option<[f64; 2]>,
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Msg {
    MouseDown(f64, f64),
    MouseMove(f64, f64),
    MouseUp(f64, f64),
    MouseWheel(f64, f64, f64),
}

impl State {
    pub fn new(pan_zoom: PanZoom) -> Self {
        Self {
            pan_zoom,
            drag: None,
        }
    }

    pub fn update(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::MouseDown(x, y) => {
                if self.drag.is_some() {
                    return false;
                }

                self.drag = Some([x, y]);
                true
            }
            Msg::MouseMove(x, y) => self.mouse_move(x, y),
            Msg::MouseUp(x, y) => {
                let rerender = self.mouse_move(x, y);
                self.drag = None;
                rerender
            }
            Msg::MouseWheel(x, y, wheel) => {
                self.pan_zoom.zoom(x, y, (wheel * 0.001).exp());
                true
            }
        }
    }

    fn mouse_move(&mut self, x: f64, y: f64) -> bool {
        let Some(coord) = &mut self.drag else {return false};

        let dx = x - coord[0];
        let dy = y - coord[1];

        coord[0] = x;
        coord[1] = y;

        self.pan_zoom.pan(dx, dy);

        // Update coord in response to changing coordinate system.
        coord[0] -= dx;
        coord[1] -= dy;

        true
    }
}
