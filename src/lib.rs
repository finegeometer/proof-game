#![warn(clippy::todo)]
#![allow(clippy::new_without_default)]

use game_data::{GameData, SaveData};
use wasm_bindgen::JsCast;

mod file;
mod game_data;
mod level;
mod render;
mod world_map;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let (send_msg, recv_msg) = async_channel::unbounded();

    let vdom = dodrio::Vdom::new(
        &web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("vdom")
            .unwrap(),
        Model::new(send_msg),
    );

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
    // static
    send_msg: async_channel::Sender<Msg>,
    save_listener: js_sys::Function,

    // semi-static
    game_data: GameData,
    save_data: game_data::SaveData,

    // dynamic
    game_state: GameState,
    global_state: GlobalState,
}

pub struct GlobalState {
    map_panzoom: render::PanZoom,
}

enum GameState {
    Menu,
    WorldMap {
        map_state: world_map::State,
    },
    Level {
        level: usize,
        next_level: Option<usize>,
        level_state: Box<level::State>,
        theorem_select: Option<(render::PanZoom, world_map::State, Option<usize>)>,
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
            level_state: Box::new(game_data.load(level, save_data.unlocks())),
            theorem_select: None,
        }
    }

    fn map() -> Self {
        Self::WorldMap {
            map_state: world_map::State::new(),
        }
    }
}

#[derive(Debug)]
enum Msg {
    Level(level::Msg),
    WorldMap(world_map::Msg),

    GotoLevel(usize),
    GotoMap { recenter: bool },

    // Messages related to selecting theorems from the world map while in a level.
    SelectTheorem,
    PreviewTheorem(usize),
    SelectedTheorem(usize),

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
            save_listener,

            game_data: Default::default(),
            save_data: Default::default(),

            game_state: GameState::Menu,
            global_state: GlobalState {
                map_panzoom: render::PanZoom::center([0.; 2], 10.),
            },
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
                    self.save_data
                        .set_unlocked(self.game_data.level(*level).unlocks)
                }
                rerender
            }
            Msg::WorldMap(msg) => match &mut self.game_state {
                GameState::WorldMap { map_state } => {
                    map_state.update(msg, &mut self.global_state.map_panzoom)
                }
                GameState::Level {
                    theorem_select: Some((panzoom, map_state, _)),
                    ..
                } => map_state.update(msg, panzoom),
                _ => false,
            },

            Msg::GotoLevel(level) => {
                self.game_state = GameState::level(&self.game_data, level, &self.save_data);
                #[allow(clippy::collapsible_if)]
                if self.game_data.level(level).axiom {
                    if self.save_data.mark_completed(level) {
                        web_sys::window()
                            .unwrap()
                            .set_onbeforeunload(Some(&self.save_listener));
                    }
                    self.save_data
                        .set_unlocked(self.game_data.level(level).unlocks)
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

            Msg::SelectTheorem => {
                let GameState::Level { level, theorem_select, .. } = &mut self.game_state else { return false };
                if theorem_select.is_some() {
                    return false;
                }

                *theorem_select = Some((
                    render::PanZoom::center(self.game_data.level(*level).map_position, 10.),
                    world_map::State::new(),
                    None,
                ));
                true
            }
            Msg::PreviewTheorem(level) => {
                let GameState::Level { theorem_select: Some((_,_,preview)), .. } = &mut self.game_state else { return false };
                if *preview == Some(level) {
                    return false;
                }
                *preview = Some(level);
                true
            }
            Msg::SelectedTheorem(level) => {
                let GameState::Level { level_state, theorem_select, .. } = &mut self.game_state else { return false };
                *theorem_select = None;
                level_state.update(level::Msg::SelectedTheorem(
                    self.game_data.level(level).spec.clone(),
                ));
                true
            }

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

        let mut builder = div(cx.bump).attributes([attr("id", "top")]).listeners([on(
            cx.bump,
            "contextmenu",
            |_, _, e| e.prevent_default(),
        )]);

        match &self.game_state {
            GameState::Level {
                level_state,
                level,
                next_level,
                theorem_select: None,
            } => {
                for child in level_state.render(cx, *level, *next_level) {
                    builder = builder.child(child);
                }
            }
            GameState::WorldMap { map_state } => {
                builder = builder
                    .child(
                        div(cx.bump)
                            .attributes([attr("class", "col wide")])
                            .children([map_state.render(
                                cx,
                                &self.game_data,
                                &self.global_state.map_panzoom,
                                &self.save_data,
                                false,
                            )])
                            .finish(),
                    )
                    .child(
                        div(cx.bump)
                            .attributes([attr("class", "col narrow")])
                            .children(save_load_buttons(cx.bump))
                            .finish(),
                    )
            }
            GameState::Level {
                theorem_select: Some((panzoom, map_state, preview)),
                ..
            } => {
                let col0 = div(cx.bump)
                    .attributes([attr("class", "col wide")])
                    .children([
                        map_state.render(cx, &self.game_data, panzoom, &self.save_data, true),
                        div(cx.bump)
                            .attributes([attr("class", "background disabled text-box")])
                            .children([text("Select a theorem to apply.")])
                            .finish(),
                    ])
                    .finish();
                let mut col1 = div(cx.bump).attributes([attr("class", "col wide")]);
                if let Some(preview) = preview {
                    let preview = self.game_data.level(*preview);
                    let mut svg = svg(cx.bump).attributes([
                        attr("class", "background disabled"),
                        attr("preserveAspectRatio", "xMidYMid meet"),
                        attr("font-size", "0.75"),
                        preview.panzoom.viewbox(cx.bump),
                    ]);
                    for child in preview.spec.render(cx, [0., 0.], |_| None) {
                        svg = svg.child(child);
                    }
                    col1 = col1.child(svg.finish());
                }
                builder = builder.child(col0).child(col1.finish())
            }
            GameState::Menu => {
                builder = builder.child(
                    div(cx.bump)
                        .attributes([attr("class", "col wide")])
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
                        .finish(),
                );
            }
        };

        builder.finish()
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
