//! Code pertaining to individual levels of the game.

mod case;
mod case_tree;
pub mod expression;
mod render;

use std::collections::HashMap;

pub use case::LevelSpec;

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
    mode: Option<Mode>,
    last_recorded_mouse_position: [f64; 2],
}

enum Mode {
    ChooseTheoremLocation(LevelSpec),
    AssignTheoremVars {
        spec: LevelSpec,
        offset: [f64; 2],
        chosen: HashMap<String, Node>,
        current: String,
        remaining: std::vec::IntoIter<String>,
    },
    SelectUndo {
        preview: CaseId,
    },
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

    SelectedTheorem(LevelSpec),
    Cancel,

    RevertPreview(CaseId),
    RevertTo(CaseId),
}

#[derive(Debug, Clone, Copy)]
pub enum DragObject {
    Node(Node),
    Wire(Wire),
    Background,
}

impl State {
    pub fn new(
        spec: &LevelSpec,
        pan_zoom: PanZoom,
        text_box: Option<String>,
        unlocks: Unlocks,
        axiom: bool,
    ) -> Self {
        Self {
            case_tree: CaseTree::new(spec.to_case([0., 0.])),
            pan_zoom,
            text_box,
            drag: None,
            unlocks,
            axiom,
            mode: None,
            last_recorded_mouse_position: [0., 0.],
        }
    }

    pub fn update(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::MouseDown(x, y, object) => {
                let rerender = self.mouse_move(x, y);

                if self.drag.is_some() {
                    return rerender;
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

                let Some(DragState { confirmed_drag, object, .. }) = self.drag else {return rerender};

                if confirmed_drag.is_ok() {
                    // This is a drag.
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
                } else {
                    // This is a click.
                    match self.mode.take() {
                        Some(Mode::ChooseTheoremLocation(spec)) => {
                            self.start_processing_var(Mode::AssignTheoremVars {
                                offset: self.last_recorded_mouse_position,
                                chosen: HashMap::new(),
                                current: Default::default(),
                                remaining: spec
                                    .vars()
                                    .map(String::from)
                                    .collect::<Vec<String>>()
                                    .into_iter(),
                                spec,
                            });
                            rerender = true;
                        }
                        Some(Mode::AssignTheoremVars {
                            spec,
                            offset,
                            mut chosen,
                            current,
                            remaining,
                        }) => match object {
                            DragObject::Node(n) => {
                                chosen.insert(current, n);
                                self.start_processing_var(Mode::AssignTheoremVars {
                                    spec,
                                    offset,
                                    chosen,
                                    current: Default::default(),
                                    remaining,
                                });
                                rerender = true;
                            }
                            DragObject::Wire(_) | DragObject::Background => {
                                self.mode = Some(Mode::AssignTheoremVars {
                                    spec,
                                    offset,
                                    chosen,
                                    current,
                                    remaining,
                                })
                            }
                        },
                        Some(Mode::SelectUndo { preview }) => {
                            self.mode = Some(Mode::SelectUndo { preview })
                        }
                        None => match object {
                            DragObject::Node(node) => {
                                let (case, complete) = self.case_tree.case(self.case_tree.current);
                                if !self.axiom && !complete && case.node_has_interaction(node) {
                                    self.case_tree.interact_node(node);
                                    rerender = true;
                                }
                            }
                            DragObject::Wire(wire) => {
                                let (case, complete) = self.case_tree.case(self.case_tree.current);
                                if !self.axiom
                                    && self.unlocks >= Unlocks::LEMMAS
                                    && !complete
                                    && case.wire_has_interaction(wire)
                                {
                                    self.case_tree.interact_wire(wire);
                                    rerender = true;
                                }
                            }
                            DragObject::Background => {}
                        },
                    }
                }

                self.drag = None;
                rerender
            }
            Msg::MouseWheel(x, y, wheel) => {
                self.mouse_move(x, y);

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
                self.case_tree.current = id;
                self.mode = None;
                true
            }

            // Theorem application
            Msg::SelectedTheorem(spec) => {
                self.mode = Some(Mode::ChooseTheoremLocation(spec));
                true
            }
            Msg::RevertPreview(preview) => {
                self.mode = Some(Mode::SelectUndo { preview });
                true
            }
            Msg::RevertTo(case) => {
                self.mode = None;
                self.case_tree.revert_to(case);
                true
            }
            Msg::Cancel => {
                self.mode = None;
                true
            }
        }
    }

    fn start_processing_var(&mut self, theorem_application: Mode) {
        let Mode::AssignTheoremVars { spec, offset, chosen, current: _, mut remaining } = theorem_application else {return};
        for v in remaining.by_ref() {
            if chosen.contains_key(&v) {
                continue;
            } else {
                self.mode = Some(Mode::AssignTheoremVars {
                    spec,
                    offset,
                    chosen,
                    current: v,
                    remaining,
                });
                return;
            }
        }

        // If control reaches here, all variables have been chosen.

        self.mode = None;
        spec.add_to_case_tree(&mut self.case_tree, move |v| chosen[v], offset)
    }

    pub fn complete(&self) -> bool {
        self.case_tree.all_complete()
    }

    fn mouse_move(&mut self, x: f64, y: f64) -> bool {
        self.last_recorded_mouse_position = [x, y];

        let rerender = self.mode.is_some();

        let Some(DragState {
            coord,
            confirmed_drag,
            object,
        }) = &mut self.drag else {return rerender};

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
            return rerender;
        }

        match object {
            DragObject::Node(node) => {
                self.case_tree.set_node_position(*node, [x, y]);
            }
            DragObject::Wire(_) => return rerender,
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
