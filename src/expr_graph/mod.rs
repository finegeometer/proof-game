use std::ops::{Deref, DerefMut};

use self::local_state::LocalState;

mod core;
mod local_state;
pub mod operation;
mod render;
mod union_find;

pub use local_state::{Node, ValidityReason, Wire};

pub struct State {
    earlier: Vec<Page>,
    page: Option<Page>,
    later: Vec<Page>,
    render: render::State,
}

impl State {
    pub fn new() -> Self {
        let mut out = Self {
            earlier: Vec::new(),
            page: None,
            later: Vec::new(),
            render: render::State::new(),
        };
        render::State::load_page(&mut out);
        out
    }

    pub fn load_level(&mut self, page: Page) {
        self.earlier.clear();
        self.page = Some(page);
        self.later.clear();
        render::State::load_page(self);
    }

    pub fn page(&self) -> Option<&Page> {
        self.page.as_ref()
    }

    pub fn next_page(&mut self) -> Option<()> {
        self.earlier
            .extend(std::mem::replace(&mut self.page, Some(self.later.pop()?)));
        render::State::load_page(self);
        Some(())
    }

    pub fn prev_page(&mut self) -> Option<()> {
        self.later
            .extend(std::mem::replace(&mut self.page, Some(self.earlier.pop()?)));
        render::State::load_page(self);
        Some(())
    }

    pub fn pages_left(&self) -> usize {
        self.earlier.len()
    }

    pub fn pages_right(&self) -> usize {
        self.later.len()
    }

    /// Edit the current page, possibly splitting it into several in the process.
    pub fn edit_page(&mut self, fs: impl IntoIterator<Item = impl FnOnce(&mut Page)>) {
        if let Some(page) = self.page.take() {
            let len = self.earlier.len();
            self.earlier.extend(
                fs.into_iter()
                    .map(|f| {
                        let mut page = page.clone();
                        f(&mut page);
                        page
                    })
                    .filter(|page| !page.complete()),
            );
            if self.earlier.len() > len {
                self.page = self.earlier.pop();
            }
        }

        render::State::load_page(self);
    }

    pub fn scroll_background(&mut self, dx: f64, dy: f64) {
        self.render.scroll_background(dx, dy);
    }

    pub fn zoom_background(&mut self, x: f64, y: f64, scale_factor: f64) {
        self.render.zoom_background(x, y, scale_factor)
    }

    pub fn set_node_position(&mut self, node: Node, x: f64, y: f64) {
        if let Some(page) = &mut self.page {
            node.set_position(page, x, y);
            self.render.set_node_position(node, page);
        }
    }
}

#[derive(Clone)]
pub struct Page {
    local: LocalState,
    pub goal: Wire,
}

impl Page {
    pub fn new<E>(goal: impl FnOnce(&mut LocalState) -> Result<Wire, E>) -> Result<Self, E> {
        let mut local = LocalState::new();
        Ok(Self {
            goal: goal(&mut local)?,
            local,
        })
    }

    fn complete(&self) -> bool {
        self.local.wire_status(self.goal).is_some()
    }
}

impl Deref for Page {
    type Target = LocalState;

    fn deref(&self) -> &Self::Target {
        &self.local
    }
}

impl DerefMut for Page {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.local
    }
}
