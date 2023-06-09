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
    name: String,
    case: crate::level::case::Case,
    pan_zoom: crate::render::PanZoom,
    text_box: Option<String>,
    map_position: [f64; 2],
    bezier_vector: [f64; 2],
    prereqs: Vec<usize>,
    next_level: Option<usize>,
    unlocks: crate::UnlockState,
}

impl GameData {
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn load(&self, level: usize, global_unlocks: crate::UnlockState) -> crate::level::State {
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
            global_unlocks.max(self.unlocks(level)),
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

    pub fn unlocks(&self, level: usize) -> crate::UnlockState {
        self.levels[level].unlocks
    }
}

/// Data describing what the player has done in the game.
/// In other words, this is what the save/load game buttons manipulate.
pub struct SaveData {
    unlocks: crate::UnlockState,
    completed: Vec<bool>,
}

impl SaveData {
    pub fn new(game_data: &GameData) -> Self {
        Self {
            unlocks: crate::UnlockState::None,
            completed: vec![false; game_data.num_levels()],
        }
    }

    pub fn save(&self, game_data: &GameData) -> String {
        serde_json::to_string(&self.to_json(game_data)).unwrap()
    }

    pub fn load(game_data: &GameData, json: &str) -> Result<Self, serde_json::Error> {
        let json: json::SaveJson = serde_json::from_str(json)?;
        Ok(json.to_data(game_data))
    }

    pub fn completed(&self, level: usize) -> bool {
        self.completed[level]
    }

    /// Returns whether the save data has changed.
    pub fn mark_completed(&mut self, level: usize) -> bool {
        !std::mem::replace(&mut self.completed[level], true)
    }

    pub fn unlocks(&self) -> crate::UnlockState {
        self.unlocks
    }

    /// Returns whether the save data has changed.
    pub fn set_unlocked(&mut self, unlock: crate::UnlockState) -> bool {
        let dirty = !(unlock <= self.unlocks);
        self.unlocks = self.unlocks.max(unlock);
        dirty
    }
}
