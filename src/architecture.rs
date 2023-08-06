//! The Elm Architecture, roughly.

use async_channel::Sender;
use dodrio::{bumpalo, Listener, Node, Render, RenderContext, VdomWeak};

pub(crate) trait Architecture: 'static {
    type Msg;

    fn new() -> Self;

    fn update(&mut self, msg: Self::Msg, rerender: &mut bool);

    fn view<'a>(&self, cx: &mut RenderContext<'a>) -> Node<'a>;

    fn listener<'a>(
        bump: &'a bumpalo::Bump,
        event: &'a str,
        msg: impl 'static + Fn(web_sys::Event) -> Self::Msg,
    ) -> Listener<'a>
    where
        Self: Sized,
    {
        Self::listener_raw(bump, event, move |e, _, _, send| {
            send.send_blocking(msg(e))
                .expect("Program is no longer receiving messages. This should not happen.")
        })
    }

    fn listener_raw<'a>(
        bump: &'a bumpalo::Bump,
        event: &'a str,
        handler: impl 'static + Fn(web_sys::Event, &mut Self, VdomWeak, &Sender<Self::Msg>),
    ) -> Listener<'a>
    where
        Self: Sized,
    {
        dodrio::builder::on(bump, event, move |root, vdom, event| {
            event.stop_propagation();
            event.prevent_default();
            let State { model, sender } = &mut root.unwrap_mut::<State<Self>>();
            handler(event, model, vdom, sender);
        })
    }
}

pub(crate) fn main<Model: Architecture>(
    setup: impl FnOnce(&web_sys::Document, &Sender<Model::Msg>),
) {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let (sender, receiver) = async_channel::unbounded();
    let document = web_sys::window()
        .expect("`window` not found.")
        .document()
        .expect("`window.document` not found.");

    setup(&document, &sender);

    let vdom = dodrio::Vdom::new(
        &document.get_element_by_id("vdom").unwrap(),
        State {
            model: Model::new(),
            sender,
        },
    );

    wasm_bindgen_futures::spawn_local(async move {
        let vdom = vdom.weak();
        while let Ok(msg) = receiver.recv().await {
            let rerender = vdom
                .with_component(|root| {
                    let mut rerender = false;
                    root.unwrap_mut::<State<Model>>()
                        .model
                        .update(msg, &mut rerender);
                    rerender
                })
                .await
                .expect("Vdom should not be dropped, as it is owned by the containing future.");
            if rerender {
                vdom.schedule_render();
            }
        }
    });
}

struct State<Model: Architecture> {
    model: Model,
    sender: Sender<Model::Msg>,
}

impl<'a, Model: Architecture> Render<'a> for State<Model> {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        self.model.view(cx)
    }
}
