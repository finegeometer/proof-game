mod json;

/// Any data that pertains to the game as a whole,
/// as opposed to what the player has done in the game.
/// In other words, to create a custom map, this is what needs to be replaced.
#[derive(serde::Deserialize)]
#[serde(try_from = "json::GameJson")]
pub struct GameData {
    levels: Vec<Level>,
}

pub struct Level {
    case: crate::level::case::Case,
    pan_zoom: crate::render::PanZoom,
    text_box: Option<String>,
    map_position: [f64; 2],
    bezier_vector: [f64; 2],
    prereqs: Vec<usize>,
    next_level: Option<usize>,
}

impl GameData {
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn load(&self, level: usize) -> crate::level::State {
        let Level {
            case,
            pan_zoom,
            text_box,
            ..
        } = &self.levels[level];
        crate::level::State::new(
            case.clone(),
            *pan_zoom,
            text_box.clone(),
            self.unlocks(level),
        )
    }

    pub fn next_level(&self, level: usize) -> Option<usize> {
        self.levels[level].next_level
    }

    pub fn prereqs(&self, level: usize) -> impl '_ + Iterator<Item = usize> {
        self.levels[level].prereqs.iter().copied()
    }

    pub fn map_position(&self, level: usize) -> [f64; 2] {
        self.levels[level].map_position
    }

    pub fn bezier_vector(&self, level: usize) -> [f64; 2] {
        self.levels[level].bezier_vector
    }

    fn unlocks(&self, level: usize) -> crate::UnlockState {
        match level {
            0..=6 => crate::UnlockState::None,
            7..=16 => crate::UnlockState::CaseTree,
            _ => crate::UnlockState::Lemmas,
        }
    }
}
