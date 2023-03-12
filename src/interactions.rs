use super::expr_graph::{operation::Operation::*, *};

impl Page {
    // Note: The behavior of this function is important to the correctness of `interact_node`.
    pub fn node_has_interaction(&self, node: Node) -> bool {
        let (op, mut inputs) = node.data();
        let output = node.output(self);
        match (op, self.wire_status(output)) {
            (And, None) => inputs.all(|wire| self.wire_status(wire).is_some()),
            (And, Some(())) => !inputs.all(|wire| self.wire_status(wire).is_some()),
            (Or, None) => inputs.any(|wire| self.wire_status(wire).is_some()),
            (Or, Some(())) => !inputs.any(|wire| self.wire_status(wire).is_some()),
            (Implies, None) => self.goal == output,
            (Implies, Some(())) => {
                let hypothesis = inputs
                    .next()
                    .expect("An implication must always have two children, not fewer.");
                let conclusion = inputs
                    .next()
                    .expect("An implication must always have two children, not fewer.");
                assert!(
                    inputs.next().is_none(),
                    "An implication must always have two children, not more."
                );

                self.wire_status(hypothesis).is_some() && self.wire_status(conclusion).is_none()
            }
            (Equal, None) => {
                let w0 = inputs
                    .next()
                    .expect("An equation must equate at least one expression.");
                inputs.all(|w| w == w0)
            }
            (Equal, Some(())) => {
                let w0 = inputs
                    .next()
                    .expect("An equation must equate at least one expression.");
                !inputs.all(|w| w == w0)
            }
            (Other(_), _) => false,
        }
    }

    pub fn wire_has_interaction(&self, wire: Wire) -> bool {
        self.wire_status(wire).is_none() && wire != self.goal
    }
}

impl State {
    pub fn interact_node(&mut self, node: Node) {
        if let Some(initial_page) = self.page() {
            if initial_page.node_has_interaction(node) {
                let (op, mut inputs) = node.data();
                let output = node.output(initial_page);
                match (op, initial_page.wire_status(output)) {
                    (And, None) => {
                        self.edit_page([|page: &mut Page| {
                            page.set_wire_status(
                            output,
                            ValidityReason(
                                "If a collection of propositions holds, so does their conjunction. \
                                This was checked in `node_has_interaction`.",
                            ),
                        )
                        }]);
                    }
                    (And, Some(())) => {
                        for wire in inputs {
                            self.edit_page([|page: &mut Page| {
                                page.set_wire_status(
                                    wire,
                                    ValidityReason(
                                        "If a conjunction holds, \
                                        so do each of the individual propositions.",
                                    ),
                                )
                            }]);
                        }
                    }
                    (Or, None) => {
                        if inputs.any(|wire| initial_page.wire_status(wire).is_some()) {
                            self.edit_page([|page: &mut Page| {
                                page.set_wire_status(
                                    output,
                                    ValidityReason(
                                        "A disjunction holds if any of the individual propositions hold. \
                                        This was checked in `node_has_interaction`.",
                                    ),
                                )
                            }]);
                        }
                    }
                    (Or, Some(())) => {
                        self.edit_page(inputs.map(|wire| {
                            move |page: &mut Page| {
                                page.set_wire_status(
                                    wire,
                                    ValidityReason(
                                        "If a disjunction holds, we can split into several cases. \
                                        In each case, one of the individual propositions holds.",
                                    ),
                                );
                            }
                        }));
                    }
                    (Implies, None) => {
                        let hypothesis = inputs
                            .next()
                            .expect("An implication must always have two children, not fewer.");
                        let conclusion = inputs
                            .next()
                            .expect("An implication must always have two children, not fewer.");
                        assert!(
                            inputs.next().is_none(),
                            "An implication must always have two children, not more."
                        );

                        self.edit_page([|page: &mut Page| {
                            page.set_wire_status(
                                hypothesis,
                                ValidityReason(
                                    "To prove an implication, one assumes the hypothesis, \
                                    and tries to prove the conclusion. \
                                    It was checked in `node_has_interaction` that the implication is the goal.",
                                ),
                            );
                            page.goal = conclusion;
                        }]);
                    }
                    (Implies, Some(())) => {
                        let _hypothesis = inputs
                            .next()
                            .expect("An implication must always have two children, not fewer.");
                        let conclusion = inputs
                            .next()
                            .expect("An implication must always have two children, not fewer.");
                        assert!(
                            inputs.next().is_none(),
                            "An implication must always have two children, not more."
                        );

                        self.edit_page([|page: &mut Page| {
                            page.set_wire_status(
                                conclusion,
                                ValidityReason(
                                    "If an implication holds, and its hypothesis holds, \
                                    then the conclusion holds. \
                                    The hypothesis was checked in `node_has_interaction`.",
                                ),
                            )
                        }]);
                    }
                    (Equal, None) => {
                        self.edit_page([|page: &mut Page| {
                            page.set_wire_status(
                                output,
                                ValidityReason(
                                    "Reflexivity. If all the inputs are literally the same wire, equality holds. \
                                    This was checked in `node_has_interaction`.",
                                ),
                            )
                        }]);
                    }
                    (Equal, Some(())) => {
                        let w0 = inputs
                            .next()
                            .expect("An equation must equate at least one expression.");
                        self.edit_page([|page: &mut Page| {
                            for w in inputs {
                                page.connect(
                                    w0,
                                    w,
                                    ValidityReason(
                                        "If two expressions are equal, \
                                    we may treat them as equivalent in all respects. \
                                    So we might as well merge the wires.",
                                    ),
                                )
                            }
                        }]);
                    }
                    (Other(_), _) => {}
                }
            }
        }
    }

    pub fn interact_wire(&mut self, wire: Wire) {
        self.edit_page([
            box_closure(move |page: &mut Page| {
                page.set_wire_status(
                    wire,
                    ValidityReason("On the next page, I require you to prove this."),
                );
            }),
            box_closure(move |page: &mut Page| {
                page.goal = wire;
            }),
        ]);
    }
}

fn box_closure(f: impl Fn(&mut Page) + 'static) -> Box<dyn Fn(&mut Page)> {
    Box::new(f)
}
