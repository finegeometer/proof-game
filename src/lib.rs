#![warn(clippy::todo)]
#![allow(clippy::new_without_default)]

mod game_data;

mod level;
mod world_map;

mod render;

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
    global_state: GlobalState,
}

pub struct GlobalState {
    map_panzoom: render::PanZoom,
    unlocks: UnlockState,
}

enum GameState {
    Level {
        level_num: usize,
        level_state: level::State,
    },
    WorldMap(world_map::State),
}

impl GameState {
    fn level(game_data: &game_data::GameData, level: usize, global_unlocks: UnlockState) -> Self {
        Self::Level {
            level_num: level,
            level_state: game_data.load(level, global_unlocks),
        }
    }

    fn map(panzoom: render::PanZoom) -> Self {
        Self::WorldMap(world_map::State::new(panzoom))
    }
}

#[derive(Debug)]
enum Msg {
    Level(level::Msg),
    WorldMap(world_map::Msg),
    LoadLevel(usize),
    LoadMap { recenter: bool },
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

        let global_state = GlobalState {
            map_panzoom: render::PanZoom::center([0.; 2], 10.),
            unlocks: UnlockState::None,
        };

        Self {
            game_data,
            game_state: GameState::map(global_state.map_panzoom),
            global_state,
        }
    }

    fn update(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Level(msg) => {
                let GameState::Level { level_state, level_num } = &mut self.game_state else {return false};
                let rerender = level_state.update(msg);
                if level_state.complete() {
                    self.global_state.unlocks = self
                        .global_state
                        .unlocks
                        .max(self.game_data.unlocks(*level_num));
                }
                rerender
            }
            Msg::WorldMap(msg) => {
                let GameState::WorldMap(map_state) = &mut self.game_state else {return false};
                map_state.update(msg, &mut self.global_state)
            }
            Msg::LoadLevel(level) => {
                // TODO: Check that all prereqs are complete. If not, load map instead.
                self.game_state =
                    GameState::level(&self.game_data, level, self.global_state.unlocks);
                true
            }
            Msg::LoadMap { recenter } => match self.game_state {
                GameState::Level { level_num, .. } => {
                    self.game_state = GameState::map(if recenter {
                        render::PanZoom::center(self.game_data.map_position(level_num), 10.)
                    } else {
                        self.global_state.map_panzoom
                    });
                    true
                }
                GameState::WorldMap { .. } => false,
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
            GameState::Level {
                level_state,
                level_num,
            } => builder
                .children(level_state.render(cx, *level_num, self.game_data.next_level(*level_num)))
                .finish(),
            GameState::WorldMap(map_state) => builder
                .children([map_state.render(cx, &self.game_data)])
                .finish(),
        }
    }
}
