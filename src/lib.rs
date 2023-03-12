mod expr_graph;
mod interactions;
mod load;

use std::cell::RefCell;

use expr_graph::*;
use wasm_bindgen::prelude::wasm_bindgen;

///////////////////
// WASM-specific //
///////////////////

thread_local! {
    static MODEL : RefCell<Model> = RefCell::new(Model::new());
}

#[wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // `MODEL` is initialized upon first access.
    // This will kick-start the rendering.
    // Everything else is handled by event handlers.
    MODEL.with(|_| {});
}

// An update function, to be called by event handlers.
fn handle_msg(msg: Msg) {
    MODEL.with(|model| model.borrow_mut().update(msg));
}

/////////////
// General //
/////////////

struct Model {
    state: State,
    current_level: load::LevelData,
    future_levels: std::vec::IntoIter<load::LevelData>,
    drag: Option<DragState>,
}

#[derive(Clone, Copy)]
struct DragState {
    coord: (f64, f64),
    /// If this is an `Err`, this "drag" might actually be a click.
    /// In this case, the `Err` stores the initial coordinate that the user clicked.
    /// If the current `coord` moves too far from this, we know that it is in fact a drag.
    confirmed_drag: Result<(), (f64, f64)>,
    object: DragObject,
}
#[derive(Clone, Copy)]
enum DragObject {
    Node(Node),
    Wire(Wire),
    Background,
}

enum Msg {
    MouseDown(f64, f64, DragObject),
    MouseMove(f64, f64),
    MouseUp(f64, f64),
    MouseWheel(f64, f64, f64),
    NextPage,
    PrevPage,
    NextLevel,
    ResetLevel,
}

impl Model {
    fn new() -> Self {
        let levels: Vec<load::LevelData> =
            serde_json::from_str(include_str!("./levels.json")).unwrap();
        let mut future_levels = levels.into_iter();

        let current_level = future_levels.next().unwrap();

        let mut state = State::new();
        state.load_level(current_level.load().unwrap());

        Self {
            state,
            current_level,
            future_levels,
            drag: None,
        }
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::MouseDown(x, y, object) => {
                if self.drag.is_none() {
                    self.drag = Some(DragState {
                        coord: (x, y),
                        confirmed_drag: Err((x, y)),
                        object,
                    });
                }
            }
            Msg::MouseMove(x, y) => self.mouse_move(x, y),
            Msg::MouseUp(x, y) => {
                self.mouse_move(x, y);
                match self.drag {
                    Some(DragState {
                        confirmed_drag: Err(_),
                        object: DragObject::Node(node),
                        ..
                    }) => self.state.interact_node(node),
                    Some(DragState {
                        confirmed_drag: Err(_),
                        object: DragObject::Wire(wire),
                        ..
                    }) => self.state.interact_wire(wire),
                    Some(DragState {
                        confirmed_drag: Err(_),
                        object: DragObject::Background,
                        ..
                    }) => {}
                    Some(DragState {
                        confirmed_drag: Ok(()),
                        ..
                    }) => {}
                    None => {}
                }
                self.drag = None;
            }
            Msg::NextPage => {
                self.state.next_page();
            }
            Msg::PrevPage => {
                self.state.prev_page();
            }
            Msg::MouseWheel(x, y, wheel) => {
                self.state.zoom_background(x, y, (wheel * 0.001).exp());
                if let Some(DragState {
                    coord,
                    confirmed_drag,
                    object: _,
                }) = &mut self.drag
                {
                    // Semantics: We do not count the move from the last known coordinate of the mouse to the zoom coordinate.
                    // We do, however, update the last known coordinate.
                    *coord = (x, y);
                    *confirmed_drag = Ok(());
                }
            }
            Msg::NextLevel => {
                if self.state.page().is_none()
                    && self.state.pages_left() == 0
                    && self.state.pages_right() == 0
                {
                    if let Some(level) = self.future_levels.next() {
                        self.state
                            .load_level(load::LevelData::load(&level).unwrap());
                        self.current_level = level;
                    }
                }
            }
            Msg::ResetLevel => {
                self.drag = None;
                self.state.load_level(self.current_level.load().unwrap());
            }
        }
    }

    fn mouse_move(&mut self, x: f64, y: f64) {
        if let Some(DragState {
            coord,
            confirmed_drag,
            object,
        }) = &mut self.drag
        {
            let dx = x - coord.0;
            let dy = y - coord.1;

            coord.0 = x;
            coord.1 = y;

            if let Err(init_coord) = confirmed_drag {
                if (coord.0 - init_coord.0).powi(2) + (coord.1 - init_coord.1).powi(2) > 0.01 {
                    *confirmed_drag = Ok(());
                }
            }

            if confirmed_drag.is_ok() {
                match object {
                    DragObject::Node(node) => {
                        self.state.set_node_position(*node, x, y);
                    }
                    DragObject::Wire(_) => {}
                    DragObject::Background => {
                        self.state.scroll_background(dx, dy);

                        // Update coord in response to changing coordinate system.
                        coord.0 -= dx;
                        coord.1 -= dy;
                    }
                }
            }
        }
    }
}
