mod json;
mod render;

/// Any data that pertains to the game as a whole,
/// as opposed to what the player has done in the game.
/// In other words, to create a custom map, this is what needs to be replaced.
#[derive(serde::Deserialize)]
#[serde(try_from = "json::GameJson")]
pub struct GameData {
    levels: Vec<Level>,
}

pub struct Level {
    case: crate::case::Case,
    svg_corners: ([f64; 2], [f64; 2]),
    text_box: Option<String>,
    map_position: [f64; 2],
}

impl GameData {
    pub fn load(&self, level: usize) -> crate::level::State {
        let Level {
            case,
            svg_corners,
            text_box,
            ..
        } = &self.levels[level];
        crate::level::State::new(
            case.clone(),
            *svg_corners,
            text_box.clone(),
            self.unlocks(level),
        )
    }

    pub fn next_level(&self, level: usize) -> Option<usize> {
        (level + 1 < self.levels.len()).then_some(level + 1)
    }

    pub fn prereqs(&self, level: usize) -> impl Iterator<Item = usize> {
        (level > 0).then_some(level - 1).into_iter()
    }

    pub fn map_position(&self, level: usize) -> [f64; 2] {
        self.levels[level].map_position
    }

    fn unlocks(&self, level: usize) -> crate::UnlockState {
        match level {
            0..=6 => crate::UnlockState::None,
            7..=16 => crate::UnlockState::CaseTree,
            _ => crate::UnlockState::Lemmas,
        }
    }
}
