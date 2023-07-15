mod json;

use super::case::*;
use super::*;
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
#[serde(try_from = "json::ExpressionJson<T>")]
pub enum Expression<T> {
    And(SmallVec<[T; 2]>),
    Or(SmallVec<[T; 2]>),
    Implies([T; 2]),
    Equal([T; 2]),
    Variable(Var),
    Function(String, Type, SmallVec<[T; 2]>),
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var(pub String, pub Type);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    #[default]
    TruthValue,
    RealNumber,
}

impl<T> Expression<T> {
    pub fn text(&self) -> &str {
        match self {
            Expression::And(_) => "∧",
            Expression::Or(_) => "∨",
            Expression::Implies(_) => "⇒",
            Expression::Equal(_) => "=",
            Expression::Variable(Var(x, _)) => x,
            Expression::Function(f, _, _) => f,
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            Expression::And(_)
            | Expression::Or(_)
            | Expression::Implies(_)
            | Expression::Equal(_) => Type::TruthValue,
            Expression::Variable(Var(_, ty)) | Expression::Function(_, ty, _) => *ty,
        }
    }

    pub fn tycheck(&self, ty: impl Fn(&T) -> Type) -> bool {
        match self {
            Expression::And(inputs) | Expression::Or(inputs) => {
                inputs.iter().all(|x| ty(x) == Type::TruthValue)
            }
            Expression::Implies([a, b]) => ty(a) == Type::TruthValue && ty(b) == Type::TruthValue,
            Expression::Equal([a, b]) => ty(a) == ty(b),
            Expression::Variable(_) => true,
            Expression::Function(_, _, _) => true,
        }
    }

    pub fn inputs(&self) -> &[T] {
        match self {
            Expression::And(inputs) | Expression::Or(inputs) => inputs,
            Expression::Implies(inputs) | Expression::Equal(inputs) => inputs,
            Expression::Variable(_) => &[],
            Expression::Function(_, _, inputs) => inputs,
        }
    }

    fn inputs_mut(&mut self) -> &mut [T] {
        match self {
            Expression::And(inputs) => inputs,
            Expression::Or(inputs) => inputs,
            Expression::Implies(inputs) => inputs,
            Expression::Equal(inputs) => inputs,
            Expression::Variable(_) => &mut [],
            Expression::Function(_, _, inputs) => inputs,
        }
    }

    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Expression<U> {
        match self {
            Expression::And(inputs) => Expression::And(inputs.into_iter().map(f).collect()),
            Expression::Or(inputs) => Expression::Or(inputs.into_iter().map(f).collect()),
            Expression::Implies(inputs) => Expression::Implies(inputs.map(f)),
            Expression::Equal(inputs) => Expression::Equal(inputs.map(f)),
            Expression::Variable(v) => Expression::Variable(v),
            Expression::Function(s, ty, inputs) => {
                Expression::Function(s, ty, inputs.into_iter().map(f).collect())
            }
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
            (Expression::Function(f1, t1, args1), Expression::Function(f2, t2, args2)) => {
                f1 == f2 && t1 == t2 && args1.len() == args2.len()
            }
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
            (Expression::Function(_, _, _), _) => false,
        }
    }

    pub fn wire_has_interaction(&self, wire: Wire) -> bool {
        self.ty(wire) == Type::TruthValue && !self.proven(wire) && !self.wire_eq(wire, self.goal())
    }
}

impl CaseTree {
    pub fn interact_node(&mut self, node: Node) {
        let initial_case = self.case(self.current).0;
        let output = initial_case.node_output(node);
        match (
            initial_case.node_expression(node),
            initial_case.proven(output),
        ) {
            (Expression::And(inputs), true) => {
                for wire in inputs.clone() {
                    self.current_case_mut().set_proven(
                        wire,
                        ValidityReason::new(
                            r"
If a conjunction holds, so do each of the individual propositions.",
                        ),
                    );
                }
            }
            (Expression::And(_), false) => self.current_case_mut().set_proven(
                output,
                ValidityReason::new(
                    r"
If a collection of propositions holds, so does their conjunction.
This was checked in `node_has_interaction`.",
                ),
            ),
            (Expression::Or(inputs), true) => {
                let subcases = inputs
                    .iter()
                    .map(|&wire| {
                        let mut case = self.case(self.current).0.clone();
                        case.set_proven(
                            wire,
                            ValidityReason::new(
                                r"
If a disjunction holds, we can split into several cases.
In each case, one of the individual propositions holds.",
                            ),
                        );
                        case
                    })
                    .collect::<Vec<_>>();
                self.case_split(subcases)
            }
            (Expression::Or(_), false) => self.current_case_mut().set_proven(
                output,
                ValidityReason::new(
                    r"
A disjunction holds if any of the individual propositions hold.
This was checked in `node_has_interaction`.",
                ),
            ),
            (Expression::Implies([_, conclusion]), true) => {
                let conclusion = *conclusion;
                self.current_case_mut().set_proven(
                    conclusion,
                    ValidityReason::new(
                        r"
If an implication holds, and its hypothesis holds, then the conclusion holds.
The hypothesis was checked in `node_has_interaction`.",
                    ),
                )
            }
            (Expression::Implies([hypothesis, conclusion]), false) => {
                let hypothesis = *hypothesis;
                let conclusion = *conclusion;
                let mut case = self.case(self.current).0.clone();

                case.set_proven(
                    hypothesis,
                    ValidityReason::new(
                        r"
To prove an implication, one assumes the hypothesis, and tries to prove the conclusion.
It was checked in `node_has_interaction` that the implication is the goal.",
                    ),
                );

                case.set_goal(conclusion);

                self.case_split([case]);
            }
            (Expression::Equal([w1, w2]), true) => {
                let w1 = *w1;
                let w2 = *w2;
                self.current_case_mut().connect(
                    w1,
                    w2,
                    ValidityReason::new(
                        r"
If two expressions are equal, we may treat them as equivalent in all respects.
So we might as well merge the wires.",
                    ),
                )
            }
            (Expression::Equal(_), false) => {
                self.current_case_mut().set_proven(
                    output,
                    ValidityReason::new("The inputs are literally the same."),
                );
            }
            (Expression::Variable(_), _) => {}
            (Expression::Function(_, _, _), _) => {}
        }
    }

    pub fn interact_wire(&mut self, wire: Wire) {
        let mut subcases = [
            self.case(self.current).0.clone(),
            self.case(self.current).0.clone(),
        ];

        subcases[0].set_goal(wire);
        subcases[1].set_proven(
            wire,
            ValidityReason::new("In the next case, you were required to prove this."),
        );

        self.case_split(subcases);
    }
}
