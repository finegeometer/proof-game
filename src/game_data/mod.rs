mod json;
use std::{
    cmp::Ordering,
    ops::{BitOr, BitOrAssign},
};

/// Any data that pertains to the game as a whole,
/// as opposed to what the player has done in the game.
/// In other words, to create a custom map, this is what needs to be replaced.
#[derive(serde::Deserialize, Default)]
#[serde(try_from = "json::GameJson")]
pub struct GameData {
    levels: Vec<Level>,
}

pub struct Level {
    name: String,
    pub spec: crate::level::LevelSpec,
    pub panzoom: crate::render::PanZoom,
    text_box: Option<(String, Option<crate::book::BookPage>)>,
    pub map_position: [f64; 2],
    pub bezier_vector: [f64; 2],
    pub prereqs: Vec<usize>,
    pub next_level: Vec<usize>,
    pub unlocks: Unlocks,
    pub axiom: bool,
}

impl GameData {
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn level(&self, level: usize) -> &Level {
        &self.levels[level]
    }

    pub fn load(&self, level: usize, global_unlocks: Unlocks) -> crate::level::State {
        let Level {
            spec,
            panzoom: pan_zoom,
            text_box,
            axiom,
            ..
        } = self.level(level);
        crate::level::State::new(
            spec,
            *pan_zoom,
            text_box.clone(),
            global_unlocks | self.level(level).unlocks,
            *axiom,
        )
    }
}

/// Data describing what the player has done in the game.
/// In other words, this is what the save/load game buttons manipulate.
#[derive(Default)]
pub struct SaveData {
    unlocks: Unlocks,
    completed: Vec<bool>,
}

impl SaveData {
    pub fn new(game_data: &GameData) -> Self {
        Self {
            unlocks: Unlocks::NONE,
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

    pub fn unlocks(&self) -> Unlocks {
        self.unlocks
    }

    pub fn set_unlocked(&mut self, unlock: Unlocks) {
        self.unlocks |= unlock;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(from = "Vec<&str>", into = "Vec<&str>")]
pub struct Unlocks(u8);

impl From<Unlocks> for Vec<&'static str> {
    fn from(unlocks: Unlocks) -> Self {
        [
            ("cases", Unlocks::CASES),
            ("lemmas", Unlocks::LEMMAS),
            ("theorem-application", Unlocks::THEOREM_APPLICATION),
            ("everything", Unlocks::ALL),
        ]
        .into_iter()
        .filter_map(|(name, unlock)| (unlocks >= unlock).then_some(name))
        .collect()
    }
}

impl<'a> From<Vec<&'a str>> for Unlocks {
    fn from(unlocks: Vec<&'a str>) -> Self {
        let mut out = Self::NONE;
        for unlock in unlocks.clone() {
            out |= match unlock {
                "cases" => Unlocks::CASES,
                "lemmas" => Unlocks::LEMMAS,
                "theorem-application" => Unlocks::THEOREM_APPLICATION,
                "everything" => Unlocks::ALL,
                _ => Unlocks::NONE,
            }
        }
        out
    }
}

impl BitOr for Unlocks {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Unlocks {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl PartialOrd for Unlocks {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.le(other), self.ge(other)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }

    fn le(&self, other: &Self) -> bool {
        self.0 & !other.0 == 0
    }

    fn ge(&self, other: &Self) -> bool {
        !self.0 & other.0 == 0
    }
}

impl Default for Unlocks {
    fn default() -> Self {
        Self::NONE
    }
}

impl Unlocks {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(!0);
    pub const CASES: Self = Self(1);
    pub const LEMMAS: Self = Self(2);
    pub const THEOREM_APPLICATION: Self = Self(4);
}
