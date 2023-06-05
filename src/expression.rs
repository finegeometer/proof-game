use super::*;
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub enum Expression {
    And(SmallVec<[Wire; 2]>),
    Or(SmallVec<[Wire; 2]>),
    Implies([Wire; 2]),
    Equal([Wire; 2]),
    Other(String),
}

impl Expression {
    pub fn text(&self) -> &str {
        match self {
            Expression::And(_) => "∧",
            Expression::Or(_) => "∨",
            Expression::Implies(_) => "⇒",
            Expression::Equal(_) => "=",
            Expression::Other(x) => x,
        }
    }

    pub fn inputs(&self) -> &[Wire] {
        match self {
            Expression::And(inputs) => inputs,
            Expression::Or(inputs) => inputs,
            Expression::Implies(inputs) => inputs,
            Expression::Equal(inputs) => inputs,
            Expression::Other(_) => &[],
        }
    }
}

impl Case {
    // Note: The behavior of this function is important to the correctness of `interact_node`.
    pub fn node_has_interaction(&self, node: Node) -> bool {
        let output = self.node_output(node);
        match (self.node_expression(node), self.proven(output)) {
            (Expression::And(inputs), true) => !inputs.iter().all(|&wire| self.proven(wire)),
            (Expression::And(inputs), false) => inputs.iter().all(|&wire| self.proven(wire)),
            (Expression::Or(inputs), true) => !inputs.iter().any(|&wire| self.proven(wire)),
            (Expression::Or(inputs), false) => inputs.iter().any(|&wire| self.proven(wire)),
            (Expression::Implies([hypothesis, conclusion]), true) => {
                self.proven(*hypothesis) && !self.proven(*conclusion)
            }
            (Expression::Implies(_), false) => self.wire_eq(self.goal(), output),
            (Expression::Equal([w1, w2]), true) => self.wire_eq(*w1, *w2),
            // FIXME
            (Expression::Equal(_), false) => todo!("Check whether nodes are equivalent."),
            (Expression::Other(_), _) => false,
        }
    }

    pub fn wire_has_interaction(&self, wire: Wire) -> bool {
        !(self.proven(wire) || self.wire_eq(wire, self.goal()))
    }
}

impl CaseTree {
    pub fn interact_node(&mut self, node: Node) {
        let initial_case = self.current_case().0;
        let output = initial_case.node_output(node);
        match (
            initial_case.node_expression(node),
            initial_case.proven(output),
        ) {
            (Expression::And(inputs), true) => {
                for wire in inputs.clone() {
                    self.edit_case([|case: &mut Case| {
                        case.set_proven(
                            wire,
                            ValidityReason::new(
                                r"
If a conjunction holds, so do each of the individual propositions.",
                            ),
                        )
                    }]);
                }
            }
            (Expression::And(_), false) => {
                self.edit_case([|case: &mut Case| {
                    case.set_proven(
                        output,
                        ValidityReason::new(
                            r"
If a collection of propositions holds, so does their conjunction.
This was checked in `node_has_interaction`.",
                        ),
                    )
                }]);
            }
            (Expression::Or(inputs), true) => {
                let inputs = inputs.clone();
                self.edit_case(inputs.iter().map(|&wire| {
                    move |case: &mut Case| {
                        case.set_proven(
                            wire,
                            ValidityReason::new(
                                r"
If a disjunction holds, we can split into several cases.
In each case, one of the individual propositions holds.",
                            ),
                        );
                    }
                }));
            }
            (Expression::Or(_), false) => {
                self.edit_case([|case: &mut Case| {
                    case.set_proven(
                        output,
                        ValidityReason::new(
                            r"
A disjunction holds if any of the individual propositions hold.
This was checked in `node_has_interaction`.",
                        ),
                    )
                }]);
            }
            (Expression::Implies([_, conclusion]), true) => {
                let conclusion = *conclusion;
                self.edit_case([|case: &mut Case| {
                    case.set_proven(
                        conclusion,
                        ValidityReason::new(
                            r"
If an implication holds, and its hypothesis holds, then the conclusion holds.
The hypothesis was checked in `node_has_interaction`.",
                        ),
                    )
                }]);
            }
            (Expression::Implies([hypothesis, conclusion]), false) => {
                let hypothesis = *hypothesis;
                let conclusion = *conclusion;
                self.edit_case([|case: &mut Case| {
                    case.set_proven(
                        hypothesis,
                        ValidityReason::new(
                            r"
To prove an implication, one assumes the hypothesis, and tries to prove the conclusion.
It was checked in `node_has_interaction` that the implication is the goal.",
                        ),
                    );
                    case.set_goal(conclusion);
                }]);
            }
            (Expression::Equal([w1, w2]), true) => {
                let w1 = *w1;
                let w2 = *w2;
                self.edit_case([|case: &mut Case| {
                    case.connect(
                        w1,
                        w2,
                        ValidityReason::new(
                            r"
If two expressions are equal, we may treat them as equivalent in all respects.
So we might as well merge the wires.",
                        ),
                    )
                }]);
            }
            (Expression::Equal(_), false) => {
                todo!("Check whether nodes are equivalent.")
            }
            (Expression::Other(_), true) => {}
            (Expression::Other(_), false) => {}
        }
    }

    pub fn interact_wire(&mut self, wire: Wire) {
        self.edit_case([
            box_closure(move |case: &mut Case| {
                case.set_goal(wire);
            }),
            box_closure(move |case: &mut Case| {
                case.set_proven(
                    wire,
                    ValidityReason::new("In the next case, you were required to prove this."),
                );
            }),
        ]);
    }
}

fn box_closure(f: impl Fn(&mut Case) + 'static) -> Box<dyn Fn(&mut Case)> {
    Box::new(f)
}
