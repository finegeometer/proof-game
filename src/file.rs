#![allow(clippy::needless_lifetimes)]

use wasm_bindgen::prelude::*;

pub(crate) fn save_listener<'a>(
    bump: &'a dodrio::bumpalo::Bump,
    save: impl 'static + Fn(&mut crate::Model) -> String,
    filename: &'static str,
) -> dodrio::Listener<'a> {
    dodrio::builder::on(bump, "click", move |root, _, _| {
        let a: web_sys::HtmlAnchorElement = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("a")
            .unwrap()
            .dyn_into()
            .unwrap();

        let data = save(root.unwrap_mut());
        let blob = web_sys::Blob::new_with_str_sequence(&js_sys::Array::from_iter(
            std::iter::once(JsValue::from_str(&data)),
        ))
        .unwrap();

        a.set_href(&web_sys::Url::create_object_url_with_blob(&blob).unwrap());
        a.set_download(filename);
        a.click();
    })
}

pub(crate) fn load_listener<'a>(
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

pub(crate) fn fetch_listener<'a>(
    bump: &'a dodrio::bumpalo::Bump,
    path: &'static str,
    msg: impl 'static + Clone + FnOnce(String) -> crate::Msg,
    fail: impl 'static + Clone + FnOnce() -> crate::Msg,
) -> dodrio::Listener<'a> {
    dodrio::builder::on(bump, "click", move |root, _, event| {
        let msg = msg.clone();
        let fail = fail.clone();

        let send_msg = root.unwrap_mut::<super::Model>().send_msg.clone();

        #[rustfmt::skip]
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(response) = wasm_bindgen_futures::JsFuture::from(web_sys::window().unwrap().fetch_with_str(path)).await
            else {return send_msg.send(fail()).await.unwrap()};
            let Ok(response) = response.dyn_into::<web_sys::Response>()
            else {return send_msg.send(fail()).await.unwrap()};
            let Ok(promise) = response.text()
            else {return send_msg.send(fail()).await.unwrap()};
            let Ok(text) = wasm_bindgen_futures::JsFuture::from(promise).await
            else {return send_msg.send(fail()).await.unwrap()};

            send_msg.send(msg(text.as_string().unwrap())).await.unwrap();
        });
    })
}
