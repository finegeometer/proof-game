#![warn(clippy::todo)]
#![allow(clippy::new_without_default)]

mod case;
mod case_tree;
mod expression;
mod game_data;
mod level;
mod render;

pub use case::{Case, Node, ValidityReason, Wire};
pub use case_tree::CaseTree;
pub use expression::Expression;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let body = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap();
    dodrio::Vdom::new(&body, Model::new()).forget()
}

struct Model {
    game_data: game_data::GameData,
    game_state: GameState,
}

enum GameState {
    Level {
        level_num: usize,
        level_state: level::State,
    },
    WorldMap {
        pan_zoom: render::PanZoom,
    },
}

impl GameState {
    fn level(game_data: &game_data::GameData, level: usize) -> Self {
        Self::Level {
            level_num: level,
            level_state: game_data.load(level),
        }
    }

    fn map(pos: [f64; 2]) -> Self {
        Self::WorldMap {
            pan_zoom: render::PanZoom::center(pos, 10.),
        }
    }
}

#[derive(Debug)]
enum Msg {
    Level(level::Msg),
    LoadLevel(usize),
    NextLevel,
    ResetLevel,
    LoadMap,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnlockState {
    None,
    CaseTree,
    Lemmas,
}

impl Model {
    fn new() -> Self {
        let game_data: game_data::GameData =
            serde_json::from_str(include_str!("./levels.json")).unwrap();
        let level_num = 0;
        let level_state = game_data.load(level_num);

        Self {
            game_data,
            game_state: GameState::Level {
                level_num,
                level_state,
            },
        }
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Level(msg) => {
                if let GameState::Level { level_state, .. } = &mut self.game_state {
                    level_state.update(msg);
                }
            }
            Msg::LoadLevel(level) => self.game_state = GameState::level(&self.game_data, level),
            Msg::NextLevel => {
                if let GameState::Level {
                    level_num,
                    level_state,
                } = &self.game_state
                {
                    if level_state.case_tree.all_complete() {
                        if let Some(level) = self.game_data.next_level(*level_num) {
                            // TODO: Check that all prereqs are complete.
                            self.game_state = GameState::level(&self.game_data, level)
                        }
                    }
                }
            }
            Msg::ResetLevel => {
                if let GameState::Level { level_num, .. } = self.game_state {
                    self.game_state = GameState::level(&self.game_data, level_num)
                }
            }
            Msg::LoadMap => match self.game_state {
                GameState::Level { level_num, .. } => {
                    self.game_state = GameState::map(self.game_data.map_position(level_num))
                }
                GameState::WorldMap { .. } => {}
            },
        }
    }
}

impl<'a> dodrio::Render<'a> for Model {
    fn render(&self, cx: &mut dodrio::RenderContext<'a>) -> dodrio::Node<'a> {
        use dodrio::builder::*;

        let builder = div(cx.bump).attributes([attr("id", "top")]).listeners([on(
            cx.bump,
            "contextmenu",
            |_, _, e| e.prevent_default(),
        )]);

        match &self.game_state {
            GameState::Level { level_state, .. } => {
                builder.children(level_state.render(cx)).finish()
            }
            GameState::WorldMap { pan_zoom } => builder
                .children([self.game_data.world_map(cx, *pan_zoom)])
                .finish(),
        }
    }
}
