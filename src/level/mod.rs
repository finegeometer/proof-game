//! Code pertaining to individual levels of the game.

pub mod case;
mod case_tree;
pub mod expression;
mod render;

use crate::{game_data::Unlocks, render::PanZoom};
use case::{Case, Node, ValidityReason, Wire};
use case_tree::{CaseId, CaseTree};

pub struct State {
    pub case_tree: CaseTree,
    pan_zoom: PanZoom,
    text_box: Option<String>,
    drag: Option<DragState>,
    unlocks: Unlocks,
    axiom: bool,
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

#[derive(Debug)]
pub enum Msg {
    MouseDown(f64, f64, DragObject),
    MouseMove(f64, f64),
    MouseUp(f64, f64, Option<Node>),
    MouseWheel(f64, f64, f64),
    GotoCase(CaseId),
}

#[derive(Debug, Clone, Copy)]
pub enum DragObject {
    Node(Node),
    Wire(Wire),
    Background,
}

impl State {
    pub fn new(
        case: Case,
        pan_zoom: PanZoom,
        text_box: Option<String>,
        unlocks: Unlocks,
        axiom: bool,
    ) -> Self {
        Self {
            case_tree: CaseTree::new(case),
            pan_zoom,
            text_box,
            drag: None,
            unlocks,
            axiom,
        }
    }

    pub fn update(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::MouseDown(x, y, object) => {
                if self.drag.is_some() {
                    return false;
                }

                self.drag = Some(DragState {
                    coord: (x, y),
                    confirmed_drag: Err((x, y)),
                    object,
                });
                true
            }
            Msg::MouseMove(x, y) => self.mouse_move(x, y),
            Msg::MouseUp(x, y, dropped_on) => {
                let mut rerender = self.mouse_move(x, y);

                #[allow(clippy::collapsible_match)]
                match self.drag {
                    Some(DragState {
                        confirmed_drag: Err(_),
                        object: DragObject::Node(node),
                        ..
                    }) => {
                        let (case, complete) = self.case_tree.current_case();
                        if !self.axiom && !complete && case.node_has_interaction(node) {
                            self.case_tree.interact_node(node);
                            rerender = true;
                        }
                    }
                    Some(DragState {
                        confirmed_drag: Err(_),
                        object: DragObject::Wire(wire),
                        ..
                    }) => {
                        let (case, complete) = self.case_tree.current_case();
                        if !self.axiom
                            && self.unlocks >= Unlocks::LEMMAS
                            && !complete
                            && case.wire_has_interaction(wire)
                        {
                            self.case_tree.interact_wire(wire);
                            rerender = true;
                        }
                    }
                    Some(DragState {
                        confirmed_drag: Err(_),
                        object: DragObject::Background,
                        ..
                    }) => {}
                    Some(DragState {
                        confirmed_drag: Ok(()),
                        object,
                        ..
                    }) => {
                        if !self.axiom {
                            if let DragObject::Node(n1) = object {
                                if let Some(n2) = dropped_on {
                                    self.case_tree.edit_case([|case: &mut Case| {
                                        let w1 = case.node_output(n1);
                                        let w2 = case.node_output(n2);
                                        if case.wire_equiv(w1, w2) {
                                            case.connect(
                                                w1,
                                                w2,
                                                ValidityReason::new("I just checked equivalence."),
                                            );
                                        }
                                    }]);
                                    rerender = true;
                                }
                            }
                        }
                    }
                    None => {}
                }
                self.drag = None;
                rerender
            }
            Msg::MouseWheel(x, y, wheel) => {
                self.pan_zoom.zoom(x, y, (wheel * 0.001).exp());

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

                true
            }
            Msg::GotoCase(id) => {
                self.case_tree.goto_case(id);
                true
            }
        }
    }

    pub fn complete(&self) -> bool {
        self.case_tree.all_complete()
    }

    fn mouse_move(&mut self, x: f64, y: f64) -> bool {
        let Some(DragState {
            coord,
            confirmed_drag,
            object,
        }) = &mut self.drag else {return false};

        let dx = x - coord.0;
        let dy = y - coord.1;

        coord.0 = x;
        coord.1 = y;

        if let Err(init_coord) = confirmed_drag {
            if (coord.0 - init_coord.0).powi(2) + (coord.1 - init_coord.1).powi(2) > 0.01 {
                *confirmed_drag = Ok(());
            }
        }

        if confirmed_drag.is_err() {
            return false;
        }

        match object {
            DragObject::Node(node) => {
                self.case_tree.set_node_position(*node, [x, y]);
            }
            DragObject::Wire(_) => return false,
            DragObject::Background => {
                self.pan_zoom.pan(dx, dy);

                // Update coord in response to changing coordinate system.
                coord.0 -= dx;
                coord.1 -= dy;
            }
        }

        true
    }
}
