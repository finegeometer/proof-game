#![warn(clippy::todo)]
#![allow(clippy::new_without_default)]

mod case;
mod case_tree;
mod expression;
mod load;
mod render;

pub use case::{Case, Node, ValidityReason, Wire};
pub use case_tree::CaseTree;
pub use expression::Expression;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let body = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap();
    dodrio::Vdom::new(&body, Model::new()).forget()
}

struct Model {
    case_tree: CaseTree,
    drag: Option<DragState>,
    current_level: load::LevelData<'static>,
    future_levels: std::vec::IntoIter<load::LevelData<'static>>,
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

#[derive(Debug, Clone, Copy)]
enum DragObject {
    Node(Node),
    Wire(Wire),
    Background,
}

#[derive(Debug)]
enum Msg {
    MouseDown(f64, f64, DragObject),
    MouseMove(f64, f64),
    MouseUp(f64, f64),
    MouseWheel(f64, f64, f64),
    NextCase,
    PrevCase,
    NextLevel,
    ResetLevel,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnlockState {
    None,
    CaseTree,
    Lemmas,
}

impl Model {
    fn unlock_state(&self) -> UnlockState {
        match self.future_levels.as_slice().len() {
            0..=1 => UnlockState::Lemmas,
            2..=11 => UnlockState::CaseTree,
            _ => UnlockState::None,
        }
    }

    fn new() -> Self {
        let levels: Vec<load::LevelData> =
            serde_json::from_str(include_str!("./levels.json")).unwrap();
        let mut future_levels = levels.into_iter();

        let current_level = future_levels.next().unwrap();
        let case_tree = current_level.load().unwrap();

        Self {
            case_tree,
            drag: None,
            current_level,
            future_levels,
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
                    }) => {
                        if self
                            .case_tree
                            .current_case()
                            .as_ref()
                            .and_then(move |x| x.node_has_interaction(node).then_some(()))
                            .is_some()
                        {
                            self.case_tree.interact_node(node)
                        }
                    }
                    Some(DragState {
                        confirmed_drag: Err(_),
                        object: DragObject::Wire(wire),
                        ..
                    }) => {
                        if self.unlock_state() >= UnlockState::Lemmas
                            && self
                                .case_tree
                                .current_case()
                                .as_ref()
                                .and_then(move |x| x.wire_has_interaction(wire).then_some(()))
                                .is_some()
                        {
                            self.case_tree.interact_wire(wire)
                        }
                    }
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
            Msg::NextCase => {
                self.case_tree.next_case();
            }
            Msg::PrevCase => {
                self.case_tree.prev_case();
            }
            Msg::MouseWheel(x, y, wheel) => {
                self.case_tree.zoom_background(x, y, (wheel * 0.001).exp());
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
                if self.case_tree.current_case().is_none()
                    && self.case_tree.cases_left() == 0
                    && self.case_tree.cases_right() == 0
                {
                    if let Some(level) = self.future_levels.next() {
                        self.case_tree = load::LevelData::load(&level).unwrap();
                        self.current_level = level;
                    }
                }
            }
            Msg::ResetLevel => {
                self.drag = None;
                self.case_tree = self.current_level.load().unwrap();
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
                        self.case_tree.set_node_position(*node, [x, y]);
                    }
                    DragObject::Wire(_) => {}
                    DragObject::Background => {
                        self.case_tree.scroll_background(dx, dy);

                        // Update coord in response to changing coordinate system.
                        coord.0 -= dx;
                        coord.1 -= dy;
                    }
                }
            }
        }
    }
}

impl<'a> dodrio::Render<'a> for Model {
    fn render(&self, cx: &mut dodrio::RenderContext<'a>) -> dodrio::Node<'a> {
        self.case_tree.render(cx, self.unlock_state())
    }
}
