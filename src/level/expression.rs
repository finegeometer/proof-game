use super::case::*;
use super::*;
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Expression<T> {
    And(SmallVec<[T; 2]>),
    Or(SmallVec<[T; 2]>),
    Implies([T; 2]),
    Equal([T; 2]),
    Variable(String),
}

impl<T> Expression<T> {
    pub fn text(&self) -> &str {
        match self {
            Expression::And(_) => "∧",
            Expression::Or(_) => "∨",
            Expression::Implies(_) => "⇒",
            Expression::Equal(_) => "=",
            Expression::Variable(x) => x,
        }
    }

    pub fn inputs(&self) -> &[T] {
        match self {
            Expression::And(inputs) => inputs,
            Expression::Or(inputs) => inputs,
            Expression::Implies(inputs) => inputs,
            Expression::Equal(inputs) => inputs,
            Expression::Variable(_) => &[],
        }
    }

    fn inputs_mut(&mut self) -> &mut [T] {
        match self {
            Expression::And(inputs) => inputs,
            Expression::Or(inputs) => inputs,
            Expression::Implies(inputs) => inputs,
            Expression::Equal(inputs) => inputs,
            Expression::Variable(_) => &mut [],
        }
    }

    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Expression<U> {
        match self {
            Expression::And(inputs) => Expression::And(inputs.into_iter().map(f).collect()),
            Expression::Or(inputs) => Expression::Or(inputs.into_iter().map(f).collect()),
            Expression::Implies(inputs) => Expression::Implies(inputs.map(f)),
            Expression::Equal(inputs) => Expression::Equal(inputs.map(f)),
            Expression::Variable(s) => Expression::Variable(s),
        }
    }
}

impl egg::Language for Expression<egg::Id> {
    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Expression::And(a), Expression::And(b)) => a.len() == b.len(),
            (Expression::Or(a), Expression::Or(b)) => a.len() == b.len(),
            (Expression::Implies(_), Expression::Implies(_)) => true,
            (Expression::Equal(_), Expression::Equal(_)) => true,
            (Expression::Variable(a), Expression::Variable(b)) => a == b,
            (_, _) => false,
        }
    }

    fn children(&self) -> &[egg::Id] {
        self.inputs()
    }

    fn children_mut(&mut self) -> &mut [egg::Id] {
        self.inputs_mut()
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
            (Expression::Equal([w1, w2]), true) => !self.wire_eq(*w1, *w2),
            (Expression::Equal([w1, w2]), false) => self.wire_eq(*w1, *w2),
            (Expression::Variable(_), _) => false,
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
                self.edit_case([|case: &mut Case| {
                    case.set_proven(
                        output,
                        ValidityReason::new("The inputs are literally the same."),
                    );
                }]);
            }
            (Expression::Variable(_), true) => {}
            (Expression::Variable(_), false) => {}
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

pub fn box_closure(f: impl Fn(&mut Case) + 'static) -> Box<dyn Fn(&mut Case)> {
    Box::new(f)
}
