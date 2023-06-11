#![warn(clippy::todo)]
#![allow(clippy::new_without_default)]

use game_data::{GameData, SaveData};
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

    game_data: GameData,
    save_data: game_data::SaveData,
    game_state: GameState,
    global_state: GlobalState,

    save_listener: js_sys::Function,
}

pub struct GlobalState {
    map_panzoom: render::PanZoom,
}

enum GameState {
    Menu,
    WorldMap(world_map::State),
    Level {
        level: usize,
        next_level: Option<usize>,
        level_state: level::State,
    },
}

impl GameState {
    fn level(game_data: &GameData, level: usize, save_data: &SaveData) -> Self {
        Self::Level {
            level,
            next_level: game_data.level(level).next_level.filter(|&next_level| {
                game_data
                    .level(next_level)
                    .prereqs
                    .iter()
                    .filter(|&&prereq| prereq != level)
                    .all(|&prereq| save_data.completed(prereq))
            }),
            level_state: game_data.load(level, save_data.unlocks()),
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
    GotoLevel(usize),
    GotoMap { recenter: bool },

    LoadedSave(String),
    LoadingSaveFailed(),
    LoadedLevels(String),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnlockState {
    None,
    CaseTree,
    Lemmas,
}

impl Model {
    fn new(send_msg: async_channel::Sender<Msg>) -> Self {
        let save_listener: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)> =
            wasm_bindgen::closure::Closure::wrap(Box::new(|e| {
                let e: web_sys::BeforeUnloadEvent = e.dyn_into().unwrap();
                e.prevent_default();
                e.set_return_value("The game is unsaved — are you sure you want to leave?");
            }));
        let save_listener: js_sys::Function = save_listener.into_js_value().unchecked_into();

        Self {
            send_msg,

            game_data: Default::default(),
            save_data: Default::default(),
            game_state: GameState::Menu,
            global_state: GlobalState {
                map_panzoom: render::PanZoom::center([0.; 2], 10.),
            },

            save_listener,
        }
    }

    fn update(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Level(msg) => {
                let GameState::Level { level_state, level, .. } = &mut self.game_state else {return false};
                let rerender = level_state.update(msg);
                if level_state.complete() {
                    if self.save_data.mark_completed(*level) {
                        web_sys::window()
                            .unwrap()
                            .set_onbeforeunload(Some(&self.save_listener));
                    }
                    if self
                        .save_data
                        .set_unlocked(self.game_data.level(*level).unlocks)
                    {
                        web_sys::window()
                            .unwrap()
                            .set_onbeforeunload(Some(&self.save_listener));
                    }
                }
                rerender
            }
            Msg::WorldMap(msg) => {
                let GameState::WorldMap(map_state) = &mut self.game_state else {return false};
                map_state.update(msg, &mut self.global_state)
            }
            Msg::GotoLevel(level) => {
                self.game_state = GameState::level(&self.game_data, level, &self.save_data);
                #[allow(clippy::collapsible_if)]
                if self.game_data.level(level).axiom {
                    if self.save_data.mark_completed(level) {
                        web_sys::window()
                            .unwrap()
                            .set_onbeforeunload(Some(&self.save_listener));
                    }
                }
                true
            }
            Msg::GotoMap { recenter } => match self.game_state {
                GameState::Level {
                    level: level_num, ..
                } => {
                    self.game_state = GameState::map();
                    if recenter {
                        self.global_state.map_panzoom = render::PanZoom::center(
                            self.game_data.level(level_num).map_position,
                            10.,
                        );
                    }
                    true
                }
                GameState::WorldMap { .. } | GameState::Menu => false,
            },

            Msg::LoadedSave(save_file) => match SaveData::load(&self.game_data, &save_file) {
                Ok(save_data) => {
                    self.save_data = save_data;
                    web_sys::window().unwrap().set_onbeforeunload(None);
                    true
                }
                Err(err) => {
                    web_sys::console::warn_1(&format!("Failed to parse save file: {err}").into());
                    false
                }
            },
            Msg::LoadingSaveFailed() => {
                web_sys::console::warn_1(&"Failed to load save file.".into());
                false
            }
            Msg::LoadedLevels(json) => {
                self.game_data = serde_json::from_str(&json).unwrap();
                self.save_data = SaveData::new(&self.game_data);
                self.game_state = GameState::map();
                self.global_state = GlobalState {
                    map_panzoom: render::PanZoom::center([0.; 2], 10.),
                };
                true
            }
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
                .children([
                    div(cx.bump)
                        .attributes([attr("id", "col0")])
                        .children([map_state.render(
                            cx,
                            &self.game_data,
                            &self.global_state,
                            &self.save_data,
                        )])
                        .finish(),
                    div(cx.bump)
                        .attributes([attr("id", "col1")])
                        .children(save_load_buttons(cx.bump))
                        .finish(),
                ])
                .finish(),
            GameState::Menu => builder
                .children([div(cx.bump)
                    .attributes([attr("id", "col0")])
                    .children([div(cx.bump)
                        .attributes([attr("class", "button green")])
                        .listeners([file::fetch_listener(
                            cx.bump,
                            "levels.json",
                            Msg::LoadedLevels,
                            || panic!("Failed to load levels."),
                        )])
                        .children([text("Start!")])
                        .finish()])
                    .finish()])
                .finish(),
        }
    }
}

fn save_load_buttons(bump: &dodrio::bumpalo::Bump) -> [dodrio::Node; 3] {
    use dodrio::builder::*;
    [
        div(bump)
            .attributes([
                attr("id", "save-game"),
                attr(
                    "class",
                    if web_sys::window().unwrap().onbeforeunload().is_none() {
                        "button blue disabled"
                    } else {
                        "button blue"
                    },
                ),
            ])
            .listeners([file::save_listener(
                bump,
                |model| {
                    web_sys::window().unwrap().set_onbeforeunload(None);
                    model.save_data.save(&model.game_data)
                },
                "savefile.json",
            )])
            .children([text("Save Game")])
            .finish(),
        div(bump)
            .attributes([attr("id", "load-savegame"), attr("class", "button blue")])
            .listeners([on(bump, "click", |_, _, _| {
                let _ = || -> Option<()> {
                    web_sys::window()?
                        .document()?
                        .get_element_by_id("load-savegame-input")?
                        .dyn_into::<web_sys::HtmlElement>()
                        .ok()?
                        .click();
                    Some(())
                }();
            })])
            .children([text("Load Save")])
            .finish(),
        input(bump)
            .attributes([attr("id", "load-savegame-input"), attr("type", "file")])
            .listeners([file::load_listener(
                bump,
                Msg::LoadedSave,
                Msg::LoadingSaveFailed,
            )])
            .finish(),
    ]
}
