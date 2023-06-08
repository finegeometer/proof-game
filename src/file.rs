use wasm_bindgen::prelude::*;

#[allow(clippy::needless_lifetimes)]
pub(crate) fn file_listener<'a>(
    bump: &'a dodrio::bumpalo::Bump,
    msg: impl 'static + Clone + FnOnce(String) -> crate::Msg,
    fail: impl 'static + Clone + FnOnce() -> crate::Msg,
) -> dodrio::Listener<'a> {
    dodrio::builder::on(bump, "change", move |root, _, event| {
        let msg = msg.clone();
        let fail = fail.clone();

        let send_msg = root.unwrap_mut::<super::Model>().send_msg.clone();

        // Immediately Invoked Function Expression
        let Some(promise) = move || -> Option<js_sys::Promise> {
            let files = event
                .current_target()?
                .dyn_into::<web_sys::HtmlInputElement>().ok()?
                .files()?;
            if files.length() != 1 {
                return None;
            }

            Some(files.get(0)?.dyn_into::<web_sys::File>().ok()?.text())
        }() else {
            send_msg.send_blocking(fail()).unwrap();
            return;
        };

        wasm_bindgen_futures::spawn_local(async move {
            send_msg
                .send(
                    wasm_bindgen_futures::JsFuture::from(promise)
                        .await
                        .ok()
                        .and_then(|value| value.as_string())
                        .map(msg)
                        .unwrap_or_else(fail),
                )
                .await
                .unwrap();
        });
    })
}
