use crate::render::PanZoom;

mod render;

pub struct State {
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
    pub fn new() -> Self {
        Self { drag: None }
    }

    pub fn update(&mut self, msg: Msg, panzoom: &mut PanZoom, rerender: &mut bool) {
        match msg {
            Msg::MouseDown(x, y) => {
                if self.drag.is_none() {
                    self.drag = Some([x, y]);
                    *rerender = true;
                }
            }
            Msg::MouseMove(x, y) => self.mouse_move(x, y, panzoom, rerender),
            Msg::MouseUp(x, y) => {
                self.mouse_move(x, y, panzoom, rerender);
                self.drag = None;
            }
            Msg::MouseWheel(x, y, wheel) => {
                panzoom.zoom(x, y, (wheel * 0.001).exp());
                *rerender = true
            }
        }
    }

    fn mouse_move(&mut self, x: f64, y: f64, panzoom: &mut PanZoom, rerender: &mut bool) {
        if let Some(coord) = &mut self.drag {
            let dx = x - coord[0];
            let dy = y - coord[1];

            coord[0] = x;
            coord[1] = y;

            panzoom.pan(dx, dy);

            // Update coord in response to changing coordinate system.
            coord[0] -= dx;
            coord[1] -= dy;

            *rerender = true
        }
    }
}
