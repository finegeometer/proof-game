use super::case::*;

// Currently more of a case list.
// I plan to make it a tree later, for better user experience.
pub struct CaseTree {
    earlier: Vec<Case>,
    current: Option<Case>,
    later: Vec<Case>,
    pub svg_corners: ([f64; 2], [f64; 2]),
}

impl CaseTree {
    pub fn new(case: Case) -> Self {
        Self {
            earlier: Vec::new(),
            current: Some(case),
            later: Vec::new(),
            svg_corners: ([-1.; 2], [19.; 2]), // NOTE: If I update this, I need to update the viewBox.
        }
    }

    pub fn current_case(&self) -> &Option<Case> {
        &self.current
    }

    pub fn next_case(&mut self) -> Option<()> {
        self.earlier.extend(std::mem::replace(
            &mut self.current,
            Some(self.later.pop()?),
        ));
        Some(())
    }

    pub fn prev_case(&mut self) -> Option<()> {
        self.later.extend(std::mem::replace(
            &mut self.current,
            Some(self.earlier.pop()?),
        ));
        Some(())
    }

    pub fn cases_left(&self) -> usize {
        self.earlier.len()
    }

    pub fn cases_right(&self) -> usize {
        self.later.len()
    }

    /// Edit the current case, possibly splitting it into several in the process.
    pub fn edit_case(&mut self, fs: impl IntoIterator<Item = impl FnOnce(&mut Case)>) {
        if let Some(case) = self.current.take() {
            let len = self.earlier.len();
            self.earlier.extend(
                fs.into_iter()
                    .map(|f| {
                        let mut case = case.clone();
                        f(&mut case);
                        case
                    })
                    .filter(|case| !case.proven(case.goal())),
            );
            if self.earlier.len() > len {
                self.current = self.earlier.pop();
            }
        }
    }

    pub fn set_node_position(&mut self, node: Node, position: [f64; 2]) {
        if let Some(case) = &mut self.current {
            case.set_position(node, position);
        }
    }

    pub fn scroll_background(&mut self, dx: f64, dy: f64) {
        self.svg_corners.0[0] -= dx;
        self.svg_corners.1[0] -= dx;
        self.svg_corners.0[1] -= dy;
        self.svg_corners.1[1] -= dy;
    }

    pub fn zoom_background(&mut self, x: f64, y: f64, scale_factor: f64) {
        self.svg_corners.0[0] = (self.svg_corners.0[0] - x) * scale_factor + x;
        self.svg_corners.1[0] = (self.svg_corners.1[0] - x) * scale_factor + x;
        self.svg_corners.0[1] = (self.svg_corners.0[1] - y) * scale_factor + y;
        self.svg_corners.1[1] = (self.svg_corners.1[1] - y) * scale_factor + y;
    }
}
