mod render;

use super::case::Case;
use super::case_tree::CaseTree;

pub struct LevelState {
    pub case_tree: CaseTree,
    svg_corners: ([f64; 2], [f64; 2]),
    text_box: Option<String>,
}

impl LevelState {
    pub fn new(case: Case, text_box: Option<String>) -> Self {
        Self {
            case_tree: CaseTree::new(case),
            svg_corners: ([-10., -1.], [10., 19.]),
            text_box,
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
