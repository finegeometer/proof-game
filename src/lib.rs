#![warn(clippy::todo)]
#![allow(clippy::new_without_default)]

use wasm_bindgen::JsCast;

mod file;

mod game_data;

mod level;
mod world_map;

mod render;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let (send_msg, recv_msg) = async_channel::unbounded();

    let body = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap();
    let vdom = dodrio::Vdom::new(&body, Model::new(send_msg));

    wasm_bindgen_futures::spawn_local(async move {
        let vdom = vdom.weak();

        loop {
            let recv_msg = recv_msg.clone();

            let msg = recv_msg.recv().await.unwrap();

            let rerender = vdom
                .with_component(move |root| {
                    let mut rerender = false;
                    let model = root.unwrap_mut::<Model>();
                    rerender |= model.update(msg);
                    while let Ok(msg) = recv_msg.try_recv() {
                        rerender |= model.update(msg);
                    }
                    rerender
                })
                .await
                .unwrap();

            if rerender {
                vdom.render().await.unwrap();
            }
        }
    })
}

struct Model {
    send_msg: async_channel::Sender<Msg>,

    game_data: game_data::GameData,
    game_state: GameState,
    global_state: GlobalState,

    save_listener: js_sys::Function,
}

pub struct GlobalState {
    map_panzoom: render::PanZoom,
    unlocks: UnlockState,
    completed: Vec<bool>,
}

enum GameState {
    Level {
        level: usize,
        next_level: Option<usize>,
        level_state: level::State,
    },
    WorldMap(world_map::State),
}

impl GameState {
    fn level(game_data: &game_data::GameData, level: usize, global_state: &GlobalState) -> Self {
        Self::Level {
            level,
            next_level: game_data.next_level(level).filter(|&next_level| {
                game_data
                    .prereqs(next_level)
                    .filter(|&prereq| prereq != level)
                    .all(|prereq| global_state.completed[prereq])
            }),
            level_state: game_data.load(level, global_state.unlocks),
        }
    }

    fn map() -> Self {
        Self::WorldMap(world_map::State::new())
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
    fn new(send_msg: async_channel::Sender<Msg>) -> Self {
        let game_data: game_data::GameData =
            serde_json::from_str(include_str!("./levels.json")).unwrap();

        let global_state = GlobalState {
            map_panzoom: render::PanZoom::center([0.; 2], 10.),
            unlocks: UnlockState::None,
            completed: vec![false; game_data.num_levels()],
        };

        let save_listener: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)> =
            wasm_bindgen::closure::Closure::wrap(Box::new(|e| {
                let e: web_sys::BeforeUnloadEvent = e.dyn_into().unwrap();
                e.prevent_default();
                e.set_return_value("The game is unsaved — are you sure you want to leave?");
            }));
        let save_listener: js_sys::Function = save_listener.into_js_value().unchecked_into();

        Self {
            send_msg,

            game_data,
            game_state: GameState::map(),
            global_state,

            save_listener,
        }
    }

    fn update(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Level(msg) => {
                let GameState::Level { level_state, level, .. } = &mut self.game_state else {return false};
                let rerender = level_state.update(msg);
                if level_state.complete() {
                    let complete = &mut self.global_state.completed[*level];
                    if !*complete {
                        *complete = true;
                        self.global_state.unlocks = self
                            .global_state
                            .unlocks
                            .max(self.game_data.unlocks(*level));
                        web_sys::window()
                            .unwrap()
                            .add_event_listener_with_callback("beforeunload", &self.save_listener)
                            .unwrap();
                    }
                }
                rerender
            }
            Msg::WorldMap(msg) => {
                let GameState::WorldMap(map_state) = &mut self.game_state else {return false};
                map_state.update(msg, &mut self.global_state)
            }
            Msg::LoadLevel(level) => {
                self.game_state = GameState::level(&self.game_data, level, &self.global_state);
                true
            }
            Msg::LoadMap { recenter } => match self.game_state {
                GameState::Level {
                    level: level_num, ..
                } => {
                    self.game_state = GameState::map();
                    if recenter {
                        self.global_state.map_panzoom =
                            render::PanZoom::center(self.game_data.map_position(level_num), 10.);
                    }
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
                level,
                next_level,
            } => builder
                .children(level_state.render(cx, *level, *next_level))
                .finish(),
            GameState::WorldMap(map_state) => builder
                .children([map_state.render(cx, &self.game_data, &self.global_state)])
                .finish(),
        }
    }
}
