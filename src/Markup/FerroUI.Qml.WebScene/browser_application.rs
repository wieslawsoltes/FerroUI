//! A compiled application's dedicated worker owns this instance and all host
//! references. Only copied values and explicitly watched properties leave it.
use super::*;
use ferroui_qml::workers::WorkerValue;
use std::{collections::BTreeSet, sync::Arc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/browser_application.js")]
extern "C" {
    #[wasm_bindgen(js_name = applicationModule)]
    fn module_url() -> String;
    #[wasm_bindgen(js_name = wakeApplication)]
    fn wake();
}

/// URL of the packaged ES module containing openQmlApplication and serveQmlApplication.
#[wasm_bindgen(js_name = qmlApplicationModule)]
pub fn browser_application_module() -> String {
    module_url()
}

#[wasm_bindgen]
pub struct BrowserApplication {
    model: ferroui_qml::Instance,
    watches: BTreeMap<(ObjectId, String), Watch>,
    revision: u64,
    last: Instant,
    timeout: Duration,
}

struct Watch {
    tokens: BTreeSet<u32>,
    snapshot: Option<String>,
}

impl BrowserApplication {
    /// Wrap an initialized instance inside its dedicated application worker.
    pub fn new(mut model: ferroui_qml::Instance, timeout: Duration) -> Result<Self, Error> {
        process::available()?;
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(|| wake());
        model.set_worker_waker(Some(wake.clone()));
        model.set_component_waker(Some(wake));
        Ok(Self {
            revision: model.models_revision(),
            model,
            watches: Default::default(),
            last: Instant::now(),
            timeout,
        })
    }
    fn object(&self, id: &str) -> Result<ObjectId, Error> {
        if id.is_empty() {
            return Ok(self.model.root());
        }
        self.model
            .id(id)
            .ok_or_else(|| Error::new(format!("unknown QML id '{id}'")))
    }
    fn request(
        &mut self,
        op: &str,
        id: &str,
        member: &str,
        wire: &str,
    ) -> Result<WorkerValue, Error> {
        let owner = self.object(id)?;
        let value = match op {
            "get" => self.model.get(owner, member)?,
            "set" => {
                let data = serde_json::from_str(wire).map_err(|e| Error::new(e.to_string()))?;
                self.model.set_data(owner, member, data)?;
                return Ok(WorkerValue::Undefined);
            }
            "call" => {
                let items: Vec<WorkerValue> =
                    serde_json::from_str(wire).map_err(|e| Error::new(e.to_string()))?;
                return self.model.call_data(owner, member, items, self.timeout);
            }
            _ => return Err(Error::new("unknown QML request")),
        };
        self.model.copy_data(owner, value)
    }
}

fn watched_data(
    model: &mut ferroui_qml::Instance,
    owner: ObjectId,
    member: &str,
) -> Result<(WorkerValue, Option<String>), Error> {
    let value = model.get(owner, member)?;
    let is_model = matches!(value, Value::Object(id) if model.is_list_model(id));
    let data = model.copy_data(owner, value)?;
    let snapshot = is_model.then(|| serde_json::to_string(&data).expect("model snapshot"));
    Ok((data, snapshot))
}

fn error(e: Error) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
impl BrowserApplication {
    pub fn dispatch(
        &mut self,
        op: &str,
        id: &str,
        member: &str,
        data: &str,
    ) -> Result<String, JsValue> {
        let value = self.request(op, id, member, data).map_err(error)?;
        serde_json::to_string(&value).map_err(|e| JsValue::from_str(&e.to_string()))
    }
    pub fn watch(&mut self, token: u32, id: &str, member: &str) -> Result<(), JsValue> {
        let owner = self.object(id).map_err(error)?;
        let key = (owner, member.to_owned());
        if let Some(watch) = self.watches.get_mut(&key) {
            watch.tokens.insert(token);
            return Ok(());
        }
        let (_, snapshot) = watched_data(&mut self.model, owner, member).map_err(error)?;
        self.watches.insert(
            key,
            Watch {
                tokens: [token].into(),
                snapshot,
            },
        );
        Ok(())
    }
    pub fn unwatch(&mut self, token: u32) {
        self.watches.retain(|_, watch| {
            watch.tokens.remove(&token);
            !watch.tokens.is_empty()
        });
    }
    pub fn changes(&mut self) -> String {
        let changes = self
            .model
            .drain_changes()
            .into_iter()
            .map(|c| (c.object, c.property))
            .collect::<BTreeSet<_>>();
        let revision = self.model.models_revision();
        let mut output = Vec::new();
        for (key, watch) in &mut self.watches {
            let (owner, member) = key;
            let changed = changes.contains(key);
            if !changed && !(revision != self.revision && watch.snapshot.is_some()) {
                continue;
            }
            let update = match watched_data(&mut self.model, *owner, member) {
                Ok((data, snapshot)) => {
                    if !changed && snapshot == watch.snapshot {
                        continue;
                    }
                    watch.snapshot = snapshot;
                    json!({"value":data})
                }
                Err(e) => {
                    watch.snapshot = None;
                    json!({"error":e.message})
                }
            };
            output.push(json!({"tokens":watch.tokens,"update":update}));
        }
        self.revision = revision;
        serde_json::to_string(&output).expect("data packet")
    }
    pub fn pump(&mut self) -> Result<(), JsValue> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        self.model.advance_time(elapsed).map_err(error)?;
        self.model.poll_worker_scripts(16).map_err(error)?;
        self.model.poll_component_loads(8).map_err(error)?;
        self.model.process_deferred(16).map_err(error)?;
        Ok(())
    }
    #[wasm_bindgen(js_name = nextWake)]
    pub fn next_wake(&self) -> f64 {
        if self.model.has_deferred_work() {
            0.
        } else {
            self.model
                .next_timer_deadline()
                .map_or(-1., |d| d.as_secs_f64() * 1000.)
        }
    }
}
