//! A browser fetch transport for QML applications on the window or a worker.
use super::{ComponentReply, ComponentTransport, ResourceError, Url, MAX_DOCUMENT_BYTES};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/network_browser.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = fetchComponent)]
    async fn fetch_component(url: &str, limit: usize) -> Result<JsValue, JsValue>;
}

/// Fetch component resources from the application's own origin. HTTP redirects
/// are rejected and cancelled factories cannot deliver into a new graph.
#[derive(Default)]
pub struct BrowserComponentTransport {
    active: Rc<Cell<usize>>,
}
impl ComponentTransport for BrowserComponentTransport {
    fn request(&self, url: &Url, reply: ComponentReply) {
        if self.active.get() >= 16 {
            reply.complete(Err(ResourceError::Failed(
                "browser component transport is busy".into(),
            )));
            return;
        }
        let active = self.active.clone();
        active.set(active.get() + 1);
        let url = url.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let response = if reply.is_pending() {
                match fetch_component(&url, MAX_DOCUMENT_BYTES).await {
                    Ok(v) if v.is_undefined() => Err(ResourceError::NotFound),
                    Ok(v) => v
                        .dyn_into::<js_sys::Uint8Array>()
                        .map(|bytes| bytes.to_vec())
                        .map_err(|_| {
                            ResourceError::Failed("invalid browser transport response".into())
                        }),
                    Err(reason) => Err(ResourceError::Failed(
                        reason
                            .as_string()
                            .unwrap_or_else(|| "browser fetch failed".into()),
                    )),
                }
            } else {
                Err(ResourceError::Failed(
                    "component request was cancelled".into(),
                ))
            };
            active.set(active.get() - 1);
            reply.complete(response);
        });
    }
}
