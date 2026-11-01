//! Browser transport for the shared Realm command and reentrant host protocol.
use super::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/browser_process.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = requireBrowserWorker)]
    fn require_worker() -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = spawnBrowserWorker)]
    async fn spawn(program: &str, channel: &str, timeout: f64) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = sendFrame)]
    fn send_frame(
        worker: &JsValue,
        text: &str,
        begin: bool,
        milliseconds: f64,
    ) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = readFrame)]
    fn read_frame(worker: &JsValue, milliseconds: f64) -> Result<String, JsValue>;
    #[wasm_bindgen(js_name = disposeBrowserWorker)]
    fn dispose(worker: &JsValue);
}
pub(super) struct Worker(JsValue);
fn error(value: JsValue) -> Error {
    Error::new(format!("browser QML transport: {value:?}"))
}
pub(super) fn available() -> Result<(), Error> {
    require_worker().map_err(error)
}

impl Worker {
    pub(super) async fn spawn(timeout: Duration) -> Result<Self, Error> {
        let channel = include_str!("browser_channel.js");
        let program = format!("({guest})(({channel}), key => ({bridge})(({modules})({loader}), ({promise}), key, ({leases}), ({turns})));",
            guest=include_str!("browser_guest.js"),bridge=include_str!("bridge.js"),modules=include_str!("modules.js"),
            loader=include_str!("vendor/systemjs-core.js"),promise=include_str!("promise.js"),leases=include_str!("leases.js"),turns=include_str!("turns.js"));
        spawn(&program, channel, timeout.as_secs_f64() * 1000.0)
            .await
            .map(Self)
            .map_err(error)
    }
    pub(super) fn send(&self, msg: Json, deadline: Instant) -> Result<(), Error> {
        let begin = msg.get("op").is_some();
        let text = serde_json::to_string(&msg).map_err(|e| Error::new(e.to_string()))?;
        send_frame(&self.0, &text, begin, remaining(deadline)).map_err(error)
    }
    pub(super) fn read(&self, deadline: Instant) -> Result<Json, Error> {
        let text = read_frame(&self.0, remaining(deadline)).map_err(error)?;
        serde_json::from_str(&text)
            .map_err(|e| Error::new(format!("invalid browser QML response: {e}")))
    }
}
fn remaining(deadline: Instant) -> f64 {
    deadline
        .saturating_duration_since(Instant::now())
        .as_secs_f64()
        * 1000.0
}
impl Drop for Worker {
    fn drop(&mut self) {
        dispose(&self.0);
    }
}
