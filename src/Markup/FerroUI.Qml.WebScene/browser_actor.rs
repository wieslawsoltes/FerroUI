//! Browser WorkerScript actor. The Wasm owner never blocks waiting for this
//! worker. Model receipts are sent only after the owner has processed the batch.
use super::*;
use ferroui_qml::workers::{
    WorkerDelivery, WorkerEndpoint, WorkerEvent, WorkerModelEdits, WorkerSyncRequest, WorkerValue,
};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/browser_actor.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = spawnActor)]
    fn spawn(
        program: &str,
        channel: &str,
        init: &str,
        timeout: f64,
        publish: &Closure<dyn Fn(String) -> JsValue>,
        stop: &Closure<dyn Fn(String)>,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = sendActor)]
    fn enqueue(actor: &JsValue, message: &str, size: usize) -> Result<(), JsValue>;
    #[wasm_bindgen(js_name = disposeActor)]
    fn dispose(actor: &JsValue);
}

struct Actor {
    worker: JsValue,
    pending: Rc<RefCell<Option<WorkerSyncRequest>>>,
    _publish: Closure<dyn Fn(String) -> JsValue>,
    _stop: Closure<dyn Fn(String)>,
}
impl WorkerEndpoint for Actor {
    fn send(&self, value: WorkerValue) -> Result<(), Error> {
        let size = value.estimated_bytes();
        if size > 32 * 1024 * 1024 {
            return Err(Error::new("WorkerScript input byte limit exceeded"));
        }
        let wire = serde_json::to_string(&value).map_err(|e| Error::new(e.to_string()))?;
        enqueue(&self.worker, &wire, size.max(wire.len())).map_err(error)
    }
}
impl Drop for Actor {
    fn drop(&mut self) {
        dispose(&self.worker);
        self.pending.borrow_mut().take();
    }
}
fn error(value: JsValue) -> Error {
    Error::new(format!("browser WorkerScript: {value:?}"))
}

pub(super) fn start(
    engine: &WebSceneEngine,
    resources: Arc<BTreeMap<String, ir::ScriptResource>>,
    origin: &str,
    source: &str,
    delivery: WorkerDelivery,
) -> Result<Box<dyn WorkerEndpoint>, Error> {
    let url = worker_program::entry_url(&resources, origin, source)?;
    let init = worker_program::initialization(&resources, &url)?;
    let pending = Rc::new(RefCell::new(None));
    let publish = {
        let sink = delivery.clone();
        let pending = pending.clone();
        Closure::new(move |wire: String| -> JsValue {
            let event: WorkerEvent = match serde_json::from_str(&wire) {
                Ok(e) => e,
                Err(e) => {
                    return json!({"stopped":format!("invalid WorkerScript event: {e}")})
                        .to_string()
                        .into()
                }
            };
            let (models, single) = match event {
                WorkerEvent::SyncTree(batch) => (batch, false),
                WorkerEvent::Sync {
                    id,
                    revision,
                    edits,
                } => (
                    vec![WorkerModelEdits {
                        id,
                        revision,
                        edits,
                    }],
                    true,
                ),
                event => {
                    return if sink.publish(event) {
                        r#"{"ok":true}"#
                    } else {
                        r#"{"stopped":"WorkerScript receiver unavailable"}"#
                    }
                    .into()
                }
            };
            let (send, receive) = async_channel::bounded(1);
            let request = sink.synchronize_tree_with(models, move |result| {
                let _ = send.try_send(result);
            });
            let request = match request {
                Ok(request) => request,
                Err(e) => return json!({"stopped":e.message}).to_string().into(),
            };
            pending.replace(Some(request));
            let pending = pending.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                let response = match receive.recv().await {
                    Ok(Ok(revisions)) if single => json!({"ok":revisions.first()}),
                    Ok(Ok(revisions)) => json!({"ok":revisions}),
                    Ok(Err(e)) => json!({"error":e.message}),
                    Err(_) => json!({"stopped":"WorkerScript synchronization cancelled"}),
                };
                pending.borrow_mut().take();
                Ok(JsValue::from_str(&response.to_string()))
            })
            .into()
        })
    };
    let stop = {
        let pending = pending.clone();
        Closure::new(move |reason: String| {
            pending.borrow_mut().take();
            delivery.publish(WorkerEvent::Stopped(reason));
        })
    };
    let channel = include_str!("browser_channel.js");
    let program = format!(
        "({guest})(({channel}), key => ({vm})(({modules})({loader}),key));",
        guest = include_str!("browser_actor_guest.js"),
        vm = include_str!("script_vm.js"),
        modules = include_str!("modules.js"),
        loader = include_str!("vendor/systemjs-core.js")
    );
    let worker = spawn(
        &program,
        channel,
        &init.to_string(),
        engine.timeout.as_secs_f64() * 1000.,
        &publish,
        &stop,
    )
    .map_err(error)?;
    Ok(Box::new(Actor {
        worker,
        pending,
        _publish: publish,
        _stop: stop,
    }))
}
